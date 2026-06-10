use actix_web::web;
use crate::handlers::uploads;
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::common::middleware::validator;

pub fn configure_uploads(cfg: &mut web::ServiceConfig) {
    let auth_middleware = HttpAuthentication::bearer(validator);

    cfg.service(
        web::scope("/uploads").wrap(auth_middleware).route("/initiate", web::post().to(uploads::initiate_uploads) ).route("/confirm", web::post().to(uploads::confirm_uploads) )
    );
}