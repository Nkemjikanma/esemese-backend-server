use crate::{
    common::api::{APIResponse, AppResponse},
    services::auth::AuthServices,
    types::{app::AppState, auth::LoginForm},
};

use actix_web::web;
use std::sync::Arc;
use actix_web::http::StatusCode;

pub async fn login(
    login_form: web::Json<LoginForm>,
    app_state: web::Data<Arc<AppState>>,
) -> AppResponse<String> {
    let token =
        AuthServices::login(login_form.into_inner(), &app_state.app_config.application).await?;

    Ok((APIResponse::success(token.to_string()), StatusCode::OK))
}
