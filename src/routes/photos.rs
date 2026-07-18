use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::common::middleware::validator;
use crate::handlers::photos;

pub fn configure_photos(cfg: &mut web::ServiceConfig) {
	// setup middleware
	let auth_middleware = HttpAuthentication::bearer(validator);

	cfg.service(
		web::scope("/photos")
			.route("", web::get().to(photos::get_all_photos))
			.route("/{id}", web::get().to(photos::get_photo_by_id)),
	);
}