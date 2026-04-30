use std::time::Duration;

use actix_web::{HttpResponse, web};
use serde_json::json;
use sqlx::PgPool;

use crate::state::AppState;

pub async fn health(state: web::Data<AppState>) -> actix_web::Result<impl actix_web::Responder> {
    match actix_web::rt::time::timeout(Duration::from_secs(2), ping(state.db_pool.clone())).await {
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

async fn ping(pool: PgPool) -> sqlx::Result<()> {
    sqlx::query("SELECT 1").execute(&pool).await?;

    Ok(())
}
