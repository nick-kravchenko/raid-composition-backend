use crate::auth::service::{
    DiscordAuthUrlResponseDto, DiscordCallbackRequestDto, get_discord_auth_url,
    handle_discord_callback,
};
use crate::state::AppState;
use actix_web::{HttpRequest, HttpResponse, web};

pub async fn auth_url(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<web::Json<DiscordAuthUrlResponseDto>> {
    let response = get_discord_auth_url(state, req).await?;
    Ok(response)
}

pub async fn callback(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<DiscordCallbackRequestDto>,
) -> actix_web::Result<HttpResponse> {
    Ok(handle_discord_callback(state, req, payload.into_inner()).await?)
}
