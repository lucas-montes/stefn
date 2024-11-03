use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::{headers::Cookie, TypedHeader};

use crate::{config::APIConfig, sessions::Sessions, AppError};

pub async fn login_required_middleware(
    State(sessions): State<Sessions>,
    State(config): State<APIConfig>,
    TypedHeader(cookie): TypedHeader<Cookie>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    //TODO: validate that the cookie is correct with hmac
    if let Some(session_id) = cookie.get(&config.session_cookie_name) {
        if let Some(session) = sessions.find_session(session_id).await? {
            request.extensions_mut().insert(session);
            return Ok(next.run(request).await);
        }
    }
    let next = format!("login?next={}?oupsi=1&doupsi=2", request.uri());
    Ok(Redirect::to(&next).into_response())
}
