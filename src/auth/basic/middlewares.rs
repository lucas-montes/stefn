use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use axum_extra::{headers::Cookie, TypedHeader};

use crate::{sessions::Session, AppError, WebsiteState};

use super::services::set_session_cookies;

pub async fn login_required_middleware(
    session: Extension<Session>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    //TODO: validate that the cookie is correct with hmac

    if session.is_authenticated().await {
        request.extensions_mut().insert(session);
        return Ok(next.run(request).await);
    }
    let next = format!("/?next={}", request.uri());
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
        Some(session_id) => sessions.find_session(session_id).await?,
        None => None,
    };

    let session = match current_session {
        Some(session) => session,
        None => {
            //TODO: how to handle no ip database
            let country = state
                .get_country_code_from_ip(&addr)
                .unwrap_or("No-countries");

            sessions
                .create_session(
                    None,
                    String::new(),
                    config.session_expiration as u64,
                    country,
                )
                .await?
        }
    };

    request.extensions_mut().insert(session.clone());
    //Before the response
    let mut resp = next.run(request).await;
    //After the response
    let session = sessions
        .update_session(session, &config.session_key)
        .await?;

    set_session_cookies(resp.headers_mut(), &session, config).await?;

    Ok(resp)
}
