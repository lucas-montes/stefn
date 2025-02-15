use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::decode;
use serde::Deserialize;

use crate::{errors::AppError, state::APIState};

use super::{dtos::JWTClaims, JWTUserRequest};

pub async fn jwt_middleware<
    U: for<'a> Deserialize<'a> + std::marker::Send + std::marker::Sync + Clone + 'static,
>(
    State(state): State<APIState>,
    TypedHeader(bearer): TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = decode::<JWTClaims<U>>(bearer.token(), state.decoding(), state.validator())
        .map_err(AppError::JWTError)?;

    let user = JWTUserRequest::new(token.claims)?;
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
