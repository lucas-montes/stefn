use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::http::{header::SET_COOKIE, HeaderValue};
use cookie::{time::Duration, SameSite};
use hyper::HeaderMap;
use serde::Deserialize;

use crate::{
    config::{ServiceConfig, WebsiteConfig},
    models::{EmailAccount, Group, User},
    service::AppError,
    sessions::Session,
    state::WebsiteState,
};

use super::email_validation::EmailValidation;

#[derive(Debug, Deserialize)]
pub enum IngressProcess {
    Login,
    Register,
}

#[derive(Debug, Deserialize)]
pub struct IngressForm {
    email: String,
    password: String,
    csrf_token: String,
    process: IngressProcess,
}

#[derive(Debug, Deserialize)]
pub struct IngressParams {
    next: Option<String>,
}

pub async fn handle_ingress<'a>(
    state: &'a WebsiteState,
    session: Session,
    params: &'a IngressParams,
    input: &'a IngressForm,
) -> Result<&'a str, AppError> {
    let config = state.config();
    session
        .validate_csrf_token(&config.session_key, &input.csrf_token)
        .await?;
    match input.process {
        IngressProcess::Login => login(&state, session, &params, &input).await,
        IngressProcess::Register => register(&state, session, &params, &input).await,
    }
}

pub async fn login<'a>(
    state: &'a WebsiteState,
    session: Session,
    params: &'a IngressParams,
    input: &'a IngressForm,
) -> Result<&'a str, AppError> {
    let config = state.config();
    let redirect = params.next.as_ref().unwrap_or(&config.login_redirect_to);

    let user = User::find_by_email_with_password(&state.database(), &input.email)
        .await?
        .ok_or(AppError::DoesNotExist)?;
    //TODO: fix
    verify_password(&input.password, "password")?;

    let sessions = state.sessions();

    sessions
        .reuse_current_as_new_one(session, user.for_session(), &config.session_key)
        .await?;

    Ok(redirect)
}
pub async fn register<'a>(
    state: &'a WebsiteState,
    session: Session,
    params: &'a IngressParams,
    input: &'a IngressForm,
) -> Result<&'a str, AppError> {
    let database = state.database();
    let config = state.config();

    let mut tx = database.start_transaction().await?;

    let password = hash_password(&input.password)?;

    let user = User::create(&mut tx, &password)
        .await?
        .add_to_group(Group::User, &mut tx)
        .await?;

    user.add_profile(&mut tx, "", "", "", "").await?;
    let email_account = EmailAccount::create_primary(&mut tx, user, &input.email).await?;

    tx.commit()
        .await
        .map_err(|e| AppError::custom_internal(&e.to_string()))?;

    let redirect = if config.email_validation {
        //TODO: add the next param
        EmailValidation::new(email_account.pk)
            .save(database)
            .await?
            .send()
            .await?;
        &config.email_validation_redirect
    } else {
        state
            .sessions()
            .reuse_current_as_new_one(
                session,
                email_account.user.for_session(),
                &config.session_key,
            )
            .await?;
        &config.login_redirect_to
    };

    Ok(redirect)
}

pub async fn handle_validate_email<'a>(
    state: &'a WebsiteState,
    session: Session,
    slug: String,
) -> Result<&'a str, AppError> {
    let database = state.database();
    let config = state.config();
    let mut tx = database.start_transaction().await?;
    let validation = EmailValidation::delete_and_get_email_pk(&mut tx, slug).await?;
    let email = EmailAccount::get_by_pk(&mut tx, validation.email_pk)
        .await?
        .set_to_active(&mut tx)
        .await?;
    email.user.set_to_active(&mut tx).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::custom_internal(&e.to_string()))?;

    state
        .sessions()
        .reuse_current_as_new_one(session, email.user.for_session(), &config.session_key)
        .await?;
    Ok(&config.login_redirect_to)
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
            .map_err(|e| AppError::custom_internal(&e.to_string()))?,
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
            .map_err(|e| AppError::custom_internal(&e.to_string()))?,
    );
    Ok(())
}
