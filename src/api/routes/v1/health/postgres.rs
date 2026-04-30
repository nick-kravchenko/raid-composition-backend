use std::time::Duration;

use actix_web::HttpResponse;
use serde_json::json;
use sqlx::PgPool;

use crate::config::Config;

pub async fn health() -> actix_web::Result<impl actix_web::Responder> {
  let config = crate::config::config();

  match actix_web::rt::time::timeout(Duration::from_secs(2), ping(config)).await {
    Ok(Ok(())) => Ok(HttpResponse::Ok().json(json!({
      "status": "ok",
      "service": "postgres"
    }))),
    Ok(Err(error)) => Ok(HttpResponse::ServiceUnavailable().json(json!({
      "status": "unavailable",
      "service": "postgres",
      "error": error.to_string()
    }))),
    Err(_) => Ok(HttpResponse::ServiceUnavailable().json(json!({
      "status": "unavailable",
      "service": "postgres",
      "error": "PostgreSQL health check timed out"
    }))),
  }
}

async fn ping(config: Config) -> sqlx::Result<()> {
  let pool = PgPool::connect(&postgres_url(&config)).await?;
  sqlx::query("SELECT 1").execute(&pool).await?;
  pool.close().await;

  Ok(())
}

fn postgres_url(config: &Config) -> String {
  format!(
    "postgres://{}:{}@{}:{}/{}",
    super::percent_encode_url_component(&config.db_user),
    super::percent_encode_url_component(&config.db_password),
    config.db_host,
    config.db_port,
    config.db_name
  )
}
