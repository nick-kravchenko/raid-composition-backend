use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterRole {
    Tank,
    Heal,
    Melee,
    Ranged,
}

impl FromStr for CharacterRole {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tank" => Ok(Self::Tank),
            "heal" => Ok(Self::Heal),
            "melee" => Ok(Self::Melee),
            "ranged" => Ok(Self::Ranged),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterRank {
    Main,
    Alt,
}

impl FromStr for CharacterRank {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "main" => Ok(Self::Main),
            "alt" => Ok(Self::Alt),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CharacterResponseDto {
    pub character: CharacterDto,
}

#[derive(Debug, Serialize)]
pub struct CharactersResponseDto {
    pub characters: Vec<CharacterDto>,
}

#[derive(Debug, Serialize)]
pub struct CharacterDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub region: String,
    pub server: String,
    pub class: String,
    pub race: String,
    pub spec: String,
    pub role: CharacterRole,
    pub rank: CharacterRank,
    pub battlenet: Option<CharacterBattleNetSnapshotDto>,
    pub warcraftlogs: Option<CharacterWarcraftLogsSnapshotDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CharacterBattleNetSnapshotDto {
    pub battlenet_character_id: Option<i64>,
    pub race_id: Option<i32>,
    pub race_name: Option<String>,
    pub class_id: Option<i32>,
    pub class_name: Option<String>,
    pub gender: Option<String>,
    pub faction: Option<String>,
    pub level: Option<i32>,
    pub active_spec_id: Option<i32>,
    pub active_spec_name: Option<String>,
    pub average_item_level: Option<i32>,
    pub equipped_item_level: Option<i32>,
    pub avatar_url: Option<String>,
    pub main_raw_url: Option<String>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub raw: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CharacterWarcraftLogsSnapshotDto {
    pub warcraftlogs_character_id: Option<i64>,
    pub gear: Option<Value>,
    pub rankings: Option<Value>,
    pub best_performance_average: Option<f64>,
    pub median_performance_average: Option<f64>,
    pub total_kills: i32,
    pub fetched_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub partial: bool,
    pub raw: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_role_accepts_known_values() {
        assert_eq!(
            "tank".parse::<CharacterRole>().expect("role"),
            CharacterRole::Tank
        );
        assert_eq!(
            "heal".parse::<CharacterRole>().expect("role"),
            CharacterRole::Heal
        );
        assert_eq!(
            "melee".parse::<CharacterRole>().expect("role"),
            CharacterRole::Melee
        );
        assert_eq!(
            "ranged".parse::<CharacterRole>().expect("role"),
            CharacterRole::Ranged
        );
    }

    #[test]
    fn character_role_rejects_unknown_values() {
        assert!("support".parse::<CharacterRole>().is_err());
    }

    #[test]
    fn character_rank_accepts_known_values() {
        assert_eq!(
            "main".parse::<CharacterRank>().expect("rank"),
            CharacterRank::Main
        );
        assert_eq!(
            "alt".parse::<CharacterRank>().expect("rank"),
            CharacterRank::Alt
        );
    }

    #[test]
    fn character_rank_rejects_unknown_values() {
        assert!("bench".parse::<CharacterRank>().is_err());
    }
}
