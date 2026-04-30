use redis::Client;
use sqlx::PgPool;

use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub db_pool: PgPool,
    pub redis_client: Client,
}
