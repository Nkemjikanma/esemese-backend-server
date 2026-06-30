use esemese_backend_server::{
    common::errors::AppError,
    config::{Config, Environment},
    startup::{create_pool, run},
    telemetry::{get_subscriber, init_subscriber},
    types::app::AppState,
};

use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::Client;
use esemese_backend_server::services::derivatives::process_derivative_for_photo;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let _environment: Environment = std::env::var("APP_ENV")
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

    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .credentials_provider(credentials)
        .endpoint_url(endpoint_url)
        .load()
        .await;

    let rustfs_client = Client::from_conf(
        aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(true)
            .build(),
    );

    let notify_on_confirm = Notify::new();

    // Force building an S3-specific config with path-style enabled
    let connection = create_pool(&config.database).expect("Failed to connect to postgres");
    let app_state = Arc::new(AppState {
        app_config: config,
        connection,
        rustfs_client,
        notify_on_confirm,
    });

    // clone app_state for workers
    let derivative_app_state = app_state.clone();
    let cleanup_app_state = app_state.clone();

    // workers encoding workers and cleanup workers.
    tokio::spawn(async move {
        run_derivatives_worker(derivative_app_state).await;
    });

    // worker to clean up
    tokio::spawn(async move {
        run_photos_cleanup(cleanup_app_state).await;
    });

    run(listener, app_state)
        .map_err(|e| -> AppError { e })?
        .await
        .map_err(Into::into)
}

// TODO: Let's comeback to confirm if the limit interval of 5 minutes is enough for batch derivatives genration
pub async fn run_derivatives_worker(app_state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 30));

    loop {
        tokio::select! {
          _ = app_state.notify_on_confirm.notified() => {}
            _ = interval.tick() => {}
        }

        // inner loop to ensure we claim all photos
        loop {
            // run claim query; shoult return a row or nothing
            // Limit is a contact saying - " I am working on these now"
            let pending_jobs = sqlx::query!(
                r#"UPDATE photos SET claimed_at = now(), attempts = attempts + 1 WHERE id IN
        (SELECT id FROM photos
                   WHERE status = 'processing'
                     AND (claimed_at IS NULL OR claimed_at < now() - interval '5 minutes')
                     AND attempts < 3
                   ORDER BY created_at
                   LIMIT 5)
                   RETURNING id, attempts"#
            )
            .fetch_all(&app_state.connection)
            .await;

            let jobs = match pending_jobs {
                Ok(jobs) => jobs,
                Err(e) => {
                    // database error so stop, wait for next trigger
                    tracing::warn!("Failed finding pending jobs, the worker will try again soon: {:?}", e);
                    break;
                }
            };

            if jobs.is_empty() {
                tracing::info!("No pending jobs were returned, nothing left. Back to waiting");
                break;
            }
            // got an id, call process_derivative_for_photo
            for row in jobs {
                if let Err(e) = process_derivative_for_photo(
                    row.id,
                    app_state.connection.clone(),
                    app_state.rustfs_client.clone(),
                    app_state.app_config.clone(),
                )
                .await
                {
                    tracing::error!(
                        "Derivative generation failed for photo_id={}: {}",
                        row.id,
                        e
                    );
                }
            }
        }
    }
}

pub async fn run_photos_cleanup(app_state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));

    loop {
        interval.tick().await;

        // mark photos with more than 3 attempts as failed
        if let Err(cleanup) = sqlx::query!(r#"UPDATE photos SET status = 'failed' WHERE status = 'processing'
                                              AND attempts >= 3
                                              AND (claimed_at IS NULL OR claimed_at < now() - interval '1 hour')"#)
           .execute(&app_state.connection).await {

           tracing::warn!("Photos with more than 3 attempts haven't been moved to failed: {:?}", cleanup);
       }

        if let Err(deletion) = sqlx::query!(
            r#"DELETE FROM photos WHERE status = 'initiated' and created_at < now() -
       interval '1 hours'"#
        )
        .execute(&app_state.connection)
        .await
        {
            tracing::warn!("Deleting orphaned Upload URL failed: {:?}", deletion);
        }
    }
}