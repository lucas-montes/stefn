use chrono::{Duration, Utc};
use jsonwebtoken::{get_current_timestamp, DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;

#[derive(Clone)]
pub struct JWTUserRequest<T: Clone> {
    pub id: i64,
    pub private: T,
}

impl<T: Clone> JWTUserRequest<T> {
    pub fn new(claims: JWTClaims<T>) -> Result<Self, AppError> {
        Ok(Self {
            id: claims.sub.parse::<i64>().map_err(AppError::JWTModified)?,
            private: claims.private.clone(),
        })
    }
}

#[derive(Clone)]
pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JWTClaims<T> {
    // Registered claims
    iss: String, // Issuer
    sub: String, // Subject
    aud: String, // Audience
    exp: i64,    // Expiration time
    iat: u64,    // Issued at
    jti: String, // JWT ID

    // Private claims
    #[serde(flatten)]
    private: T,
}

impl<T> JWTClaims<T> {
    pub fn new(user_id: i64, site: &str, private: T) -> Self {
        JWTClaims {
            iss: site.to_owned(),
            sub: user_id.to_string(),
            aud: site.to_owned(),
            exp: (Utc::now() + Duration::days(1)).timestamp(),
            iat: get_current_timestamp(),
            jti: String::new(),
            private,
        }
    }
}
