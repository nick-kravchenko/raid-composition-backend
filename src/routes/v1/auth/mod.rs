use actix_web::{web, HttpResponse, Scope};

pub fn scope() -> Scope {
  web::scope("/auth")
    .service(
      web::scope("")
        .route("/me", web::get().to(me))
        .route("/refresh", web::post().to(session::refresh))
        .route("/logout", web::post().to(session::logout))
    )
}

async fn me() -> actix_web::Result<impl actix_web::Responder> {
  Ok(HttpResponse::Ok().json(serde_json::json!({
    "message": "User info endpoint."
  })))
}

pub mod session;
pub mod discord;