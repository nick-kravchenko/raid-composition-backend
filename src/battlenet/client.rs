use std::{error::Error, fmt};

use base64::{Engine as _, engine::general_purpose};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use crate::{config::BattleNetConfig, config::percent_encode_url_component};

#[derive(Debug)]
pub enum BattleNetError {
    HttpStatus { status: StatusCode, body: String },
    Http(reqwest::Error),
    InvalidResponse(serde_json::Error),
}

impl fmt::Display for BattleNetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpStatus { status, body } => {
                write!(formatter, "Battle.net HTTP {status}: {body}")
            }
            Self::Http(error) => write!(formatter, "Battle.net HTTP client error: {error}"),
            Self::InvalidResponse(error) => {
                write!(formatter, "Battle.net response was invalid: {error}")
            }
        }
    }
}

impl Error for BattleNetError {}

impl From<reqwest::Error> for BattleNetError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for BattleNetError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidResponse(error)
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub struct BattleNetClient<'a> {
    http_client: &'a Client,
    config: &'a BattleNetConfig,
}

impl<'a> BattleNetClient<'a> {
    pub fn new(http_client: &'a Client, config: &'a BattleNetConfig) -> Self {
        Self {
            http_client,
            config,
        }
    }

    pub async fn access_token(&self, region: &str) -> Result<String, BattleNetError> {
        let response = self
            .http_client
            .post(build_oauth_token_url(region))
            .header(
                reqwest::header::AUTHORIZATION,
                build_battlenet_basic_auth_value(
                    &self.config.client_id,
                    &self.config.client_secret,
                ),
            )
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BattleNetError::HttpStatus {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        let token = response.json::<TokenResponse>().await?;
        Ok(token.access_token)
    }

    pub async fn character(
        &self,
        token: &str,
        region: &str,
        realm_slug: &str,
        character_name: &str,
    ) -> Result<Option<Value>, BattleNetError> {
        self.get_json_or_none(
            token,
            &build_character_url(
                region,
                &self.config.game_version,
                realm_slug,
                character_name,
            ),
        )
        .await
    }

    pub async fn character_media(
        &self,
        token: &str,
        region: &str,
        realm_slug: &str,
        character_name: &str,
    ) -> Result<Option<Value>, BattleNetError> {
        self.get_json_or_none(
            token,
            &build_character_media_url(
                region,
                &self.config.game_version,
                realm_slug,
                character_name,
            ),
        )
        .await
    }

    async fn get_json_or_none(
        &self,
        token: &str,
        url: &str,
    ) -> Result<Option<Value>, BattleNetError> {
        let response = self.http_client.get(url).bearer_auth(token).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(BattleNetError::HttpStatus {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        Ok(Some(response.json::<Value>().await?))
    }
}

pub fn build_oauth_token_url(region: &str) -> String {
    format!(
        "https://{}.battle.net/oauth/token",
        region.trim().to_ascii_lowercase()
    )
}

pub fn build_battlenet_basic_auth_value(client_id: &str, client_secret: &str) -> String {
    let credentials = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
    format!("Basic {credentials}")
}

pub fn profile_namespace(region: &str, game_version: &str) -> String {
    let region = region.trim().to_ascii_lowercase();
    match game_version {
        "retail" => format!("profile-{region}"),
        "classic1x" => format!("profile-classic1x-{region}"),
        _ => format!("profile-classicann-{region}"),
    }
}

pub fn build_character_url(
    region: &str,
    game_version: &str,
    realm_slug: &str,
    character_name: &str,
) -> String {
    let region = region.trim().to_ascii_lowercase();
    let namespace = profile_namespace(&region, game_version);
    let realm = percent_encode_url_component(&realm_slug.trim().to_ascii_lowercase());
    let name = percent_encode_url_component(&character_name.trim().to_lowercase());

    format!(
        "https://{region}.api.blizzard.com/profile/wow/character/{realm}/{name}?namespace={namespace}&locale=en_GB"
    )
}

pub fn build_character_media_url(
    region: &str,
    game_version: &str,
    realm_slug: &str,
    character_name: &str,
) -> String {
    let region = region.trim().to_ascii_lowercase();
    let namespace = profile_namespace(&region, game_version);
    let realm = percent_encode_url_component(&realm_slug.trim().to_ascii_lowercase());
    let name = percent_encode_url_component(&character_name.trim().to_lowercase());

    format!(
        "https://{region}.api.blizzard.com/profile/wow/character/{realm}/{name}/character-media?namespace={namespace}&locale=en_US"
    )
}
