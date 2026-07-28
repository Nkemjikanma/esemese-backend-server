use crate::common::errors::AppError;
use actix_web::http::StatusCode;
use actix_web::web::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct APIResponse<T> {
    pub response_message: String,
    pub response_data: T,
}

impl<T: Serialize> APIResponse<T> {
    pub fn success(items: T) -> Json<Self> {
        Json(Self {
            response_message: "success".to_string(),
            response_data: items,
        })
    }
}

pub type AppResponse<T> = Result<(Json<APIResponse<T>>, StatusCode), AppError>;
