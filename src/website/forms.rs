use axum::{
    async_trait,
    extract::{rejection::FormRejection, FromRef, FromRequest, Request},
    response::{IntoResponse, Response},
    Extension, Form, RequestExt,
};
use serde::{de::DeserializeOwned, Deserialize};
use validator::Validate;

use crate::{
    config::WebsiteConfig, log_and_wrap_custom_internal, service::AppError, sessions::Session,
};


#[derive(Debug, Deserialize,  )]
pub struct SecureForm<T> {
    #[serde(flatten)]
    pub data: T,
    csrf_token: String,
}

impl<T> SecureForm<T> {
    pub fn data(self) -> T {
        self.data
    }
}

#[async_trait]
impl<S, T:   Send> FromRequest<S> for SecureForm<T>
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

        let Form(payload) = Form::<SecureForm<T>>::from_request(req, _state)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        payload.data.validate().map_err(|e| log_and_wrap_custom_internal!(e))?;
        let config = WebsiteConfig::from_ref(_state);

        session
            .validate_csrf_token(&config.session_key, &payload.csrf_token)
            .await?;

        Ok(payload)
    }
}
