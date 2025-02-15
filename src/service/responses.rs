
use axum::{
    extract::{ FromRequest},

    response::{IntoResponse, Json, Response},
};

use serde::{Deserialize, Serialize};
use sqlx::error::DatabaseError;
use utoipa::{ToResponse, ToSchema};

use crate::errors::AppError;

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

#[derive(Serialize, ToResponse, ToSchema)]
pub struct ErrorMessage {
    #[schema(example = "Sorry no sorry, something wrong happened")]
    message: String,
}

impl ErrorMessage{
    pub fn new(message: String) -> Self {
        Self { message }
    }
}