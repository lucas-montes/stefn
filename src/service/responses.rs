use std::num::ParseIntError;

use axum::{
    extract::{rejection::JsonRejection, FromRequest},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use maxminddb::MaxMindDBError;
use serde::{Deserialize, Serialize};
use sqlx::error::DatabaseError;
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
    JWTModified(ParseIntError), //TODO: wrong error
    //
    TemplateError(askama::Error),
    //
    DoesNotExist,
    UniqueViolation(String),
    ForeignKeyViolation(String),
    NotNullViolation(String),
    CheckViolation(String),
    DatabaseError(String),

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

    pub fn get_status_code_and_message(self) -> (StatusCode, String) {
        match self {
            Self::TemplateError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),

            Self::ErrorHashingPassword(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Self::WrongPassword(err) => (StatusCode::NOT_FOUND, err.to_string()),
            //
            Self::JsonEnumDeserialization(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            Self::JsonRejection(rejection) => (rejection.status(), rejection.body_text()),

            Self::JWTError(err) => (StatusCode::UNAUTHORIZED, err.to_string()),
            Self::JWTModified(err) => (StatusCode::UNAUTHORIZED, err.to_string()),

            Self::RoleError => (StatusCode::UNAUTHORIZED, "Not authorized".to_string()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Not authorized".to_string()),

            Self::DoesNotExist => (StatusCode::NOT_FOUND, "Not found".into()),
            Self::UniqueViolation(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::ForeignKeyViolation(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::NotNullViolation(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::CheckViolation(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),

            Self::IpError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Self::IpDataNotFound => (StatusCode::INTERNAL_SERVER_ERROR, "Ip wrong".into()),
            Self::IpDatabaseNotEnabled => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Ip database not enabled".into(),
            ),

            Self::Custom(status, message) => (status, message),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error=?&self);

        let (status, message) = self.get_status_code_and_message();

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

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateError(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(db_err) => {
                // Check for specific database error codes
                let sqlite_error = db_err.downcast_ref::<sqlx::sqlite::SqliteError>();
                match sqlite_error.kind() {
                    sqlx::error::ErrorKind::UniqueViolation => {
                        AppError::UniqueViolation(db_err.message().to_string())
                    }
                    sqlx::error::ErrorKind::ForeignKeyViolation => {
                        AppError::ForeignKeyViolation(db_err.message().to_string())
                    }
                    sqlx::error::ErrorKind::NotNullViolation => {
                        AppError::NotNullViolation(db_err.message().to_string())
                    }
                    sqlx::error::ErrorKind::CheckViolation => {
                        AppError::CheckViolation(db_err.message().to_string())
                    }
                    sqlx::error::ErrorKind::Other => {
                        AppError::DatabaseError(db_err.message().to_string())
                    }
                    _ => AppError::DatabaseError(db_err.to_string()),
                }
            }
            sqlx::Error::RowNotFound => AppError::DoesNotExist,
            sqlx::Error::PoolTimedOut => {
                AppError::DatabaseError("Database pool timed out".to_string())
            }
            sqlx::Error::Io(err) => AppError::DatabaseError(format!("IO error: {}", err)),
            _ => {
                // Fallback for other unhandled SQLx errors
                AppError::custom_internal(&format!("Unhandled SQLx error: {}", error))
            }
        }
    }
}

// impl std::error::Error for AppError{}
// impl Display for AppError {}
