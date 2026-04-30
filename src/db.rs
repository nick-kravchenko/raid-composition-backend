use sqlx::{PgPool, migrate::MigrateError};

use crate::config::DatabaseConfig;

pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(&config.url()).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
