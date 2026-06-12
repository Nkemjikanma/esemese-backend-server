use crate::{common::errors::AppError, config, routes::{auth::configure_auth, uploads::configure_uploads}, types::app::AppState};
use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::NormalizePath;
use actix_web::{App, HttpResponse, HttpServer, dev::Server, http, web, error, ResponseError};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tracing_actix_web::TracingLogger;
use crate::types::uploads::ContentType;

pub fn run(listener: TcpListener, app_state: Arc<AppState>) -> Result<Server, AppError> {
    let connection = web::Data::new(app_state);

    let governor_config = GovernorConfigBuilder::default()
        .seconds_per_request(1)
        .burst_size(6)
        .finish()
        .unwrap();

    let server = HttpServer::new(move || {
        // ensure correct mime type
        let json_config = web::JsonConfig::default().content_type(move |mime| {
            mime.to_string() == "image/jpg" || mime.to_string() == "image/png" || mime.to_string() == "image/webp"
        }).error_handler(|err, _req| {
            tracing::error!(%err, "A deserialization error has occured");

            let api_error = match &err {
                error::JsonPayloadError::Deserialize(e) => {
                    tracing::error!("Error deserializing the input");

                    AppError::InputValidationError(e.to_string())
                }
                error::JsonPayloadError::ContentType => {
                    tracing::error!("Wrong mime type provided");
                    AppError::InvalidContentType(err.to_string())
                }
                _ => {
                    tracing::error!("Something went wrong and we need to check the JsonConfig in startup.rs");
                    AppError::InputValidationError(err.to_string())
                }
            };

            error::InternalError::from_response(err, api_error.error_response()).into()
        });
        App::new()
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:3000")
                    .allowed_origin("https://esemese.xyz")
                    .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                    .allowed_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::ACCEPT,
                        http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600),
            )
            .wrap(NormalizePath::trim())
            .wrap(TracingLogger::default())
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().finish() }),
            )
            .configure(|cfg| configure_auth(cfg, &governor_config))
            .configure(configure_uploads)
            .service(web::scope("/api"))
            .app_data(connection.clone())
            .app_data(json_config)
    })
    .listen(listener)?
    .run();

    Ok(server)
}

#[tracing::instrument(name = "pool", skip_all)]
pub fn create_pool(config: &config::DBConfig) -> Result<PgPool, sqlx::Error> {
    tracing::info!("Creating database pool");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(200))
        .idle_timeout(Duration::from_secs(300))
        .connect_lazy_with(config.connection_option());

    tracing::info!("Database pool connection created");

    Ok(pool)
}
