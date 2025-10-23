use actix_web::{HttpRequest, HttpResponse};

pub async fn refresh(_req: HttpRequest) -> actix_web::Result<impl actix_web::Responder> {
  Ok(HttpResponse::Ok().json(serde_json::json!({
    "accessToken": "new.jwt.token.here",
    "expiresIn": 3600,
  })))
}

pub async fn logout(_req: HttpRequest) -> actix_web::Result<impl actix_web::Responder> {
  Ok(HttpResponse::NoContent().finish())
}
