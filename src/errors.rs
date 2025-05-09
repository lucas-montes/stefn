use std::num::ParseIntError;

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use maxminddb::MaxMindDBError;

use sqlx::error::DatabaseError;

use crate::service::ErrorMessage;

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
    TooManyRequests(reqwest::Error),
    UnauthorizedRequest(reqwest::Error),
    RequestFailed(reqwest::Error),
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

impl AppError {
    pub fn custom_internal(message: &str) -> Self {
        Self::Custom(StatusCode::INTERNAL_SERVER_ERROR, message.to_owned())
    }

    pub fn custom_bad_request(message: &str) -> Self {
        Self::Custom(StatusCode::BAD_REQUEST, message.to_owned())
    }

    pub fn get_status_code_and_message(self) -> (StatusCode, String) {
        match self {
            Self::TooManyRequests(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Self::UnauthorizedRequest(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Self::RequestFailed(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),

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

        (status, Json(ErrorMessage::new(message))).into_response()
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

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        match err.status() {
            Some(status) => {
                if status == StatusCode::TOO_MANY_REQUESTS {
                    AppError::TooManyRequests(err)
                } else if status == StatusCode::UNAUTHORIZED {
                    AppError::UnauthorizedRequest(err)
                } else {
                    AppError::RequestFailed(err)
                }
            }
            None => AppError::RequestFailed(err),
        }
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
