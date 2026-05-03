use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRankingsResponse {
    pub character_data: CharacterData,
}

#[derive(Debug, Deserialize)]
pub struct CharacterData {
    pub character: Option<WclCharacter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WclCharacter {
    pub id: i64,
    pub name: String,
    pub class_id: i32,
    pub server: WclServer,
    pub zone_rankings: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WclServer {
    pub name: String,
    pub slug: String,
    pub region: WclRegion,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WclRegion {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentReportsResponse {
    pub character_data: RecentReportCharacterData,
}

#[derive(Debug, Deserialize)]
pub struct RecentReportCharacterData {
    pub character: Option<RecentReportCharacter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentReportCharacter {
    pub recent_reports: RecentReports,
}

#[derive(Debug, Deserialize)]
pub struct RecentReports {
    pub data: Vec<RecentReport>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentReport {
    pub code: String,
    pub fights: Option<Vec<Fight>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fight {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMasterDataResponse {
    pub report_data: ReportData,
}

#[derive(Debug, Deserialize)]
pub struct ReportData {
    pub report: Option<ReportMasterDataReport>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMasterDataReport {
    pub master_data: Option<MasterData>,
}

#[derive(Debug, Deserialize)]
pub struct MasterData {
    pub actors: Vec<Actor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: i32,
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatantInfoResponse {
    pub report_data: CombatantInfoReportData,
}

#[derive(Debug, Deserialize)]
pub struct CombatantInfoReportData {
    pub report: Option<CombatantInfoReport>,
}

#[derive(Debug, Deserialize)]
pub struct CombatantInfoReport {
    pub events: CombatantInfoEvents,
}

#[derive(Debug, Deserialize)]
pub struct CombatantInfoEvents {
    pub data: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ZoneRankingsSummary {
    pub rankings: Vec<EncounterRanking>,
    pub best_performance_average: Option<f64>,
    pub median_performance_average: Option<f64>,
    pub total_kills: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EncounterRanking {
    pub encounter_id: i64,
    pub encounter_name: String,
    pub rank_percent: Option<f64>,
    pub median_performance: Option<f64>,
    pub total_kills: i32,
    pub fastest_kill: i64,
    pub best_amount: f64,
    pub spec: String,
    pub all_stars: Option<AllStars>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AllStars {
    pub points: f64,
    pub rank: Option<i64>,
    pub rank_percent: Option<f64>,
    pub total: i64,
}
