use actix_web::{HttpRequest, HttpResponse, web};
use uuid::Uuid;

use crate::{
    guilds::{dto::UpdateGuildMemberRoleRequestDto, service},
    state::AppState,
};

pub async fn list(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    Ok(service::list_members_response(state, req, guild_id.into_inner()).await?)
}

pub async fn update_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    payload: web::Json<UpdateGuildMemberRoleRequestDto>,
) -> actix_web::Result<HttpResponse> {
    let (guild_id, user_id) = path.into_inner();
    Ok(
        service::update_member_role_response(state, req, guild_id, user_id, payload.into_inner())
            .await?,
    )
}
