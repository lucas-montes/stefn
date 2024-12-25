use axum::{
    async_trait,
    extract::{FromRef, FromRequest, Request},
    response::{IntoResponse, Response},
    Extension, Form, RequestExt,
};
use serde::Deserialize;

use crate::{config::WebsiteConfig, sessions::Session};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SecureForm<T: Clone + Default> {
    data: T,
    csrf_token: String,
}

impl<T: Clone + Default> SecureForm<T> {
    pub fn data(self) -> T {
        self.data
    }
}

#[async_trait]
impl<S, T: Clone + Default + Send> FromRequest<S> for SecureForm<T>
where
    Form<SecureForm<T>>: FromRequest<()>,
    WebsiteConfig: FromRef<S>,
    T: 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(mut req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let session = req
            .extract_parts::<Extension<Session>>()
            .await
            .map_err(|err| err.into_response())?;

        // let content_type = req
        //     .headers()
        //     .get(CONTENT_TYPE)
        //     .and_then(|value| value.to_str().ok())
        //     .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;

        let Form(payload) = req
            .extract::<Form<SecureForm<T>>, _>()
            .await
            .map_err(|err| err.into_response())?;

        let config = WebsiteConfig::from_ref(_state);

        session
            .validate_csrf_token(&config.session_key, &payload.csrf_token)
            .await
            .map_err(|err| err.into_response())?;

        Ok(payload)
    }
}
