use actix_web::{HttpRequest, HttpResponse, Scope, web};
use uuid::Uuid;

use crate::{
    guilds::{
        dto::{CreateGuildRequestDto, UpdateGuildRequestDto},
        service,
    },
    state::AppState,
};

pub fn scope() -> Scope {
    web::scope("")
        .service(
            web::scope("/guilds")
                .route("", web::post().to(create))
                .route("", web::get().to(list))
                .route("/{guild_id}", web::get().to(get))
                .route("/{guild_id}", web::patch().to(update))
                .route("/{guild_id}", web::delete().to(delete))
                .route("/{guild_id}/invites", web::post().to(invites::create))
                .route("/{guild_id}/members", web::get().to(members::list))
                .route(
                    "/{guild_id}/members/{user_id}",
                    web::patch().to(members::update_role),
                ),
        )
        .service(
            web::scope("/guild-invites")
                .route("/{invite_code}/accept", web::post().to(invites::accept)),
        )
}

async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<CreateGuildRequestDto>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::create_guild_response(state, req, payload.into_inner()).await?)
}

async fn list(state: web::Data<AppState>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    Ok(service::list_guilds_response(state, req).await?)
}

async fn get(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::get_guild_response(state, req, guild_id.into_inner()).await?)
}

async fn update(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: web::Path<Uuid>,
    payload: web::Json<UpdateGuildRequestDto>,
) -> actix_web::Result<HttpResponse> {
    Ok(
        service::update_guild_response(state, req, guild_id.into_inner(), payload.into_inner())
            .await?,
    )
}

async fn delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::delete_guild_response(state, req, guild_id.into_inner()).await?)
}

mod invites;
mod members;
