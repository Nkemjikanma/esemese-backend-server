use crate::config::Config;
use sqlx::PgPool;
use aws_sdk_s3::Client;
pub struct AppState {
    pub app_config: Config,
    pub connection: PgPool,
    pub rustfs_client: Client,

}
