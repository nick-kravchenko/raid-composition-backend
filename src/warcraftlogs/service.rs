use std::time::Duration;

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::service as auth_service,
    state::AppState,
    warcraftlogs::{
        client::{WarcraftLogsClient, WarcraftLogsError},
        dto::{
            Actor, CharacterRankingsResponse, CombatantInfoResponse, RecentReportsResponse,
            ReportMasterDataResponse, ZoneRankingsSummary,
        },
        normalize::normalize_zone_rankings,
        queries::{
            GET_CHARACTER_RANKINGS, GET_CHARACTER_RECENT_REPORT, GET_REPORT_COMBATANT_INFO,
            GET_REPORT_MASTER_DATA,
        },
    },
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
    warcraftlogs_character_id: Option<i64>,
    gear: Option<Value>,
    rankings: Option<Value>,
    best_performance_average: Option<f64>,
    median_performance_average: Option<f64>,
    total_kills: i32,
    fetched_at: Option<DateTime<Utc>>,
    error: Option<String>,
    partial: bool,
    raw: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct SnapshotDraft {
    pub character_id: Uuid,
    pub warcraftlogs_character_id: Option<i64>,
    pub gear: Option<Value>,
    pub rankings: Value,
    pub best_performance_average: Option<f64>,
    pub median_performance_average: Option<f64>,
    pub total_kills: i32,
    pub error: Option<String>,
    pub partial: bool,
    pub raw: Value,
}

#[derive(Debug, Serialize)]
pub struct WarcraftLogsSnapshotResponseDto {
    pub snapshot: WarcraftLogsSnapshotDto,
}

#[derive(Debug, Serialize)]
pub struct WarcraftLogsSnapshotDto {
    pub character_id: Uuid,
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
}

pub async fn refresh_character_warcraftlogs_response(
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
    Ok(response.json(WarcraftLogsSnapshotResponseDto {
        snapshot: snapshot_row_to_dto(snapshot),
    }))
}

