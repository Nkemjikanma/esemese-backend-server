use esemese_backend_server::{
    common::errors::AppError,
    config::{Config, Environment},
    startup::{create_pool, run},
    telemetry::{get_subscriber, init_subscriber},
    types::app::AppState,
};

use std::net::TcpListener;
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), AppError> {
    let environment: Environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "development".to_string())
        .try_into()
        .expect("Failed to load env");

    let env_file = format!(".env");

    dotenvy::from_filename(&env_file).ok();

    let subscriber = get_subscriber("esemese_backend".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = Config::load_config()?;

    let address = format!("{}:{}", config.application.host, config.application.port);

    let listener = TcpListener::bind(address)?;

    tracing::info!("Listening here: {:?}", listener);

    let connection = create_pool(&config.database).expect("Failed to connect to postgres");
    let app_state = Arc::new(AppState {
        app_config: config,
        connection,
    });

    run(listener, app_state)
        .map_err(|e| -> AppError { e })?
        .await
        .map_err(Into::into)
}
