use actix_web::{web, Scope};

pub fn api_v1() -> Scope {
  web::scope("/api")
    .service(v1::scope())
}

pub mod v1;
