use crate::api::controllers::auth::{DiscordAuthUrlResponseDto, get_discord_auth_url};
use crate::state::AppState;
use actix_web::web;

pub async fn auth_url(
    state: web::Data<AppState>,
) -> actix_web::Result<web::Json<DiscordAuthUrlResponseDto>> {
    let response = get_discord_auth_url(state).await?;
    Ok(response)
}
