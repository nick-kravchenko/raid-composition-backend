use actix_web::{web, Scope};

pub fn scope() -> Scope {
  web::scope("/v1")
    .service(auth::scope())
}

pub mod auth;