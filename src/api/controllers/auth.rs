use actix_web::web;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct DiscordAuthUrlResponseDto {
    pub url: String,
}

pub async fn get_discord_auth_url(
    state: web::Data<AppState>,
) -> Result<web::Json<DiscordAuthUrlResponseDto>, actix_web::Error> {
    let discord_auth_url = state
        .config
        .discord
        .authorization_url(&state.config.frontend.base_url);

    Ok(web::Json(DiscordAuthUrlResponseDto {
        url: discord_auth_url,
    }))
}
