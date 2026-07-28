use crate::common::api::{APIResponse, AppResponse};
use crate::services::collections::CollectionsService;
use crate::types::app::AppState;
use crate::types::collections::{Collection, CollectionsQueryInfo};
use crate::types::common::{PaginatedResponse, Pagination};
use actix_web::http::StatusCode;
use actix_web::web;
use std::sync::Arc;

pub async fn get_all_collections(
    pagination: web::Query<Pagination>,
    app_state: web::Data<Arc<AppState>>,
) -> AppResponse<PaginatedResponse<Collection>> {
    let collections =
        CollectionsService::get_all_collections(pagination.into_inner(), &app_state.connection)
            .await?;

    Ok((APIResponse::success(collections), StatusCode::OK))
}

pub async fn get_collection_by_slug(
    query: web::Path<String>,
    app_state: web::Data<Arc<AppState>>,
) -> AppResponse<Collection> {
    let collection =
        CollectionsService::get_collection_by_slug(query.into_inner(), &app_state.connection)
            .await?;

    Ok((APIResponse::success(collection), StatusCode::OK))
}
