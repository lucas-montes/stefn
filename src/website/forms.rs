use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
    Form, RequestExt,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SecureForm<T: Clone + Default> {
    data: T,
    token: String,
}

#[async_trait]
impl<S, T: Clone + Default> FromRequest<S> for SecureForm<T>
where
    Form<SecureForm<T>>: FromRequest<()>,
    T: 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;

        let Form(payload) = req
            .extract::<Form<SecureForm<T>>, _>()
            .await
            .map_err(|err| err.into_response())?;

        Ok(payload)
    }
}
