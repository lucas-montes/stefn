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
    service::AppError,
    sessions::Session,
    state::WebsiteState,
};

use super::find_user_by_email;

#[derive(Deserialize)]
pub enum IngressProcess {
    Login,
    Register,
}

#[derive(Deserialize)]
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
    let redirect = params
        .next
        .as_ref()
        .unwrap_or(&state.config().login_redirect_to);

    let user = find_user_by_email(&state.database(), &input.email)
        .await?
        .ok_or(AppError::DoesNotExist)?;
    verify_password(&input.password, &user.password)?;

    let sessions = state.sessions();

    sessions
        .reuse_current_as_new_one(session, user.pk, user.groups)
        .await?;

    Ok(redirect)
}
pub async fn register<'a>(
    state: &'a WebsiteState,
    session: Session,
    params: &'a IngressParams,
    input: &'a IngressForm,
) -> Result<&'a str, AppError> {
    let redirect = params
        .next
        .as_ref()
        .unwrap_or(&state.config().login_redirect_to);

    let user = find_user_by_email(&state.database(), &input.email)
        .await?
        .ok_or(AppError::DoesNotExist)?;
    verify_password(&input.password, &user.password)?;

    state
        .sessions()
        .reuse_current_as_new_one(session, user.pk, user.groups)
        .await?;

    Ok(redirect)
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
