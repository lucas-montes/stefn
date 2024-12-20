use std::num::ParseIntError;

use axum::{
    extract::{rejection::JsonRejection, FromRequest},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use maxminddb::MaxMindDBError;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

pub type AppResult<T> = std::result::Result<Json<T>, AppError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedQuery<T> {
    #[serde(flatten)]
    query: T,

    page: Option<i64>,
    per_page: Option<i64>,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct PaginatedResponse<T> {
//     data: Vec<T>,
//     total_pages: i64,
// }

// impl<T> PaginatedResponse<T> {
//     pub fn new(data: Vec<T>, total_pages: i64) -> Self {
//         Self { data, total_pages }
//     }
//     pub fn response(data: Vec<T>, total_pages: i64) -> AppResult<Self> {
//         Ok(Json(Self::new(data, total_pages)))
//     }
// }

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

#[derive(Debug)]
pub enum AppError {
    //
    WrongPassword(argon2::password_hash::Error),
    ErrorHashingPassword(argon2::password_hash::Error),
    // The request body contained invalid JSON
    JsonRejection(JsonRejection),
    JsonEnumDeserialization(serde_json::Error),
    //
    JWTError(jsonwebtoken::errors::Error),
    JWTModified(ParseIntError),
    //
    DoesNotExist,
    //
    RoleError,
    Unauthorized,
    //
    IpError(MaxMindDBError),
    IpDataNotFound,
    IpDatabaseNotEnabled,
    //
    Custom(StatusCode, String),
}

#[macro_export]
macro_rules! log_and_wrap_custom_internal {
    ($e:expr) => {{
        tracing::error!("{:?}", $e);
        AppError::custom_internal(&$e.to_string())
    }};
}

#[derive(Serialize, ToResponse, ToSchema)]
pub struct ErrorMessage {
    #[schema(example = "Sorry no sorry, something wrong happened")]
    message: String,
}

impl AppError {
    pub fn custom_internal(message: &str) -> Self {
        Self::Custom(StatusCode::INTERNAL_SERVER_ERROR, message.to_owned())
    }
    pub fn custom_bad_request(message: &str) -> Self {
        Self::Custom(StatusCode::BAD_REQUEST, message.to_owned())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{:?}", self);

        let (status, message) = match self {
            AppError::ErrorHashingPassword(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            AppError::WrongPassword(_err) => {
                (StatusCode::NOT_FOUND, "Contraseña incorrecta".into())
            }
            //
            AppError::JsonEnumDeserialization(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            AppError::JsonRejection(rejection) => (rejection.status(), rejection.body_text()),

            AppError::JWTError(err) => (StatusCode::UNAUTHORIZED, err.to_string()),
            AppError::JWTModified(err) => (StatusCode::UNAUTHORIZED, err.to_string()),

            AppError::RoleError => (StatusCode::UNAUTHORIZED, "Not authorized".to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Not authorized".to_string()),

            AppError::DoesNotExist => (StatusCode::NOT_FOUND, "Not found".into()),

            AppError::IpError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::IpDataNotFound => (StatusCode::INTERNAL_SERVER_ERROR, "Ip wrong".into()),
            AppError::IpDatabaseNotEnabled => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Ip database not enabled".into(),
            ),

            AppError::Custom(status, message) => (status, message),
        };

        (status, Json(ErrorMessage { message })).into_response()
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::JsonEnumDeserialization(error)
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        Self::JWTError(error)
    }
}
