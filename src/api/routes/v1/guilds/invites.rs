use actix_web::{HttpRequest, HttpResponse, web};
use uuid::Uuid;

use crate::{guilds::service, state::AppState};

pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::create_invite_response(state, req, guild_id.into_inner()).await?)
}

pub async fn accept(
    state: web::Data<AppState>,
    req: HttpRequest,
    invite_code: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::accept_invite_response(state, req, invite_code.into_inner()).await?)
}
