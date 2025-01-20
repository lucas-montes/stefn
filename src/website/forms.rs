use axum::{
    async_trait,
    extract::{rejection::FormRejection, FromRef, FromRequest, Request},
    Extension, Form, RequestExt,
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
    error_code: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CloudflareCaptchaParams<'a> {
    secret: &'a str,
    response: &'a str,
    remoteip: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct CaptchaForm<T> {
    #[serde(flatten)]
    pub data: T,
    #[serde(rename = "cf-turnstile-response")]
    cf_turnstile_response: String,
}

#[async_trait]
impl<S, T: Send> FromRequest<S> for CaptchaForm<T>
where
    SecureForm<CaptchaForm<T>>: FromRequest<S, Rejection = FormRejection>,
    WebsiteConfig: FromRef<S>,
    HttpClient: FromRef<S>,
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let cf_ip = req
            .headers()
            .get("CF-Connecting-IP")
            .ok_or(AppError::custom_bad_request("missing captcha ip"))?
            .to_str()
            .map_err(|e| log_and_wrap_custom_internal!(e))?
            .to_owned();

        let payload = SecureForm::<CaptchaForm<T>>::from_request(req, _state)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        let payload = payload.data;
        let config = WebsiteConfig::from_ref(_state);
        let http_client = HttpClient::from_ref(_state);

        let form = CloudflareCaptchaParams {
            secret: &config.captcha_secrect_key,
            response: &payload.cf_turnstile_response,
            remoteip: &cf_ip,
        };
        let verification: CloudflareCaptchaResponse = http_client
            .0
            .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
            .form(&form)
            .send()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?
            .json()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        //TODO: create custom errors for captcha
        if !verification.success {
            AppError::custom_internal(&verification.error_code.join(","));
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

#[async_trait]
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

        payload
            .data
            .validate()
            .map_err(|e| log_and_wrap_custom_internal!(e))?; //TODO: return a 4xx
        let config = WebsiteConfig::from_ref(_state);

        session
            .validate_csrf_token(&config.session_key, &payload.csrf_token)
            .await?;

        Ok(payload)
    }
}
