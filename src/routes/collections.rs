use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::common::middleware::validator;
use crate::handlers::collections::{get_all_collections, get_collection_by_slug};

pub fn configure_collections(cfg: &mut web::ServiceConfig) {
	// setup middleware
	let auth_middleware = HttpAuthentication::bearer(validator);

	cfg.service(
		web::scope("/collections")
			.route("", web::get().to(get_all_collections))
			.route("/{slug}", web::get().to(get_collection_by_slug))
	);
}