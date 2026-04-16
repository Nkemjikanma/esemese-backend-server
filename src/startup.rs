use crate::{common::errors::AppError, config, routes::auth::configure_auth, types::app::AppState};
use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::NormalizePath;
use actix_web::{App, HttpResponse, HttpServer, dev::Server, http, web};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tracing_actix_web::TracingLogger;
pub fn run(listener: TcpListener, app_state: Arc<AppState>) -> Result<Server, AppError> {
    let connection = web::Data::new(app_state);

    let governor_config = GovernorConfigBuilder::default()
        .seconds_per_request(1)
        .burst_size(6)
        .finish()
        .unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:3000")
                    .allowed_origin("https://nkem.dev")
                    .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                    .allowed_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::ACCEPT,
                        http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600),
            )
            .wrap(Governor::new(&governor_config))
            .wrap(NormalizePath::trim())
            .wrap(TracingLogger::default())
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().finish() }),
            )
            .configure(configure_auth)
            .service(web::scope("/api"))
            .app_data(connection.clone())
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
