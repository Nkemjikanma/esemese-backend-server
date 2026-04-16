use crate::handlers::auth;
use actix_web::web::{self, ServiceConfig};
pub fn configure_auth(cfg: &mut ServiceConfig) {
    cfg.service(web::scope("/auth").route("login", web::post().to(auth::login)));
}
