use axum::{
    extract::{Extension, Query, State},
    response::Redirect,
};
use serde::Deserialize;

use crate::{errors::AppError, sessions::Session, state::WebsiteState};

use super::{
    oauth::{CallbackValidation, OauthTokenResponse},
    service::GoogleOauthCallbackHook,
};

#[derive(Debug, Deserialize)]
pub struct LoginParam {
    next: Option<String>,
}

pub async fn start_oauth(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    Query(params): Query<LoginParam>,
) -> Result<Redirect, AppError> {
    let config = state.config();
    let database = state.database();
    let return_url = params.next.as_ref().unwrap_or(&config.login_redirect_to);

    if session.is_authenticated(database).await? {
        return Ok(Redirect::to(return_url));
    };

    let authorize_url = CallbackValidation::new(config, config.google_scopes())?
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

pub async fn oauth_return<T: GoogleOauthCallbackHook + Send>(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    Query(params): Query<GoogleOauthParams>,
) -> Result<Redirect, AppError> {
    let config = state.config();
    let database = state.database();

    let (pkce_code, return_url) = CallbackValidation::validate(params.state, database).await?;

    let token_response = OauthTokenResponse::login(config, params.code, pkce_code).await?;

    T::run(&token_response, session, &state).await?;

    Ok(Redirect::to(&return_url))
}
