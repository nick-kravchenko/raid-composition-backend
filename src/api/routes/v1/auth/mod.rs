use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/auth")
        .service(
            web::scope("/discord")
                .route("/url", web::get().to(discord::auth_url))
                .route("/callback", web::post().to(discord::callback)),
        )
        .service(
            web::scope("")
                .route("/session", web::get().to(session::current))
                .route("/sessions", web::get().to(session::list))
                .route("/logout", web::post().to(session::logout))
                .route(
                    "/logout-all-other-sessions",
                    web::post().to(session::logout_all_other_sessions),
                )
                .route("/sessions/{session_id}", web::delete().to(session::revoke))
                .route("/csrf", web::get().to(session::csrf)),
        )
}

mod discord;
mod session;
