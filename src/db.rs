use actix_web::Result;
use sqlx::PgPool;
use crate::config::config;

pub async fn connect() -> Result<PgPool, actix_web::Error> {
  let config = config();

  let database_url = format!(
    "postgres://{}:{}@{}:{}/{}",
    config.db_user,
    config.db_password,
    config.db_host,
    config.db_port,
    config.db_name
  );

  let pool = PgPool::connect(&database_url).await.map_err(|e| {
    actix_web::error::ErrorInternalServerError(format!("Failed to connect to database: {}", e))
  })?;

  Ok(pool)
}