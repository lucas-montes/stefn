use axum::{
    extract::{
        rejection::{FormRejection, JsonRejection},
        FromRef, FromRequest, Request,
    },
    Extension, Form, Json, RequestExt,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use validator::Validate;

use crate::{
    config::WebsiteConfig, log_and_wrap_custom_internal, service::AppError, sessions::Session,
    state::HttpClient,
};

#[derive(Debug, Deserialize)]
struct CloudflareCaptchaResponse {
    success: bool,
    #[serde(rename = "error-code")]
    error_code: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CloudflareCaptchaParams<'a> {
    secret: &'a str,
    response: &'a str,
    remoteip: Option<String>,
    idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CaptchaForm<T: std::fmt::Debug> {
    #[serde(flatten)]
    pub data: T,
    #[serde(rename = "cf-turnstile-response")]
    cf_turnstile_response: String,
    csrf_token: String,
}

impl<T: std::fmt::Debug> CaptchaForm<T> {
    pub fn data(self) -> T {
        self.data
    }
}

impl<S, T> FromRequest<S> for CaptchaForm<T>
where
    Form<CaptchaForm<T>>: FromRequest<S, Rejection = FormRejection>,
    WebsiteConfig: FromRef<S>,
    HttpClient: FromRef<S>,
    T: DeserializeOwned + Validate + Send + std::fmt::Debug,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(mut req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let session = req
            .extract_parts::<Extension<Session>>()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        let cf_ip = req
            .headers()
            .get("CF-Connecting-IP")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_owned());

        // let content_type = req
        //     .headers()
        //     .get(CONTENT_TYPE)
        //     .and_then(|value| value.to_str().ok())
        //     .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;
        //     TODO: check also the cookies from the headers

        let Form(payload) = Form::<CaptchaForm<T>>::from_request(req, _state)
            .await
            .map_err(|e| AppError::custom_bad_request(&e.to_string()))?;

        let config = WebsiteConfig::from_ref(_state);

        session
            .validate_csrf_token(&config.session_key, &payload.csrf_token)
            .await?;

        payload
            .data
            .validate()
            .map_err(|e| AppError::custom_bad_request(&e.to_string()))?;

        let http_client = HttpClient::from_ref(_state);

        let form = CloudflareCaptchaParams {
            secret: &config.captcha_secret_key,
            response: &payload.cf_turnstile_response,
            remoteip: cf_ip,
            idempotency_key: None,
        };
        let verification: CloudflareCaptchaResponse = http_client
            .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
            .json(&form)
            .send()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?
            .json()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        //TODO: create custom errors for captcha
        if !verification.success {
            AppError::custom_bad_request(&verification.error_code.unwrap().join(","));
        };

        Ok(payload)
    }
}

#[derive(Debug, Deserialize)]
pub struct SecureForm<T> {
    #[serde(flatten)]
    data: T,
    csrf_token: String,
}

impl<T> SecureForm<T> {
    pub fn data(self) -> T {
        self.data
    }
}

impl<S, T: Send> FromRequest<S> for SecureForm<T>
where
    Form<SecureForm<T>>: FromRequest<S, Rejection = FormRejection>,
    WebsiteConfig: FromRef<S>,
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(mut req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let session = req
            .extract_parts::<Extension<Session>>()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        // let content_type = req
        //     .headers()
        //     .get(CONTENT_TYPE)
        //     .and_then(|value| value.to_str().ok())
        //     .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;
        //     TODO: check also the cookies from the headers

        let Form(payload) = Form::<SecureForm<T>>::from_request(req, _state)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        let config = WebsiteConfig::from_ref(_state);

        session
            .validate_csrf_token(&config.session_key, &payload.csrf_token)
            .await?;

        payload
            .data
            .validate()
            .map_err(|e| log_and_wrap_custom_internal!(e))?; //TODO: return a 4xx

        Ok(payload)
    }
}

#[derive(Debug, Deserialize)]
pub struct SecureJson<T> {
    #[serde(flatten)]
    data: T,
    csrf_token: String,
}

impl<T> SecureJson<T> {
    pub fn data(self) -> T {
        self.data
    }
}

impl<S, T: Send> FromRequest<S> for SecureJson<T>
where
    Json<SecureJson<T>>: FromRequest<S, Rejection = JsonRejection>,
    WebsiteConfig: FromRef<S>,
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(mut req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let session = req
            .extract_parts::<Extension<Session>>()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        // let content_type = req
        //     .headers()
        //     .get(CONTENT_TYPE)
        //     .and_then(|value| value.to_str().ok())
        //     .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;
        //     TODO: check also the cookies from the headers

        let Json(payload) = Json::<SecureJson<T>>::from_request(req, _state)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        let config = WebsiteConfig::from_ref(_state);

        session
            .validate_csrf_token(&config.session_key, &payload.csrf_token)
            .await?;

        payload
            .data
            .validate()
            .map_err(|e| log_and_wrap_custom_internal!(e))?; //TODO: return a 4xx

        Ok(payload)
    }
}
