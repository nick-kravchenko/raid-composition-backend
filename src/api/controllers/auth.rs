use actix_web::web;
use serde::Serialize;

#[derive(Serialize)]
pub struct DiscordAuthUrlResponseDto {
  pub url: String,
}

pub async fn get_discord_auth_url() -> Result<web::Json<DiscordAuthUrlResponseDto>, actix_web::Error> {
  let config = crate::config::config();
  let discord_client_id: String = config.discord_client_id;
  let redirect_url: String = config.frontend_base_url;
  let scope: &str = "identify";
  let response_type: &str = "code";
  let discord_auth_url: String = format!(
    "https://discord.com/oauth2/authorize?client_id={}&response_type={}&redirect_uri={}&scope={}",
    discord_client_id,
    response_type,
    redirect_url,
    scope
  );
  Ok(web::Json(DiscordAuthUrlResponseDto {
    url: discord_auth_url,
  }))
}