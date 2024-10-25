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

pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
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
struct JWTClaims<T> {
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
    fn new(user_id: i64, site: &str, private: T) -> Self {
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

pub fn create_token(user_id: i64, user_role: &str, state: &AppState) -> Result<String, AppError> {
    encode(
        &Header::default(),
        &JWTClaims::new(user_id, &state.domain, user_role),
        &state.keys.encoding,
    )
    .map_err(AppError::JWTError)
}

#[derive(Clone)]
pub struct JWTUserRequest<T> {
    pub id: i64,
    private: T,
}

pub async fn jwt_middleware<
    T: for<'a> Deserialize<'a> + std::marker::Send + std::marker::Sync + Clone + 'static,
>(
    State(state): State<AppState>,
    TypedHeader(bearer): TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let mut validation = Validation::default();
    validation.set_audience(&[&state.domain]); // TODO: get it from config
    validation.set_issuer(&[&state.domain]); // TODO: get it from config
    validation.leeway = 60 * 60 * 60 * 24 * 30; //TODO: keep at one hour instead of days
    validation.reject_tokens_expiring_in_less_than = 86400u64;
    validation.set_required_spec_claims(&["iss", "sub", "aud", "exp", "iat", "jti", "private"]);

    let token_data = decode::<JWTClaims<T>>(bearer.token(), &state.keys.decoding, &validation)
        .map_err(AppError::JWTError)?;

    request.extensions_mut().insert(JWTUserRequest {
        id: token_data
            .claims
            .sub
            .parse::<i64>()
            .map_err(AppError::JWTModified)?,
        private: token_data.claims.private.clone(),
    });

    Ok(next.run(request).await)
}
