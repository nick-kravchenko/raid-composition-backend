use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuildRole {
    Applicant,
    Raider,
    Officer,
    Admin,
}

impl GuildRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applicant => "applicant",
            Self::Raider => "raider",
            Self::Officer => "officer",
            Self::Admin => "admin",
        }
    }

    pub fn is_allowed_promotion_target(self) -> bool {
        matches!(self, Self::Raider | Self::Officer)
    }
}

impl FromStr for GuildRole {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "applicant" => Ok(Self::Applicant),
            "raider" => Ok(Self::Raider),
            "officer" => Ok(Self::Officer),
            "admin" => Ok(Self::Admin),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameVersion {
    #[serde(rename = "classic1x")]
    Classic1x,
    #[serde(rename = "classic")]
    Classic,
    #[serde(rename = "classicann")]
    ClassicAnn,
}

impl FromStr for GameVersion {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "classic1x" => Ok(Self::Classic1x),
            "classic" => Ok(Self::Classic),
            "classicann" => Ok(Self::ClassicAnn),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGuildRequestDto {
    pub name: String,
    pub realm: String,
    pub region: String,
    pub faction: String,
    pub game_version: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuildRequestDto {
    pub name: Option<String>,
    pub realm: Option<String>,
    pub region: Option<String>,
    pub faction: Option<String>,
    pub game_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuildResponseDto {
    pub guild: GuildDto,
}

#[derive(Debug, Serialize)]
pub struct GuildsResponseDto {
    pub guilds: Vec<GuildDto>,
}

#[derive(Debug, Serialize)]
pub struct GuildDto {
    pub id: Uuid,
    pub name: String,
    pub realm: String,
    pub region: String,
    pub faction: String,
    pub game_version: GameVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub membership_role: GuildRole,
}

#[derive(Debug, Serialize)]
pub struct GuildInviteResponseDto {
    pub invite: GuildInviteDto,
}

#[derive(Debug, Serialize)]
pub struct GuildInviteDto {
    pub guild_id: Uuid,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct GuildMembersResponseDto {
    pub members: Vec<GuildMemberDto>,
}

#[derive(Debug, Serialize)]
pub struct GuildMemberDto {
    pub user_id: Uuid,
    pub role: GuildRole,
    pub discord: GuildMemberDiscordDto,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GuildMemberDiscordDto {
    pub id: String,
    pub username: Option<String>,
    pub global_name: Option<String>,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuildMemberRoleRequestDto {
    pub role: GuildRole,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_role_accepts_known_values() {
        assert_eq!(
            "admin".parse::<GuildRole>().expect("role"),
            GuildRole::Admin
        );
        assert_eq!(
            "officer".parse::<GuildRole>().expect("role"),
            GuildRole::Officer
        );
        assert_eq!(
            "raider".parse::<GuildRole>().expect("role"),
            GuildRole::Raider
        );
        assert_eq!(
            "applicant".parse::<GuildRole>().expect("role"),
            GuildRole::Applicant
        );
    }

    #[test]
    fn guild_role_rejects_unknown_values() {
        assert!("owner".parse::<GuildRole>().is_err());
    }

    #[test]
    fn promotion_targets_are_limited_to_raider_and_officer() {
        assert!(GuildRole::Raider.is_allowed_promotion_target());
        assert!(GuildRole::Officer.is_allowed_promotion_target());
        assert!(!GuildRole::Applicant.is_allowed_promotion_target());
        assert!(!GuildRole::Admin.is_allowed_promotion_target());
    }

    #[test]
    fn game_version_accepts_known_values() {
        assert_eq!(
            "classic1x".parse::<GameVersion>().expect("game version"),
            GameVersion::Classic1x
        );
        assert_eq!(
            "classic".parse::<GameVersion>().expect("game version"),
            GameVersion::Classic
        );
        assert_eq!(
            "classicann".parse::<GameVersion>().expect("game version"),
            GameVersion::ClassicAnn
        );
    }

    #[test]
    fn game_version_rejects_unknown_values() {
        assert!("retail".parse::<GameVersion>().is_err());
    }
}
