use actix_web::HttpResponse;
use serde_json::json;

pub async fn health() -> actix_web::Result<impl actix_web::Responder> {
  Ok(HttpResponse::Ok().json(json!({
    "status": "ok",
    "service": "app"
  })))
}
