use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::service as auth_service,
    battlenet::{
        client::{BattleNetClient, BattleNetError},
        dto::BattleNetCharacterSummary,
        normalize::normalize_character,
    },
    state::AppState,
};

#[derive(Debug, FromRow)]
struct CharacterRow {
    id: Uuid,
    name: String,
    region: String,
    server: String,
}

#[derive(Debug, FromRow)]
struct SnapshotRow {
    character_id: Uuid,
    battlenet_character_id: Option<i64>,
    race_id: Option<i32>,
    race_name: Option<String>,
    class_id: Option<i32>,
    class_name: Option<String>,
    gender: Option<String>,
    faction: Option<String>,
    level: Option<i32>,
    active_spec_id: Option<i32>,
    active_spec_name: Option<String>,
    average_item_level: Option<i32>,
    equipped_item_level: Option<i32>,
    avatar_url: Option<String>,
    main_raw_url: Option<String>,
    fetched_at: Option<DateTime<Utc>>,
    error: Option<String>,
    raw: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct SnapshotDraft {
    pub character_id: Uuid,
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
    pub error: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Serialize)]
pub struct BattleNetSnapshotResponseDto {
    pub snapshot: BattleNetSnapshotDto,
}

#[derive(Debug, Serialize)]
pub struct BattleNetSnapshotDto {
    pub character_id: Uuid,
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
}

pub async fn refresh_character_battlenet_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    character_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;

    let snapshot = refresh_character_snapshot(&state, context.user_id, character_id).await?;

    let mut response = HttpResponse::Ok();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response.json(BattleNetSnapshotResponseDto {
        snapshot: snapshot_row_to_dto(snapshot),
    }))
}

async fn refresh_character_snapshot(
    state: &AppState,
    user_id: Uuid,
    character_id: Uuid,
) -> Result<SnapshotRow, ApiError> {
    let character = load_user_character(&state.db_pool, user_id, character_id).await?;
    let client = BattleNetClient::new(&state.http_client, &state.config.battlenet);
    let realm_slug = character.server.trim().to_ascii_lowercase();
    let region = character.region.trim().to_ascii_lowercase();
    let token = client
        .access_token(&region)
        .await
        .map_err(battlenet_bad_gateway)?;

    let Some(profile) = client
        .character(&token, &region, &realm_slug, &character.name)
        .await
        .map_err(battlenet_bad_gateway)?
    else {
        let snapshot = build_error_snapshot(
            character.id,
            &character.name,
            &realm_slug,
            &region,
            &format!("Character not found on {realm_slug}-{region}"),
        );
        return upsert_snapshot(&state.db_pool, snapshot).await;
    };

    let media = client
        .character_media(&token, &region, &realm_slug, &character.name)
        .await
        .map_err(battlenet_bad_gateway)?;
    let summary = normalize_character(&profile, media.as_ref(), &realm_slug, &region);

    upsert_snapshot(&state.db_pool, build_snapshot(character.id, summary)).await
}

async fn load_user_character(
    db_pool: &PgPool,
    user_id: Uuid,
    character_id: Uuid,
) -> Result<CharacterRow, ApiError> {
    sqlx::query_as::<_, CharacterRow>(
        r#"
        select id, name, region, server
        from characters
        where id = $1
          and user_id = $2
          and deleted_at is null
        "#,
    )
    .bind(character_id)
    .bind(user_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| ApiError::not_found("character.not_found", "Character was not found."))
}

pub fn build_snapshot(character_id: Uuid, summary: BattleNetCharacterSummary) -> SnapshotDraft {
    SnapshotDraft {
        character_id,
        battlenet_character_id: summary.battlenet_character_id,
        race_id: summary.race_id,
        race_name: summary.race_name,
        class_id: summary.class_id,
        class_name: summary.class_name,
        gender: summary.gender,
        faction: summary.faction,
        level: summary.level,
        active_spec_id: summary.active_spec_id,
        active_spec_name: summary.active_spec_name,
        average_item_level: summary.average_item_level,
        equipped_item_level: summary.equipped_item_level,
        avatar_url: summary.avatar_url,
        main_raw_url: summary.main_raw_url,
        error: None,
        raw: summary.raw,
    }
}

