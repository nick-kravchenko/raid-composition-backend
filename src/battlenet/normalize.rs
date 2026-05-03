use serde_json::{Value, json};

use crate::battlenet::dto::BattleNetCharacterSummary;

pub fn normalize_character(
    profile: &Value,
    media: Option<&Value>,
    fallback_realm_slug: &str,
    region: &str,
) -> BattleNetCharacterSummary {
    BattleNetCharacterSummary {
        character_name: string_at(profile, &["name"]).unwrap_or_default(),
        realm_slug: string_at(profile, &["realm", "slug"])
            .unwrap_or_else(|| fallback_realm_slug.to_string()),
        region: region.to_string(),
        battlenet_character_id: i64_at(profile, &["id"]),
        race_id: i32_at(profile, &["race", "id"]),
        race_name: string_at(profile, &["race", "name"]),
        class_id: i32_at(profile, &["character_class", "id"]),
        class_name: string_at(profile, &["character_class", "name"]),
        gender: typed_value(profile, &["gender"]),
        faction: typed_value(profile, &["faction"]),
        level: i32_at(profile, &["level"]),
        active_spec_id: i32_at(profile, &["active_spec", "id"]),
        active_spec_name: string_at(profile, &["active_spec", "name"]),
        average_item_level: i32_at(profile, &["average_item_level"]),
        equipped_item_level: i32_at(profile, &["equipped_item_level"]),
        avatar_url: media.and_then(|media| media_asset_url(media, "avatar")),
        main_raw_url: media.and_then(|media| media_asset_url(media, "main-raw")),
        raw: json!({
            "profile": profile,
            "media": media,
        }),
    }
}

fn typed_value(profile: &Value, path: &[&str]) -> Option<String> {
    string_at(nested_value(profile, path)?, &["type"]).map(|value| value.to_ascii_lowercase())
}

fn media_asset_url(media: &Value, key: &str) -> Option<String> {
    media
        .get("assets")
        .and_then(Value::as_array)?
        .iter()
        .find(|asset| asset.get("key").and_then(Value::as_str) == Some(key))
        .and_then(|asset| asset.get("value").and_then(Value::as_str))
        .map(ToString::to_string)
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    nested_value(value, path)?.as_str().map(ToString::to_string)
}

fn i64_at(value: &Value, path: &[&str]) -> Option<i64> {
    nested_value(value, path)?.as_i64()
}

fn i32_at(value: &Value, path: &[&str]) -> Option<i32> {
    nested_value(value, path)?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

fn nested_value<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(key))
}
