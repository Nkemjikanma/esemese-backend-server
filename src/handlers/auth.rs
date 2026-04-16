use crate::{
    common::api::{APIResponse, AppResponse},
    services::auth::AuthServices,
    types::{app::AppState, auth::LoginForm},
};

use actix_web::web;
use std::sync::Arc;

pub async fn login(
    login_form: web::Json<LoginForm>,
    app_state: web::Data<Arc<AppState>>,
) -> AppResponse<String> {
    AuthServices::login(login_form.into_inner(), &app_state.app_config.application).await?;

    Ok(APIResponse::success(
        "User successfully authenticated".to_string(),
    ))
}
