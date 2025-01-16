use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    async_trait,
    extract::{Path, Query, State},
    http::{header::SET_COOKIE, HeaderValue},
    response::Redirect,
    Extension, Form,
};
use cookie::{time::Duration, SameSite};
use hyper::HeaderMap;
use serde::Deserialize;
use sqlx::SqliteConnection;
use validator::Validate;

use crate::{
    config::{ServiceConfig, WebsiteConfig},
    database::Database,
    log_and_wrap_custom_internal,
    models::{EmailAccount, Group, User, UserWithPassword},
    service::AppError,
    sessions::{Session, Sessions},
    state::WebsiteState, website::SecureForm,
};

use super::infrastructure::EmailValidationManager;

#[derive(Debug, Deserialize)]
pub enum IngressProcess {
    Login,
    Register,
}

#[derive(Debug, Validate, Deserialize)]
pub struct IngressForm {
    #[validate(email)]
    email: String,
    password: String,
    process: IngressProcess,
}

#[derive(Debug, Deserialize)]
pub struct IngressParams {
    next: Option<String>,
}

#[async_trait]
pub trait Ingress {
    async fn route(
        state: State<WebsiteState>,
        Extension(session): Extension<Session>,
        params: Query<IngressParams>,
        input: SecureForm<IngressForm>,
    ) -> Result<Redirect, AppError> {
        let input = input.data();
        match input.process {
            IngressProcess::Login => Self::login(&state, session, &params, input).await,
            IngressProcess::Register => Self::register(&state, session, &params, input).await,
        }
        .map(Redirect::to)
    }

    async fn login<'a>(
        state: &'a WebsiteState,
        session: Session,
        params: &'a IngressParams,
        input: IngressForm,
    ) -> Result<&'a str, AppError> {
        let config = state.config();

        let user = Self::validate_login(state, &input).await?;

        Self::handle_login_session(state.sessions(), session, config, user).await?;

        Ok(Self::get_login_redirect(config, params))
    }

    async fn validate_login<'a>(
        state: &'a WebsiteState,
        input: &'a IngressForm,
    ) -> Result<UserWithPassword, AppError> {
        User::find_by_email_with_password(state.database(), &input.email)
            .await?
            .ok_or(AppError::DoesNotExist)
            .and_then(|u| verify_password(&input.password, &u.password).map(|_| u))
    }

    async fn handle_login_session<'a>(
        sessions: &'a Sessions,
        session: Session,
        config: &'a WebsiteConfig,
        user: UserWithPassword,
    ) -> Result<(), AppError> {
        sessions
            .reuse_current_as_new_one(&session, user.user.for_session(), &config.session_key)
            .await
    }

    fn get_login_redirect<'a>(config: &'a WebsiteConfig, params: &'a IngressParams) -> &'a str {
        params.next.as_ref().unwrap_or(&config.login_redirect_to)
    }

    async fn register<'a>(
        state: &'a WebsiteState,
        session: Session,
        params: &'a IngressParams,
        input: IngressForm,
    ) -> Result<&'a str, AppError> {
        let config = state.config();
        let database = state.database();

        let user = Self::create_user(database, input, config).await?;

        if config.email_validation {
            Self::handle_email_validation(state, &user).await?;
        } else {
            Self::handle_register_session(state.sessions(), session, config, user).await?;
        };

        Ok(Self::get_register_redirect(config, params))
    }

    async fn create_user<'a>(
        database: &'a Database,
        input: IngressForm,
        config: &'a WebsiteConfig,
    ) -> Result<EmailAccount, AppError> {
        let mut tx = database.start_transaction().await?;

        let password = hash_password(&input.password)?;

        let activated_at = if config.email_validation {
            None
        } else {
            Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs() as i64,
            )
        };

        let user = User::create(&mut tx, &password, activated_at)
            .await?
            .add_to_group(Group::User, &mut tx)
            .await?;

        let email_account =
            EmailAccount::create_primary(&mut tx, user, input.email, activated_at).await?;

        tx.commit().await?;
        Ok(email_account)
    }

    async fn handle_email_validation<'a>(
        state: &'a WebsiteState,
        user: &'a EmailAccount,
    ) -> Result<(), AppError> {
        let config = state.config();
        let database = state.database();
        let mailer = state.mailer();
        EmailValidationManager::new(user.pk)
            .save(database)
            .await?
            .send(config, mailer, &user.email)
            .await?;
        Ok(())
    }

    async fn handle_register_session<'a>(
        sessions: &'a Sessions,
        session: Session,
        config: &'a WebsiteConfig,
        user: EmailAccount,
    ) -> Result<(), AppError> {
        sessions
            .reuse_current_as_new_one(&session, user.user.for_session(), &config.session_key)
            .await
    }

    fn get_register_redirect<'a>(config: &'a WebsiteConfig, _params: &'a IngressParams) -> &'a str {
        if config.email_validation {
            &config.email_validation_redirect
        } else {
            &config.login_redirect_to
        }
    }
}

