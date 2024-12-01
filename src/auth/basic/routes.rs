use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use serde::Deserialize;

use crate::{sessions::Session, AppError, WebsiteState};

use super::{infrastructures::find_user_by_email, services::verify_password};

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct Params {
    next: Option<String>,
}

pub async fn login_user(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    params: Query<Params>,
    input: Form<LoginForm>,
) -> Result<Response, AppError> {
    let user = find_user_by_email(&state.database(), &input.email).await?;
    verify_password(&input.password, &user.password)?;

    let config = state.config();
    let sessions = state.sessions();

    sessions
        .reuse_current_as_new_one(session, user.pk, user.groups)
        .await?;

    let redirect = params.next.as_ref().unwrap_or(&config.login_redirect_to);
    Ok(Redirect::to(&redirect).into_response())
}
