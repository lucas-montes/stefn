
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use axum::{
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use http_body_util::BodyExt;
use serde::Serialize;
use stefn::{
    auth::{sessions_middleware, EmailValidation, Ingress},
    service::{Service, StubService},
    state::WebsiteState,
};

struct FormulaireCompletedTemplate;

async fn save_proposal(
    Path(slug): Path<String>,
    database: State<Database>,
    proposal: CaptchaForm<ProposalForm>,
) -> Result<FormulaireCompletedTemplate, AppError> {
    let proposal = proposal.data;
    proposal.save(slug, &database).await?;
    Ok(FormulaireCompletedTemplate {})
}


fn routes(state: WebsiteState) -> Router<WebsiteState> {
    Router::new()
        .route("/ingress", post(IngressService::route))
        .route("/email-validation-sent", get(|| async { "Hello" }))
        .route(
            "/email-validation/:slug",
            get(EmailValidationService::route),
        )
        .layer(from_fn_with_state(state.clone(), sessions_middleware))
        .with_state(state)
}

async fn setup() -> StubService {
    StubService::new(Service::website("WEB_", routes)).await
}

#[derive(Debug, Serialize)]
pub struct IngressFormTest {
    email: String,
    password: String,
}

#[tokio::test]
async fn test_ingress() {
    let app = setup().await;

    let cookies_response = app
        .request(
            Request::builder()
                .uri("/email-validation-sent")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    let mut headers = cookies_response.headers().get_all("set-cookie").iter();
    let csrf = headers.next().unwrap();

    let f = IngressFormTest {
        email: "example@gmail.com".into(),
        password: String::default(),
        csrf_token: csrf
            .to_str()
            .unwrap()
            .split_once(";")
            .unwrap()
            .0
            .replace("csrf_token=", ""),
        process: IngressProcess::Register,
    };
    let body = Form(f).into_response().into_body();

    let response = app
        .request(
            Request::builder()
                .method("POST")
                .uri("/ingress")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", csrf)
                .header("Cookie", headers.next().unwrap())
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

#[tokio::test]
async fn test_get_token() {
    let app = setup().await;

    let response = app
        .request(
            Request::builder()
                .uri("/api/v1/auth/token")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
