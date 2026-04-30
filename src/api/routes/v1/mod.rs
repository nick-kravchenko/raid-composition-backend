use actix_web::{web, Scope};

pub fn scope() -> Scope {
  web::scope("/v1")
    .service(auth::scope())
    .service(health::scope())
}

pub mod auth;
pub mod health;
