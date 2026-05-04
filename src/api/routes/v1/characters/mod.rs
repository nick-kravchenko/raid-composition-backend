use actix_web::{HttpRequest, HttpResponse, Scope, web};
use uuid::Uuid;

use crate::{battlenet, characters, state::AppState, warcraftlogs};

pub fn scope() -> Scope {
    web::scope("/characters")
        .route("/{character_id}", web::get().to(get))
        .route(
            "/{character_id}/warcraftlogs/refresh",
            web::post().to(refresh_warcraftlogs),
        )
        .route(
            "/{character_id}/battlenet/refresh",
            web::post().to(refresh_battlenet),
        )
}

async fn get(
    state: web::Data<AppState>,
    req: HttpRequest,
    character_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(characters::service::get_character_response(state, req, character_id.into_inner()).await?)
}

async fn refresh_warcraftlogs(
    state: web::Data<AppState>,
    req: HttpRequest,
    character_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(
        warcraftlogs::service::refresh_character_warcraftlogs_response(
            state,
            req,
            character_id.into_inner(),
        )
        .await?,
    )
}

async fn refresh_battlenet(
    state: web::Data<AppState>,
    req: HttpRequest,
    character_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(battlenet::service::refresh_character_battlenet_response(
        state,
        req,
        character_id.into_inner(),
    )
    .await?)
}
