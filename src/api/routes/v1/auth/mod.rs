use actix_web::{HttpResponse, Scope, web};

pub fn scope() -> Scope {
    web::scope("/auth")
        .service(web::scope("/discord").route("/url", web::get().to(discord::auth_url)))
        .service(
            web::scope("")
                .route("/me", web::get().to(me))
                .route("/logout", web::post().to(session::logout)),
        )
}

async fn me() -> actix_web::Result<impl actix_web::Responder> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
      "message": "User info endpoint."
    })))
}

mod discord;
mod session;
