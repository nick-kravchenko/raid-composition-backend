use serde_json::Value;

use crate::warcraftlogs::dto::{AllStars, EncounterRanking, ZoneRankingsSummary};

pub fn normalize_zone_rankings(raw: Option<&Value>) -> ZoneRankingsSummary {
    let Some(raw) = raw else {
        return ZoneRankingsSummary {
            rankings: Vec::new(),
            best_performance_average: None,
            median_performance_average: None,
            total_kills: 0,
        };
    };

    let mut rankings = raw
        .get("rankings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(normalize_encounter_ranking)
        .collect::<Vec<_>>();
    rankings.sort_by_key(|ranking| ranking.encounter_id);

    ZoneRankingsSummary {
        rankings,
        best_performance_average: raw.get("bestPerformanceAverage").and_then(Value::as_f64),
        median_performance_average: raw.get("medianPerformanceAverage").and_then(Value::as_f64),
        total_kills: raw
            .get("totalKills")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(0),
    }
}

fn normalize_encounter_ranking(raw: &Value) -> EncounterRanking {
    let encounter = raw.get("encounter").unwrap_or(&Value::Null);

    EncounterRanking {
        encounter_id: encounter.get("id").and_then(Value::as_i64).unwrap_or(0),
        encounter_name: encounter
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        rank_percent: raw.get("rankPercent").and_then(Value::as_f64),
        median_performance: raw.get("medianPercent").and_then(Value::as_f64),
        total_kills: raw
            .get("totalKills")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(0),
        fastest_kill: raw.get("fastestKill").and_then(Value::as_i64).unwrap_or(0),
        best_amount: raw.get("bestAmount").and_then(Value::as_f64).unwrap_or(0.0),
        spec: raw
            .get("spec")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        all_stars: raw.get("allStars").and_then(normalize_all_stars),
    }
}

fn normalize_all_stars(raw: &Value) -> Option<AllStars> {
    if raw.is_null() {
        return None;
    }

    Some(AllStars {
        points: raw.get("points").and_then(Value::as_f64).unwrap_or(0.0),
        rank: raw.get("rank").and_then(Value::as_i64),
        rank_percent: raw.get("rankPercent").and_then(Value::as_f64),
        total: raw.get("total").and_then(Value::as_i64).unwrap_or(0),
    })
}
