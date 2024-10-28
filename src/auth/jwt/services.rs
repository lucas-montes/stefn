use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, get_current_timestamp, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

use crate::service::{AppError, AppState};

use super::dtos::{JWTClaims, JWTUserRequest};

pub fn create_token<
    T: for<'a> Deserialize<'a> + std::marker::Send + std::marker::Sync + Clone + 'static + Serialize,
>(
    user_id: i64,
    private_claims: T,
    domain: &str,
    encoding: &EncodingKey,
) -> Result<String, AppError> {
    encode(
        &Header::default(),
        &JWTClaims::new(user_id, domain, private_claims),
        encoding,
    )
    .map_err(AppError::JWTError)
}

pub fn get_user_from_valid_token<
    T: for<'a> Deserialize<'a> + std::marker::Send + std::marker::Sync + Clone + 'static,
>(
    domain: &str,
    decoding: &DecodingKey,
    token: &str,
) -> Result<JWTUserRequest<T>, AppError> {
    let mut validation = Validation::default();
    validation.set_audience(&[domain]);
    validation.set_issuer(&[domain]);
    validation.leeway = 60 * 60 * 60 * 24 * 30; //TODO: keep at one hour instead of days
    validation.reject_tokens_expiring_in_less_than = 86400u64;
    validation.set_required_spec_claims(&["iss", "sub", "aud", "exp", "iat", "jti", "private"]);

    let token = decode::<JWTClaims<T>>(token, decoding, &validation).map_err(AppError::JWTError)?;

    Ok(JWTUserRequest::new(token.claims)?)
}
