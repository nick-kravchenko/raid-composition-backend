use actix_web::web;
use crate::api::controllers::auth::{get_discord_auth_url, DiscordAuthUrlResponseDto};

pub async fn auth_url() -> actix_web::Result<web::Json<DiscordAuthUrlResponseDto>> {
  let response = get_discord_auth_url().await?;
  Ok(response)
}