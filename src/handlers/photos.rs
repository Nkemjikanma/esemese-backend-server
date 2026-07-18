use actix_web::web;
use std::sync::Arc;
use actix_web::http::StatusCode;
use crate::common::api::{APIResponse, AppResponse};
use crate::types::{app::AppState, photos::{Photo, Photos, PhotosQueryInfo}};
use crate::services::photos::PhotosService;
use uuid::Uuid;
use crate::types::common::{PaginatedResponse, Pagination};

// TODO: List of all photos (Paginated, filterable by collection)
pub async fn get_all_photos(query: web::Query<PhotosQueryInfo>,
                            pagination: web::Query<Pagination>,
                            app_state: web::Data<Arc<AppState>>) -> AppResponse<PaginatedResponse<Photo>>{
	let photos = PhotosService::get_all_photos(query.into_inner(), pagination.into_inner(), &app_state.connection).await?;
	Ok((APIResponse::success(photos), StatusCode::OK))
}

pub async fn get_photo_by_id(path: web::Path<Uuid>, app_state: web::Data<Arc<AppState>>) -> AppResponse<Photo> {
	let book = PhotosService::get_photo_by_id(path.into_inner(), &app_state.connection).await?;

	Ok((APIResponse::success(book), StatusCode::OK))
}