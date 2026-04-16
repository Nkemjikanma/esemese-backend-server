use crate::config::Config;
use sqlx::PgPool;

pub struct AppState {
    pub app_config: Config,
    pub connection: PgPool,
}