pub fn build_error_snapshot(
    character_id: Uuid,
    character_name: &str,
    realm_slug: &str,
    region: &str,
    error: &str,
) -> SnapshotDraft {
    SnapshotDraft {
        character_id,
        battlenet_character_id: None,
        race_id: None,
        race_name: None,
        class_id: None,
        class_name: None,
        gender: None,
        faction: None,
        level: None,
        active_spec_id: None,
        active_spec_name: None,
        average_item_level: None,
        equipped_item_level: None,
        avatar_url: None,
        main_raw_url: None,
        error: Some(error.to_string()),
        raw: json!({
            "characterName": character_name,
            "realmSlug": realm_slug,
            "region": region,
            "error": error,
        }),
    }
}

async fn upsert_snapshot(
    db_pool: &PgPool,
    snapshot: SnapshotDraft,
) -> Result<SnapshotRow, ApiError> {
    sqlx::query_as::<_, SnapshotRow>(
        r#"
        insert into character_battlenet_snapshots (
            character_id,
            battlenet_character_id,
            race_id,
            race_name,
            class_id,
            class_name,
            gender,
            faction,
            level,
            active_spec_id,
            active_spec_name,
            average_item_level,
            equipped_item_level,
            avatar_url,
            main_raw_url,
            fetched_at,
            error,
            raw
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, now(), $16, $17)
        on conflict (character_id) do update
        set battlenet_character_id = excluded.battlenet_character_id,
            race_id = excluded.race_id,
            race_name = excluded.race_name,
            class_id = excluded.class_id,
            class_name = excluded.class_name,
            gender = excluded.gender,
            faction = excluded.faction,
            level = excluded.level,
            active_spec_id = excluded.active_spec_id,
            active_spec_name = excluded.active_spec_name,
            average_item_level = excluded.average_item_level,
            equipped_item_level = excluded.equipped_item_level,
            avatar_url = excluded.avatar_url,
            main_raw_url = excluded.main_raw_url,
            fetched_at = excluded.fetched_at,
            error = excluded.error,
            raw = excluded.raw
        returning character_id,
                  battlenet_character_id,
                  race_id,
                  race_name,
                  class_id,
                  class_name,
                  gender,
                  faction,
                  level,
                  active_spec_id,
                  active_spec_name,
                  average_item_level,
                  equipped_item_level,
                  avatar_url,
                  main_raw_url,
                  fetched_at,
                  error,
                  raw
        "#,
    )
    .bind(snapshot.character_id)
    .bind(snapshot.battlenet_character_id)
    .bind(snapshot.race_id)
    .bind(snapshot.race_name)
    .bind(snapshot.class_id)
    .bind(snapshot.class_name)
    .bind(snapshot.gender)
    .bind(snapshot.faction)
    .bind(snapshot.level)
    .bind(snapshot.active_spec_id)
    .bind(snapshot.active_spec_name)
    .bind(snapshot.average_item_level)
    .bind(snapshot.equipped_item_level)
    .bind(snapshot.avatar_url)
    .bind(snapshot.main_raw_url)
    .bind(snapshot.error)
    .bind(snapshot.raw)
    .fetch_one(db_pool)
    .await
    .map_err(ApiError::from)
}

fn snapshot_row_to_dto(row: SnapshotRow) -> BattleNetSnapshotDto {
    BattleNetSnapshotDto {
        character_id: row.character_id,
        battlenet_character_id: row.battlenet_character_id,
        race_id: row.race_id,
        race_name: row.race_name,
        class_id: row.class_id,
        class_name: row.class_name,
        gender: row.gender,
        faction: row.faction,
        level: row.level,
        active_spec_id: row.active_spec_id,
        active_spec_name: row.active_spec_name,
        average_item_level: row.average_item_level,
        equipped_item_level: row.equipped_item_level,
        avatar_url: row.avatar_url,
        main_raw_url: row.main_raw_url,
        fetched_at: row.fetched_at,
        error: row.error,
        raw: row.raw,
    }
}

fn battlenet_bad_gateway(error: BattleNetError) -> ApiError {
    eprintln!("battle.net error: {error}");
    ApiError::bad_gateway("battlenet.upstream_failed", "Battle.net request failed.")
}
