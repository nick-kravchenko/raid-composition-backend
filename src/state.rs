use redis::Client;
use sqlx::PgPool;

use crate::auth::geoip::GeoIp;
use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub db_pool: PgPool,
    pub redis_client: Client,
    pub http_client: reqwest::Client,
    pub geoip: GeoIp,
}
