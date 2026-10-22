use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::service as auth_service,
    characters::dto::{
        CharacterBattleNetSnapshotDto, CharacterDto, CharacterRank, CharacterResponseDto,
        CharacterRole, CharacterWarcraftLogsSnapshotDto, CharactersResponseDto,
    },
    state::AppState,
};

#[derive(Debug, FromRow)]
struct CharacterRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    region: String,
    server: String,
    class: String,
    race: String,
    spec: String,
    role: String,
    rank: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    battlenet_snapshot_character_id: Option<Uuid>,
    battlenet_character_id: Option<i64>,
    battlenet_race_id: Option<i32>,
    battlenet_race_name: Option<String>,
    battlenet_class_id: Option<i32>,
    battlenet_class_name: Option<String>,
    battlenet_gender: Option<String>,
    battlenet_faction: Option<String>,
    battlenet_level: Option<i32>,
    battlenet_active_spec_id: Option<i32>,
    battlenet_active_spec_name: Option<String>,
    battlenet_average_item_level: Option<i32>,
    battlenet_equipped_item_level: Option<i32>,
    battlenet_avatar_url: Option<String>,
    battlenet_main_raw_url: Option<String>,
    battlenet_fetched_at: Option<DateTime<Utc>>,
    battlenet_error: Option<String>,
    battlenet_raw: Option<Value>,
    battlenet_created_at: Option<DateTime<Utc>>,
    battlenet_updated_at: Option<DateTime<Utc>>,
    warcraftlogs_snapshot_character_id: Option<Uuid>,
    warcraftlogs_character_id: Option<i64>,
    warcraftlogs_gear: Option<Value>,
    warcraftlogs_rankings: Option<Value>,
    warcraftlogs_best_performance_average: Option<f64>,
    warcraftlogs_median_performance_average: Option<f64>,
    warcraftlogs_total_kills: Option<i32>,
    warcraftlogs_fetched_at: Option<DateTime<Utc>>,
    warcraftlogs_error: Option<String>,
    warcraftlogs_partial: Option<bool>,
    warcraftlogs_raw: Option<Value>,
    warcraftlogs_created_at: Option<DateTime<Utc>>,
    warcraftlogs_updated_at: Option<DateTime<Utc>>,
}

pub async fn list_guild_characters_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    require_guild_membership(&state.db_pool, guild_id, context.user_id).await?;

    let characters = sqlx::query_as::<_, CharacterRow>(sqlx::AssertSqlSafe(format!(
        r#"
        select {}
        from characters c
        join guild_memberships gm on gm.user_id = c.user_id
        left join character_battlenet_snapshots b on b.character_id = c.id
        left join character_warcraftlogs_snapshots w on w.character_id = c.id
        where gm.guild_id = $1
          and gm.deleted_at is null
          and c.deleted_at is null
        order by gm.created_at asc, c.created_at asc
        "#,
        select_character_columns()
    )))
    .bind(guild_id)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        CharactersResponseDto {
            characters: characters
                .into_iter()
                .map(character_row_to_dto)
                .collect::<Result<Vec<_>, _>>()?,
        },
    ))
}

pub async fn get_character_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    character_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;

    let character = sqlx::query_as::<_, CharacterRow>(sqlx::AssertSqlSafe(format!(
        r#"
        select {}
        from characters c
        left join character_battlenet_snapshots b on b.character_id = c.id
        left join character_warcraftlogs_snapshots w on w.character_id = c.id
        where c.id = $1
          and c.deleted_at is null
          and (
            c.user_id = $2
            or exists (
                select 1
                from guild_memberships viewer
                join guild_memberships owner on owner.guild_id = viewer.guild_id
                join guilds g on g.id = viewer.guild_id
                where viewer.user_id = $2
                  and owner.user_id = c.user_id
                  and viewer.deleted_at is null
                  and owner.deleted_at is null
                  and g.deleted_at is null
            )
          )
        "#,
        select_character_columns()
    )))
    .bind(character_id)
    .bind(context.user_id)
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(character_not_found)?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        CharacterResponseDto {
            character: character_row_to_dto(character)?,
        },
    ))
}

