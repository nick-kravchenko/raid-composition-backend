use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/health")
        .route("", web::get().to(app::health))
        .route("/postgres", web::get().to(postgres::health))
        .route("/redis", web::get().to(redis::health))
}

mod app;
mod postgres;
mod redis;
