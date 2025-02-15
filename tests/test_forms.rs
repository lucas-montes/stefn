use axum::{
    body::Body,
    http::{Request, StatusCode},
    Json,
};
use axum::{
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use stefn::{
    auth::sessions_middleware, errors::AppError, service::{ Service, StubService}, state::WebsiteState, website::{CaptchaForm, SecureForm}
};
use validator::Validate;

#[derive(Debug, Serialize)]
struct IngressFormCsrfTest {
    email: String,
    password: String,
    csrf_token: String,
}

#[derive(Debug, Serialize)]
struct IngressFormCaptchaTest {
    email: String,
    password: String,
    csrf_token: String,
    #[serde(rename = "cf-turnstile-response")]
    cf_turnstile_response: String,
}

#[derive(Debug, Deserialize, Validate)]
struct IngressFormTest {
    _email: String,
    _password: String,
}

async fn form_with_csrf_and_captcha(
    _: CaptchaForm<IngressFormTest>,
) -> Result<Json<i64>, AppError> {
    Ok(Json(0))
}
async fn form_with_csrf(_: SecureForm<IngressFormTest>) -> Result<Json<i64>, AppError> {
    Ok(Json(0))
}

fn routes(state: WebsiteState) -> Router<WebsiteState> {
    Router::new()
        .route("/csrf", post(form_with_csrf))
        .route("/captcha", post(form_with_csrf_and_captcha))
        .route("/set-headers", get(|| async { "Hello" }))
        .layer(from_fn_with_state(state.clone(), sessions_middleware))
        .with_state(state)
}

async fn setup() -> StubService {
    StubService::new(Service::website("WEB_", routes)).await
}

#[tokio::test]
async fn test_form_with_csrf() {
    let app = setup().await;

    let cookies_response = app
        .request(
            Request::builder()
                .uri("/set-headers")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    let mut headers = cookies_response.headers().get_all("set-cookie").iter();
    let csrf = headers.next().unwrap();

    let f = IngressFormCsrfTest {
        email: "example@gmail.com".into(),
        password: String::default(),
        csrf_token: csrf
            .to_str()
            .unwrap()
            .split_once(";")
            .unwrap()
            .0
            .replace("csrf_token=", ""),
    };
    let body = Form(f).into_response().into_body();

    let response = app
        .request(
            Request::builder()
                .method("POST")
                .uri("/csrf")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", csrf)
                .header("Cookie", headers.next().unwrap())
                .body(body)
                .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_form_with_csrf_and_captcha() {
    let app = setup().await;

    let cookies_response = app
        .request(
            Request::builder()
                .uri("/set-headers")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    let mut headers = cookies_response.headers().get_all("set-cookie").iter();
    let csrf = headers.next().unwrap();

    let f = IngressFormCaptchaTest {
        email: "example@gmail.com".into(),
        password: String::default(),
        csrf_token: csrf
            .to_str()
            .unwrap()
            .split_once(";")
            .unwrap()
            .0
            .replace("csrf_token=", ""),
        cf_turnstile_response: "XXXX.DUMMY.TOKEN.XXXX".into(),
    };
    let body = Form(f).into_response().into_body();

    let response = app
        .request(
            Request::builder()
                .method("POST")
                .uri("/captcha")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", csrf)
                .header("Cookie", headers.next().unwrap())
                .header("CF-Connecting-IP", "127.0.0.1")
                .body(body)
                .unwrap(),
        )
        .await;

    println!("{:?}", &response);
    println!(
        "{:?}",
        response.into_body().collect().await.unwrap().to_bytes()
    );
    // assert_eq!(response.status(), StatusCode::OK);
}
