use askama::Template;
use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::get,
    Extension, Form, Router,
};

use crate::{service::AppError, sessions::Session, state::WebsiteState, website::Meta};

use super::services::{handle_ingress, IngressForm, IngressParams};

pub fn routes(state: WebsiteState) -> Router<WebsiteState> {
    Router::new()
        .route("/ingress", get(ingress).post(post_ingress))
        .with_state(state)
}

#[derive(Template)]
#[template(path = "auth/ingress.html")]
struct IngressTemplate<'a> {
    meta: Meta<'a>,
}

pub async fn ingress<'a>() -> IngressTemplate<'a> {
    let meta = Meta::new(
        "Favoris".into(),
        "Tes favoris".into(),
        "smartlink,b2b".into(),
        "Lucas Montes".into(),
        "smartlink.ai".into(),
        "image.com".into(),
    );
    IngressTemplate { meta }
}

pub async fn post_ingress(
    state: State<WebsiteState>,
    Extension(session): Extension<Session>,
    params: Query<IngressParams>,
    input: Form<IngressForm>,
) -> Result<Redirect, AppError> {
    handle_ingress(&state, session, &params, &input)
        .await
        .map(|r| Redirect::to(r))
}
