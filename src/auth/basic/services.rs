use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::http::{header::SET_COOKIE, HeaderValue};
use cookie::{time::Duration, SameSite};
use hyper::HeaderMap;

use crate::{
    config::{ServiceConfig, WebsiteConfig},
    sessions::Session,
    AppError,
};

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
