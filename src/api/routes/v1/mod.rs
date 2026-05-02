use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/v1")
        .service(auth::scope())
        .service(guilds::scope())
        .service(health::scope())
}

pub mod auth;
pub mod guilds;
pub mod health;
