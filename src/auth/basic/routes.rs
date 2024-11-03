use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header::SET_COOKIE, HeaderValue},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use cookie::time::Duration;
use serde::Deserialize;
use std::net::SocketAddr;

use crate::{
    config::{ServiceConfig, WebsiteConfig},
    sessions::Sessions,
    AppError, Database, IpsDatabase,
};

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
    database: State<Database>,
    ips_database: State<IpsDatabase>,
    sessions: State<Sessions>,
    config: State<WebsiteConfig>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<Params>,
    input: Form<LoginForm>,
) -> Result<Response, AppError> {
    let user = find_user_by_email(&database, &input.email).await?;
    verify_password(&input.password, &user.password)?;

    let country = ips_database.get_country_code_from_ip(addr)?;

    let session = sessions
        .create_session(
            user.pk,
            user.groups,
            config.session_expiration as u64,
            country,
        )
        .await?
        .new_csrf_token(&config.session_key);

    let cookie = cookie::Cookie::build(("session_id", session.id().to_string()))
        .domain(config.domain())
        .path("/")
        .max_age(Duration::days(config.session_expiration))
        .secure(true)
        .http_only(true)
        .build();

    let redirect = params.next.unwrap_or("dashboard".to_owned());
    let mut resp = Redirect::to(&redirect).into_response();
    resp.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_bytes(cookie.encoded().to_string().as_bytes())
            .map_err(|e| AppError::custom_internal(&e.to_string()))?,
    );
    Ok(resp)
}
