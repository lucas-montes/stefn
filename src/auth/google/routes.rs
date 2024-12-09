use axum::{
    extract::{Extension, Host, Query, State},
    response::Redirect,
};
use serde::Deserialize;

use crate::{service::AppError, sessions::Session, state::WebsiteState};

use super::{
    oauth::{CallbackValidation, OauthTokenResponse},
    service::GoogleOauthCallbackHook,
};

#[derive(Debug, Deserialize)]
pub struct LoginParam {
    next: Option<String>,
}

async fn start_oauth(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    Query(params): Query<LoginParam>,
    Host(hostname): Host,
) -> Result<Redirect, AppError> {
    let config = state.config();
    let database = state.database();
    let return_url = params.next.as_ref().unwrap_or(&config.login_redirect_to);

    if session.is_authenticated().await {
        return Ok(Redirect::to(return_url));
    };

    let authorize_url = CallbackValidation::new(hostname, config)?
        .save(database, return_url)
        .await?
        .authorize_url();

    Ok(Redirect::to(authorize_url.as_str()))
}

#[derive(Debug, Deserialize)]
pub struct GoogleOauthParams {
    pub state: String,
    pub code: String,
}

async fn oauth_return<T: GoogleOauthCallbackHook>(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    Query(params): Query<GoogleOauthParams>,
    Host(hostname): Host,
) -> Result<Redirect, AppError> {
    let config = state.config();
    let database = state.database();

    let (pkce_code, return_url) = CallbackValidation::validate(params.state, database).await?;

    let token_response = OauthTokenResponse::request(
        config.google_client_id.clone(),
        config.google_client_secret.clone(),
        params.code,
        pkce_code,
        hostname,
    )
    .await?;

    T::run(&token_response, session, &state).await?;

    Ok(Redirect::to(&return_url))
}