async fn require_guild_membership(
    pool: &PgPool,
    guild_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let membership = sqlx::query_scalar::<_, i32>(
        r#"
        select 1
        from guild_memberships gm
        join guilds g on g.id = gm.guild_id
        where gm.guild_id = $1
          and gm.user_id = $2
          and gm.deleted_at is null
          and g.deleted_at is null
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if membership.is_none() {
        return Err(ApiError::not_found(
            "guild.not_found",
            "Guild was not found.",
        ));
    }

    Ok(())
}

fn select_character_columns() -> &'static str {
    r#"
        c.id,
        c.user_id,
        c.name,
        c.region,
        c.server,
        c.class,
        c.race,
        c.spec,
        c.role,
        c.rank,
        c.created_at,
        c.updated_at,
        c.deleted_at,
        b.character_id as battlenet_snapshot_character_id,
        b.battlenet_character_id,
        b.race_id as battlenet_race_id,
        b.race_name as battlenet_race_name,
        b.class_id as battlenet_class_id,
        b.class_name as battlenet_class_name,
        b.gender as battlenet_gender,
        b.faction as battlenet_faction,
        b.level as battlenet_level,
        b.active_spec_id as battlenet_active_spec_id,
        b.active_spec_name as battlenet_active_spec_name,
        b.average_item_level as battlenet_average_item_level,
        b.equipped_item_level as battlenet_equipped_item_level,
        b.avatar_url as battlenet_avatar_url,
        b.main_raw_url as battlenet_main_raw_url,
        b.fetched_at as battlenet_fetched_at,
        b.error as battlenet_error,
        b.raw as battlenet_raw,
        b.created_at as battlenet_created_at,
        b.updated_at as battlenet_updated_at,
        w.character_id as warcraftlogs_snapshot_character_id,
        w.warcraftlogs_character_id,
        w.gear as warcraftlogs_gear,
        w.rankings as warcraftlogs_rankings,
        w.best_performance_average as warcraftlogs_best_performance_average,
        w.median_performance_average as warcraftlogs_median_performance_average,
        w.total_kills as warcraftlogs_total_kills,
        w.fetched_at as warcraftlogs_fetched_at,
        w.error as warcraftlogs_error,
        w.partial as warcraftlogs_partial,
        w.raw as warcraftlogs_raw,
        w.created_at as warcraftlogs_created_at,
        w.updated_at as warcraftlogs_updated_at
    "#
}

fn character_row_to_dto(row: CharacterRow) -> Result<CharacterDto, ApiError> {
    let battlenet = battlenet_snapshot_to_dto(&row)?;
    let warcraftlogs = warcraftlogs_snapshot_to_dto(&row)?;

    Ok(CharacterDto {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        region: row.region,
        server: row.server,
        class: row.class,
        race: row.race,
        spec: row.spec,
        role: parse_role(&row.role)?,
        rank: parse_rank(&row.rank)?,
        battlenet,
        warcraftlogs,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
    })
}

fn battlenet_snapshot_to_dto(
    row: &CharacterRow,
) -> Result<Option<CharacterBattleNetSnapshotDto>, ApiError> {
    if row.battlenet_snapshot_character_id.is_none() {
        return Ok(None);
    }

    Ok(Some(CharacterBattleNetSnapshotDto {
        battlenet_character_id: row.battlenet_character_id,
        race_id: row.battlenet_race_id,
        race_name: row.battlenet_race_name.clone(),
        class_id: row.battlenet_class_id,
        class_name: row.battlenet_class_name.clone(),
        gender: row.battlenet_gender.clone(),
        faction: row.battlenet_faction.clone(),
        level: row.battlenet_level,
        active_spec_id: row.battlenet_active_spec_id,
        active_spec_name: row.battlenet_active_spec_name.clone(),
        average_item_level: row.battlenet_average_item_level,
        equipped_item_level: row.battlenet_equipped_item_level,
        avatar_url: row.battlenet_avatar_url.clone(),
        main_raw_url: row.battlenet_main_raw_url.clone(),
        fetched_at: row.battlenet_fetched_at,
        error: row.battlenet_error.clone(),
        raw: row.battlenet_raw.clone(),
        created_at: row.battlenet_created_at.ok_or_else(ApiError::internal)?,
        updated_at: row.battlenet_updated_at.ok_or_else(ApiError::internal)?,
    }))
}

fn warcraftlogs_snapshot_to_dto(
    row: &CharacterRow,
) -> Result<Option<CharacterWarcraftLogsSnapshotDto>, ApiError> {
    if row.warcraftlogs_snapshot_character_id.is_none() {
        return Ok(None);
    }

    Ok(Some(CharacterWarcraftLogsSnapshotDto {
        warcraftlogs_character_id: row.warcraftlogs_character_id,
        gear: row.warcraftlogs_gear.clone(),
        rankings: row.warcraftlogs_rankings.clone(),
        best_performance_average: row.warcraftlogs_best_performance_average,
        median_performance_average: row.warcraftlogs_median_performance_average,
        total_kills: row
            .warcraftlogs_total_kills
            .ok_or_else(ApiError::internal)?,
        fetched_at: row.warcraftlogs_fetched_at,
        error: row.warcraftlogs_error.clone(),
        partial: row.warcraftlogs_partial.ok_or_else(ApiError::internal)?,
        raw: row.warcraftlogs_raw.clone(),
        created_at: row.warcraftlogs_created_at.ok_or_else(ApiError::internal)?,
        updated_at: row.warcraftlogs_updated_at.ok_or_else(ApiError::internal)?,
    }))
}

fn parse_role(value: &str) -> Result<CharacterRole, ApiError> {
    value.parse().map_err(|_| ApiError::internal())
}

fn parse_rank(value: &str) -> Result<CharacterRank, ApiError> {
    value.parse().map_err(|_| ApiError::internal())
}

fn character_not_found() -> ApiError {
    ApiError::not_found("character.not_found", "Character was not found.")
}

fn json_response<T: serde::Serialize>(
    mut builder: actix_web::HttpResponseBuilder,
    refreshed_cookie: Option<actix_web::cookie::Cookie<'static>>,
    payload: T,
) -> HttpResponse {
    if let Some(cookie) = refreshed_cookie {
        builder.cookie(cookie);
    }
    builder.json(payload)
}
