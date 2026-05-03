use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BattleNetCharacterSummary {
    pub character_name: String,
    pub realm_slug: String,
    pub region: String,
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
    pub raw: Value,
}
