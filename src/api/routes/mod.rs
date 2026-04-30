use actix_web::{Scope, web};

pub fn api_v1() -> Scope {
    web::scope("/api").service(v1::scope())
}

pub mod v1;
