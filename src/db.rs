use actix_web::Result;
use sqlx::PgPool;

use crate::config::DatabaseConfig;

pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, actix_web::Error> {
    let pool = PgPool::connect(&config.url()).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to connect to database: {}", e))
    })?;

    Ok(pool)
}
