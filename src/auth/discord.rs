use chrono::{DateTime, Duration, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{api::error::ApiError, config::DiscordConfig};

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: String,
    scope: String,
}

#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiscordUserProfile {
    pub id: String,
    pub username: Option<String>,
    pub discriminator: Option<String>,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
    pub system: Option<bool>,
    pub mfa_enabled: Option<bool>,
    pub banner: Option<String>,
    pub accent_color: Option<i32>,
    pub locale: Option<String>,
    pub verified: Option<bool>,
    pub flags: Option<i32>,
    pub premium_type: Option<i32>,
    pub public_flags: Option<i32>,
    pub avatar_decoration_data: Option<Value>,
    pub collectibles: Option<Value>,
    pub primary_guild: Option<Value>,
}

#[derive(Clone)]
pub struct DiscordClient {
    http: reqwest::Client,
}

impl DiscordClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn exchange_code(
        &self,
        config: &DiscordConfig,
        code: &str,
    ) -> Result<OAuthTokens, ApiError> {
        let response = self
            .http
            .post("https://discord.com/api/oauth2/token")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", config.redirect_url.as_str()),
            ])
            .send()
            .await
            .map_err(|error| {
                eprintln!("discord token exchange request failed: {error}");
                ApiError::bad_gateway(
                    "auth.discord_code_exchange_failed",
                    "Discord code exchange failed.",
                )
            })?;

        if response.status() != StatusCode::OK {
            eprintln!("discord token exchange returned {}", response.status());
            return Err(ApiError::bad_gateway(
                "auth.discord_code_exchange_failed",
                "Discord code exchange failed.",
            ));
        }

        let token_response = response
            .json::<DiscordTokenResponse>()
            .await
            .map_err(|error| {
                eprintln!("discord token exchange response decode failed: {error}");
                ApiError::bad_gateway(
                    "auth.discord_code_exchange_failed",
                    "Discord code exchange failed.",
                )
            })?;

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            scope: token_response.scope,
            expires_at: Utc::now() + Duration::seconds(token_response.expires_in),
        })
    }

    pub async fn fetch_user_profile(
        &self,
        tokens: &OAuthTokens,
    ) -> Result<DiscordUserProfile, ApiError> {
        let response = self
            .http
            .get("https://discord.com/api/users/@me")
            .bearer_auth(&tokens.access_token)
            .send()
            .await
            .map_err(|error| {
                eprintln!("discord profile request failed: {error}");
                ApiError::bad_gateway(
                    "auth.discord_profile_fetch_failed",
                    "Discord profile fetch failed.",
                )
            })?;

        if response.status() != StatusCode::OK {
            eprintln!("discord profile fetch returned {}", response.status());
            return Err(ApiError::bad_gateway(
                "auth.discord_profile_fetch_failed",
                "Discord profile fetch failed.",
            ));
        }

        response
            .json::<DiscordUserProfile>()
            .await
            .map_err(|error| {
                eprintln!("discord profile response decode failed: {error}");
                ApiError::bad_gateway(
                    "auth.discord_profile_fetch_failed",
                    "Discord profile fetch failed.",
                )
            })
    }
}

pub fn avatar_url(
    discord_user_id: &str,
    avatar: Option<&str>,
    discriminator: Option<&str>,
) -> String {
    if let Some(avatar) = avatar {
        let extension = if avatar.starts_with("a_") {
            "gif"
        } else {
            "png"
        };
        return format!(
            "https://cdn.discordapp.com/avatars/{discord_user_id}/{avatar}.{extension}?size=128"
        );
    }

    let default_index = default_avatar_index(discord_user_id, discriminator);
    format!("https://cdn.discordapp.com/embed/avatars/{default_index}.png")
}

fn default_avatar_index(discord_user_id: &str, discriminator: Option<&str>) -> u64 {
    if let Some(discriminator) = discriminator.and_then(|value| value.parse::<u64>().ok())
        && discriminator > 0
    {
        return discriminator % 5;
    }

    discord_user_id
        .parse::<u64>()
        .map(|id| (id >> 22) % 6)
        .unwrap_or(0)
}
