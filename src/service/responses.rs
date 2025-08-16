use axum::{
    extract::FromRequest,
    response::{IntoResponse, Json, Response},
};

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToResponse, ToSchema};

use crate::errors::AppError;

pub type AppResult<T> = std::result::Result<Json<T>, AppError>;

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct Pagination {
    page: Option<u64>,
    offset: Option<u64>,
    per_page: Option<u16>,
}

impl Pagination {
    pub fn offset(&self) -> u64 {
        self.offset.unwrap_or_default()
    }

    pub fn page(&self) -> u64 {
        self.page.unwrap_or_default()
    }

    pub fn per_page(&self) -> u16 {
        self.per_page.unwrap_or(20).min(100)
    }
}

#[derive(Debug, Serialize, Deserialize, ToResponse, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    data: Vec<T>,
    total_pages: u64,
}

impl<T: ToSchema> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total_pages: u64) -> Self {
        Self { data, total_pages }
    }
}

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

impl ErrorMessage {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
