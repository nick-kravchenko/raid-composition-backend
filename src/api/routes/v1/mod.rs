use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/v1")
        .service(auth::scope())
        .service(characters::scope())
        .service(guilds::scope())
        .service(guilds::invite_scope())
        .service(health::scope())
}

pub mod auth;
pub mod characters;
pub mod guilds;
pub mod health;
