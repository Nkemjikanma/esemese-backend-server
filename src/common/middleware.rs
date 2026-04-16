use crate::{
    common::{errors::AppError, utils::JWT},
    types::app::AppState,
};
use actix_web::{Error, dev::ServiceRequest, web};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use secrecy::ExposeSecret;

use std::sync::Arc;
pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let token = credentials.token();

    let app_state = req
        .app_data::<web::Data<Arc<AppState>>>()
        .expect("App state not found");

    let secret = app_state.app_config.application.jwt_secret.expose_secret();

    match JWT::verify_token(token, secret) {
        Ok(_claims) => Ok(req),
        Err(_) => Err((AppError::InvalidToken.into(), req)),
    }
}
