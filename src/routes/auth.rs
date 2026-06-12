use actix_governor::{self, Governor, GovernorConfig, PeerIpKeyExtractor, governor::{middleware::NoOpMiddleware}};
use crate::handlers::auth;
use actix_web::web::{self, ServiceConfig};
pub fn configure_auth(cfg: &mut ServiceConfig, gov: &GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware> ) {
    cfg.service(web::scope("/auth").wrap(Governor::new(gov)).route("login", web::post().to(auth::login)));
}
