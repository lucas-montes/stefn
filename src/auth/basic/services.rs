use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{Path, Query, State},
    http::{header::SET_COOKIE, HeaderValue},
    response::Redirect,
    Extension,
};
use cookie::{time::Duration, SameSite};
use hyper::HeaderMap;
use serde::Deserialize;
use sqlx::PgConnection;
use validator::Validate;

use crate::{
    config::{ServiceConfig, WebsiteConfig},
    database::Database,
    errors::AppError,
    log_and_wrap_custom_internal,
    models::{EmailAccount, Group, User, UserWithPassword},
    sessions::{Session, Sessions},
    state::WebsiteState,
    website::SecureForm,
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

pub trait Ingress {
    fn route(
        state: State<WebsiteState>,
        Extension(session): Extension<Session>,
        params: Query<IngressParams>,
        input: SecureForm<IngressForm>,
    ) -> impl std::future::Future<Output = Result<Redirect, AppError>> + Send {
        async move {
            let input = input.data();
            match input.process {
                IngressProcess::Login => Self::login(&state, session, &params, input).await,
                IngressProcess::Register => Self::register(&state, session, &params, input).await,
            }
            .map(Redirect::to)
        }
    }

    fn login<'a>(
        state: &'a WebsiteState,
        session: Session,
        params: &'a IngressParams,
        input: IngressForm,
    ) -> impl std::future::Future<Output = Result<&'a str, AppError>> + Send {
        async move {
            let config = state.config();

            let user = Self::validate_login(state, &input).await?;

            Self::handle_login_session(state.sessions(), session, config, user).await?;

            Ok(Self::get_login_redirect(config, params))
        }
    }

    fn validate_login<'a>(
        state: &'a WebsiteState,
        input: &'a IngressForm,
    ) -> impl std::future::Future<Output = Result<UserWithPassword, AppError>> + Send {
        async {
            User::find_by_email_with_password(&input.email, state.database())
                .await?
                .ok_or(AppError::DoesNotExist)
                .and_then(|u| verify_password(&input.password, &u.password).map(|_| u))
        }
    }

    fn handle_login_session<'a>(
        sessions: &'a Sessions,
        session: Session,
        config: &'a WebsiteConfig,
        user: UserWithPassword,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send {
        async move {
            sessions
                .reuse_current_as_new_one(&session, user.user.for_session(), &config.session_key)
                .await
        }
    }

    fn get_login_redirect<'a>(config: &'a WebsiteConfig, params: &'a IngressParams) -> &'a str {
        params.next.as_ref().unwrap_or(&config.login_redirect_to)
    }

    fn register<'a>(
        state: &'a WebsiteState,
        session: Session,
        params: &'a IngressParams,
        input: IngressForm,
    ) -> impl std::future::Future<Output = Result<&'a str, AppError>> + Send {
        async {
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
    }

    fn create_user<'a>(
        database: &'a Database,
        input: IngressForm,
        config: &'a WebsiteConfig,
    ) -> impl std::future::Future<Output = Result<EmailAccount, AppError>> + Send {
        async move {
            let mut tx = database.start_transaction().await?;

            let password = hash_password(&input.password)?;

            let activated_at = if config.email_validation {
                None
            } else {
                Some(chrono::Utc::now().naive_utc())
            };

            let user = User::create(&password, activated_at, &mut *tx)
                .await?
                .add_to_group(Group::User, &mut *tx)
                .await?;

            let email_account =
                EmailAccount::create_primary(user, input.email, activated_at, &mut *tx).await?;

            tx.commit().await?;
            Ok(email_account)
        }
    }

    fn handle_email_validation<'a>(
        state: &'a WebsiteState,
        user: &'a EmailAccount,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send {
        async {
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
    }

    fn handle_register_session<'a>(
        sessions: &'a Sessions,
        session: Session,
        config: &'a WebsiteConfig,
        user: EmailAccount,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send {
        async move {
            sessions
                .reuse_current_as_new_one(&session, user.user.for_session(), &config.session_key)
                .await
        }
    }

    fn get_register_redirect<'a>(config: &'a WebsiteConfig, _params: &'a IngressParams) -> &'a str {
        if config.email_validation {
            &config.email_validation_redirect
        } else {
            &config.login_redirect_to
        }
    }
}

pub trait EmailValidation {
    fn route(
        state: State<WebsiteState>,
        Extension(session): Extension<Session>,
        Path(slug): Path<String>,
    ) -> impl std::future::Future<Output = Result<Redirect, AppError>> + Send {
        async move {
            Self::validate_email(&state, session, slug)
                .await
                .map(|r| Redirect::to(r.as_str()))
        }
    }

    fn validate_email(
        state: &WebsiteState,
        session: Session,
        slug: String,
    ) -> impl std::future::Future<Output = Result<&String, AppError>> + Send {
        async {
            let database = state.database();
            let config = state.config();
            let mut tx = database.start_transaction().await?;
            let validation = EmailValidationManager::delete_and_get_email_pk(slug, &mut tx).await?;
            let user = Self::activate_user(validation, &mut tx).await?;
            tx.commit()
                .await
                .map_err(|e| log_and_wrap_custom_internal!(e))?;

            Self::handle_session(state.sessions(), session, config, user).await?;
            Ok(&config.login_redirect_to)
        }
    }

    fn activate_user(
        validation: EmailValidationManager,
        tx: &mut PgConnection,
    ) -> impl std::future::Future<Output = Result<EmailAccount, AppError>> + Send {
        async move {
            let email = EmailAccount::get_by_pk(validation.email_pk, &mut *tx)
                .await?
                .set_to_active(&mut *tx)
                .await?;
            email.user.set_to_active(&mut *tx).await?;
            Ok(email)
        }
    }

    fn handle_session(
        sessions: &Sessions,
        session: Session,
        config: &WebsiteConfig,
        user: EmailAccount,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send {
        async move {
            sessions
                .reuse_current_as_new_one(&session, user.user.for_session(), &config.session_key)
                .await
        }
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

    // Security Headers:
    // Enforce HTTPS using HSTS.
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Prevent MIME type sniffing.
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // Set a basic Content-Security-Policy.
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("frame-ancestors 'none'"),
    );

    // Existing header for framing.
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    Ok(())
}
