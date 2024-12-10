use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::get,
    Extension, Form, Router,
};

use crate::{service::AppError, sessions::Session, state::WebsiteState};

use super::services::{handle_ingress, handle_validate_email, IngressForm, IngressParams};

pub async fn ingress(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    params: Query<IngressParams>,
    input: Form<IngressForm>,
) -> Result<Redirect, AppError> {
    handle_ingress(&state, session, &params, &input)
        .await
        .map(|r| Redirect::to(r))
}

pub async fn validate_email(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    Path(slug): Path<String>,
) -> Result<Redirect, AppError> {
    handle_validate_email(&state, session, slug)
        .await
        .map(|r| Redirect::to(r))
}
