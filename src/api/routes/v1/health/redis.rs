use std::time::Duration;

use actix_web::HttpResponse;
use serde_json::json;

use crate::config::Config;

pub async fn health() -> actix_web::Result<impl actix_web::Responder> {
  let config = crate::config::config();

  match actix_web::rt::time::timeout(Duration::from_secs(2), ping(config)).await {
    Ok(Ok(())) => Ok(HttpResponse::Ok().json(json!({
      "status": "ok",
      "service": "redis"
    }))),
    Ok(Err(error)) => Ok(HttpResponse::ServiceUnavailable().json(json!({
      "status": "unavailable",
      "service": "redis",
      "error": error.to_string()
    }))),
    Err(_) => Ok(HttpResponse::ServiceUnavailable().json(json!({
      "status": "unavailable",
      "service": "redis",
      "error": "Redis health check timed out"
    }))),
  }
}

async fn ping(config: Config) -> redis::RedisResult<()> {
  let client = redis::Client::open(redis_url(&config))?;
  let mut connection = client.get_multiplexed_async_connection().await?;
  let response = redis::cmd("PING").query_async::<String>(&mut connection).await?;

  if response == "PONG" {
    Ok(())
  } else {
    Err(redis::RedisError::from((
      redis::ErrorKind::UnexpectedReturnType,
      "Redis PING returned an unexpected response",
    )))
  }
}

fn redis_url(config: &Config) -> String {
  format!(
    "redis://:{}@{}:{}/",
    super::percent_encode_url_component(&config.redis_password),
    config.redis_host,
    config.redis_port
  )
}