#[async_trait]
pub trait EmailValidation {
    async fn route(
        state: State<WebsiteState>,
        Extension(session): Extension<Session>,
        Path(slug): Path<String>,
    ) -> Result<Redirect, AppError> {
        Self::validate_email(&state, session, slug)
            .await
            .map(Redirect::to)
    }

    async fn validate_email<'a>(
        state: &'a WebsiteState,
        session: Session,
        slug: String,
    ) -> Result<&'a str, AppError> {
        let database = state.database();
        let config = state.config();
        let mut tx = database.start_transaction().await?;
        let validation = EmailValidationManager::delete_and_get_email_pk(&mut tx, slug).await?;
        let user = Self::activate_user(&mut tx, validation).await?;
        tx.commit()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        Self::handle_session(state.sessions(), session, config, user).await?;
        Ok(&config.login_redirect_to)
    }

    async fn activate_user<'a>(
        tx: &mut SqliteConnection,
        validation: EmailValidationManager,
    ) -> Result<EmailAccount, AppError> {
        let email = EmailAccount::get_by_pk(tx, validation.email_pk)
            .await?
            .set_to_active(tx)
            .await?;
        email.user.set_to_active(tx).await?;
        Ok(email)
    }

    async fn handle_session<'a>(
        sessions: &'a Sessions,
        session: Session,
        config: &'a WebsiteConfig,
        user: EmailAccount,
    ) -> Result<(), AppError> {
        sessions
            .reuse_current_as_new_one(&session, user.user.for_session(), &config.session_key)
            .await
    }
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(AppError::ErrorHashingPassword)?
        .to_string())
}

pub fn verify_password(raw_password: &str, db_password: &str) -> Result<(), AppError> {
    let parsed_hash = PasswordHash::new(db_password).map_err(AppError::ErrorHashingPassword)?;
    Argon2::default()
        .verify_password(raw_password.as_bytes(), &parsed_hash)
        .map_err(AppError::WrongPassword)
}

pub async fn set_session_cookies(
    headers: &mut HeaderMap<HeaderValue>,
    session: &Session,
    config: &WebsiteConfig,
) -> Result<(), AppError> {
    let cookie = cookie::Cookie::build((&config.csrf_cookie_name, session.csrf_token().await))
        .domain(config.domain())
        .path("/")
        .max_age(Duration::days(config.session_expiration))
        .secure(true)
        .http_only(false)
        .same_site(SameSite::Lax)
        .build();

    headers.append(
        SET_COOKIE,
        HeaderValue::from_bytes(cookie.encoded().to_string().as_bytes())
            .map_err(|e| log_and_wrap_custom_internal!(e))?,
    );

    let cookie = cookie::Cookie::build((&config.session_cookie_name, session.id().await))
        .domain(config.domain())
        .path("/")
        .max_age(Duration::days(config.session_expiration))
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    headers.append(
        SET_COOKIE,
        HeaderValue::from_bytes(cookie.encoded().to_string().as_bytes())
            .map_err(|e| log_and_wrap_custom_internal!(e))?,
    );
    //TODO: see how and where to put those
    // headers.insert(
    //     "Content-Security-Policy",
    //     "default-src 'self'".parse().unwrap(),
    // );
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    Ok(())
}
