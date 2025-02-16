use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use axum_extra::{headers::Cookie, TypedHeader};
use std::net::SocketAddr;

use super::services::set_session_cookies;
use crate::{errors::AppError, models::UserSession, sessions::Session, state::WebsiteState};

pub async fn login_required_middleware(
    state: State<WebsiteState>,
    session: Extension<Session>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    //TODO: validate that the cookie is correct with hmac
    // TODO: check that the users exists and other validations
    let database = state.database();
    let config = state.config();

    if session.is_authenticated(database).await? {
        request.extensions_mut().insert(session);
        return Ok(next.run(request).await);
    }
    let next = format!("{}?next={}", config.login_path(), request.uri());
    Ok(Redirect::to(&next).into_response())
}

pub async fn sessions_middleware(
    state: State<WebsiteState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(cookie): TypedHeader<Cookie>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let sessions = state.sessions();
    let config = state.config();

    let current_session = match cookie.get(&config.session_cookie_name) {
        Some(session_id) => {
            sessions
                .find_session(session_id, &config.session_key)
                .await?
        }
        None => None,
    };
    let session = match current_session {
        Some(session) => session,
        None => {
            //TODO: improve overall
            let country = state.get_country_code_from_ip(&addr).ok().map(|s| s.into());
            sessions
                .create_session(
                    UserSession::default(),
                    config.session_expiration as u64,
                    &config.session_key,
                    country,
                )
                .await?
        }
    };

    request.extensions_mut().insert(session.clone());
    //Before the response

    let mut resp = next.run(request).await;

    set_session_cookies(resp.headers_mut(), &session, config).await?;

    Ok(resp)
}
