use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use serde::Deserialize;

use crate::service::{AppError, AppState};

use super::services::get_user_from_valid_token;

pub async fn jwt_middleware<
    T: for<'a> Deserialize<'a> + std::marker::Send + std::marker::Sync + Clone + 'static,
>(
    State(state): State<AppState>,
    TypedHeader(bearer): TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user =
        get_user_from_valid_token::<T>(&state.config.domain, &state.keys.decoding, bearer.token())?;
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