async fn refresh_character_snapshot(
    state: &AppState,
    user_id: Uuid,
    character_id: Uuid,
) -> Result<SnapshotRow, ApiError> {
    let character = load_user_character(&state.db_pool, user_id, character_id).await?;
    let client = WarcraftLogsClient::new(&state.http_client, &state.config.warcraftlogs);
    let token = client.access_token().await.map_err(wcl_bad_gateway)?;

    let rankings_response = client
        .query::<CharacterRankingsResponse>(
            &token,
            GET_CHARACTER_RANKINGS,
            json!({
                "name": character.name.trim(),
                "serverSlug": character.server.trim().to_ascii_lowercase(),
                "serverRegion": character.region.trim().to_ascii_lowercase(),
                "zoneID": state.config.warcraftlogs.zone_id,
            }),
        )
        .await
        .map_err(wcl_bad_gateway)?;

    let Some(wcl_character) = rankings_response.character_data.character else {
        return Err(ApiError::not_found(
            "warcraftlogs.character_not_found",
            "Warcraft Logs character was not found.",
        ));
    };

    let rankings = wcl_character.zone_rankings.unwrap_or(Value::Null);
    let summary = normalize_zone_rankings(Some(&rankings));
    let raw_character = json!({
        "id": wcl_character.id,
        "name": wcl_character.name,
        "classID": wcl_character.class_id,
        "server": wcl_character.server,
        "zoneRankings": rankings,
    });

    let (gear, gear_error) = if state.config.warcraftlogs.skip_gear {
        (None, None)
    } else {
        delay_between_wcl_calls(state.config.warcraftlogs.api_delay_ms).await;
        match fetch_gear(&client, &token, &character).await {
            Ok(gear) => (gear, None),
            Err(error) => (None, Some(error.to_string())),
        }
    };

    let snapshot = build_snapshot(
        character.id,
        Some(wcl_character.id),
        rankings,
        summary,
        gear,
        gear_error,
        raw_character,
    );

    upsert_snapshot(&state.db_pool, snapshot).await
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

async fn fetch_gear(
    client: &WarcraftLogsClient<'_>,
    token: &str,
    character: &CharacterRow,
) -> Result<Option<Value>, WarcraftLogsError> {
    let recent_report = client
        .query::<RecentReportsResponse>(
            token,
            GET_CHARACTER_RECENT_REPORT,
            json!({
                "name": character.name.trim(),
                "serverSlug": character.server.trim().to_ascii_lowercase(),
                "serverRegion": character.region.trim().to_ascii_lowercase(),
            }),
        )
        .await?;

    let report = recent_report
        .character_data
        .character
        .and_then(|character| character.recent_reports.data.into_iter().next())
        .ok_or(WarcraftLogsError::MissingData)?;
    let fight_id = report
        .fights
        .and_then(|fights| fights.into_iter().next())
        .map(|fight| fight.id)
        .ok_or(WarcraftLogsError::MissingData)?;

    let master_data = client
        .query::<ReportMasterDataResponse>(
            token,
            GET_REPORT_MASTER_DATA,
            json!({ "reportCode": report.code }),
        )
        .await?;
    let actors = master_data
        .report_data
        .report
        .and_then(|report| report.master_data)
        .map(|master_data| master_data.actors)
        .ok_or(WarcraftLogsError::MissingData)?;
    let source_id =
        find_player_actor_id(&actors, &character.name).ok_or(WarcraftLogsError::MissingData)?;

    let combatant_info = client
        .query::<CombatantInfoResponse>(
            token,
            GET_REPORT_COMBATANT_INFO,
            json!({
                "reportCode": report.code,
                "fightID": fight_id,
                "sourceID": source_id,
            }),
        )
        .await?;

    Ok(combatant_info
        .report_data
        .report
        .and_then(|report| report.events.data.into_iter().next())
        .and_then(|event| event.get("gear").cloned()))
}

pub fn find_player_actor_id(actors: &[Actor], character_name: &str) -> Option<i32> {
    let character_name = character_name.to_lowercase();
    actors
        .iter()
        .find(|actor| actor.r#type == "Player" && actor.name.to_lowercase() == character_name)
        .map(|actor| actor.id)
}

pub fn build_snapshot(
    character_id: Uuid,
    warcraftlogs_character_id: Option<i64>,
    rankings: Value,
    summary: ZoneRankingsSummary,
    gear: Option<Value>,
    gear_error: Option<String>,
    raw: Value,
) -> SnapshotDraft {
    SnapshotDraft {
        character_id,
        warcraftlogs_character_id,
        gear,
        rankings,
        best_performance_average: summary.best_performance_average,
        median_performance_average: summary.median_performance_average,
        total_kills: summary.total_kills,
        partial: gear_error.is_some(),
        error: gear_error,
        raw,
    }
}

async fn upsert_snapshot(
    db_pool: &PgPool,
    snapshot: SnapshotDraft,
) -> Result<SnapshotRow, ApiError> {
    sqlx::query_as::<_, SnapshotRow>(
        r#"
        insert into character_warcraftlogs_snapshots (
            character_id,
            warcraftlogs_character_id,
            gear,
            rankings,
            best_performance_average,
            median_performance_average,
            total_kills,
            fetched_at,
            error,
            partial,
            raw
        )
        values ($1, $2, $3, $4, $5, $6, $7, now(), $8, $9, $10)
        on conflict (character_id) do update
        set warcraftlogs_character_id = excluded.warcraftlogs_character_id,
            gear = excluded.gear,
            rankings = excluded.rankings,
            best_performance_average = excluded.best_performance_average,
            median_performance_average = excluded.median_performance_average,
            total_kills = excluded.total_kills,
            fetched_at = excluded.fetched_at,
            error = excluded.error,
            partial = excluded.partial,
            raw = excluded.raw
        returning character_id,
                  warcraftlogs_character_id,
                  gear,
                  rankings,
                  best_performance_average,
                  median_performance_average,
                  total_kills,
                  fetched_at,
                  error,
                  partial,
                  raw
        "#,
    )
    .bind(snapshot.character_id)
    .bind(snapshot.warcraftlogs_character_id)
    .bind(snapshot.gear)
    .bind(snapshot.rankings)
    .bind(snapshot.best_performance_average)
    .bind(snapshot.median_performance_average)
    .bind(snapshot.total_kills)
    .bind(snapshot.error)
    .bind(snapshot.partial)
    .bind(snapshot.raw)
    .fetch_one(db_pool)
    .await
    .map_err(ApiError::from)
}

fn snapshot_row_to_dto(row: SnapshotRow) -> WarcraftLogsSnapshotDto {
    WarcraftLogsSnapshotDto {
        character_id: row.character_id,
        warcraftlogs_character_id: row.warcraftlogs_character_id,
        gear: row.gear,
        rankings: row.rankings,
        best_performance_average: row.best_performance_average,
        median_performance_average: row.median_performance_average,
        total_kills: row.total_kills,
        fetched_at: row.fetched_at,
        error: row.error,
        partial: row.partial,
        raw: row.raw,
    }
}

async fn delay_between_wcl_calls(delay_ms: u64) {
    if delay_ms > 0 {
        actix_web::rt::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

fn wcl_bad_gateway(error: WarcraftLogsError) -> ApiError {
    eprintln!("warcraft logs error: {error}");
    ApiError::bad_gateway(
        "warcraftlogs.upstream_failed",
        "Warcraft Logs request failed.",
    )
}
