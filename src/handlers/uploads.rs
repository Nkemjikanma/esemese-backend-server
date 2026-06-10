use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;
use actix_web::http::StatusCode;
use crate::common::api::{APIResponse, AppResponse};
use crate::services::uploads::UploadsService;
use crate::types::app::AppState;
use crate::types::uploads::{InitiateUploadsRequest, ConfirmUploadsRequest, InitiateUploadResponse, ConfirmUploadsResponse};

// MIME type allowlist — image/jpeg, image/png, image/webp
const MAX_PHOTO_SIZE_BYTES: u64 = 50 * 1024 * 1024;
const MIN_PHOTO_SIZE_BYTES: u64 = 2 * 1024 * 1024;

// Start upload(s)
pub async fn initiate_uploads(app_state: web::Data<Arc<AppState>>, body: web::Json<InitiateUploadsRequest>) -> AppResponse<InitiateUploadResponse> {
    let presigned_obj = UploadsService::request_presigned_url(&body, &app_state).await?;

    Ok((APIResponse::success(presigned_obj), StatusCode::OK))

}

// confirm upload + save metadata
pub async fn confirm_uploads(app_state: web::Data<Arc<AppState>>, body: web::Json<ConfirmUploadsRequest>) -> AppResponse<ConfirmUploadsResponse> {
    let confirm_response = UploadsService::confirm_upload_exists(&body, &app_state).await?;

    Ok((APIResponse::success(confirm_response), StatusCode::ACCEPTED))
}