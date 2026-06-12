use esemese_backend_server::{
    common::errors::AppError,
    config::{Config, Environment},
    startup::{create_pool, run},
    telemetry::{get_subscriber, init_subscriber},
    types::app::AppState,
};

use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::config::Credentials;
use std::net::TcpListener;
use std::sync::Arc;
use aws_sdk_s3::Client;

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

    let credentials = Credentials::new(
        &config.rustfs_config.access_key_id,
        &config.rustfs_config.secret_access_key,
        None,
        None,
        "rustfs",
    );

    let extracted_region = config.rustfs_config.region.clone();

    let region = Region::new(extracted_region);

    let endpoint_url = &config.rustfs_config.endpoint_public;

    let shared_config = aws_config::defaults(BehaviorVersion::latest()).region(region).credentials_provider(credentials).endpoint_url(endpoint_url).load().await;

    let rustfs_client = Client::from_conf(aws_sdk_s3::config::Builder::from(&shared_config).force_path_style(true).build());

    // Force building an S3-specific config with path-style enabled
    let connection = create_pool(&config.database).expect("Failed to connect to postgres");
    let app_state = Arc::new(AppState {
        app_config: config,
        connection,
        rustfs_client,
    });


    run(listener, app_state)
        .map_err(|e| -> AppError { e })?
        .await
        .map_err(Into::into)
}
