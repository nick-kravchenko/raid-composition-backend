use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, http::header, web};
use dotenv::dotenv;

use crate::api::error::json_error_handler;
use crate::api::routes;
use crate::auth::geoip::GeoIp;
use crate::config::Config;
use crate::state::AppState;

mod api;
mod auth;
mod battlenet;
mod characters;
mod config;
mod db;
mod guilds;
mod state;
mod warcraftlogs;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config = Config::from_env()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;

    println!("Configuration loaded.");
    println!("App Port: {}", config.app.port);
    println!("Frontend Base URL: {}", config.frontend.base_url);
    println!("Database Host: {}", config.database.host);
    println!("Database Port: {}", config.database.port);
    println!("Database User: {}", mask_secret(&config.database.user));
    println!(
        "Database Password: {}",
        mask_secret(&config.database.password)
    );
    println!("Database Name: {}", config.database.name);
    println!("Redis Host: {}", config.redis.host);
    println!("Redis Port: {}", config.redis.port);
    println!("Redis Password: {}", mask_secret(&config.redis.password));
    println!("Discord Client ID: {}", config.discord.client_id);
    println!(
        "Discord Client Secret: {}",
        mask_secret(&config.discord.client_secret)
    );
    println!("Discord Redirect URL: {}", config.discord.redirect_url);
    println!("Cookie Domain: {}", config.cookie.domain);
    println!(
        "GeoIP Database Path: {}",
        config.geoip.database_path.display()
    );
    println!("WCL Client ID: {}", config.warcraftlogs.client_id);
    println!(
        "WCL Client Secret: {}",
        mask_secret(&config.warcraftlogs.client_secret)
    );
    println!(
        "WCL OAuth Token URL: {}",
        config.warcraftlogs.oauth_token_url
    );
    println!("WCL Endpoint: {}", config.warcraftlogs.endpoint);
    println!("WCL Zone ID: {:?}", config.warcraftlogs.zone_id);
    println!("WCL Skip Gear: {}", config.warcraftlogs.skip_gear);
    println!("WCL API Delay Ms: {}", config.warcraftlogs.api_delay_ms);
    println!("Battle.net Client ID: {}", config.battlenet.client_id);
    println!(
        "Battle.net Client Secret: {}",
        mask_secret(&config.battlenet.client_secret)
    );
    println!("Battle.net Game Version: {}", config.battlenet.game_version);
    println!("Run DB Seeds: {}", config.seeds.run_db_seeds);

    let app_port: u16 = config.app.port;
    println!("Starting server on port: {}", app_port);

    println!("Connecting to the database...");
    let db_pool = db::connect(&config.database).await.map_err(|error| {
        std::io::Error::other(format!("Failed to connect to database: {error}"))
    })?;
    println!("db_pool.size: {}", db_pool.size());
    println!("Database connection established.");

    println!("Running database migrations...");
    db::run_migrations(&db_pool).await.map_err(|error| {
        eprintln!("Failed to run database migrations: {error}");
        std::io::Error::other(format!("Failed to run database migrations: {error}"))
    })?;
    println!("Database migrations completed.");

    if config.seeds.run_db_seeds {
        println!("Running database seeds...");
        db::run_seeds(&db_pool).await.map_err(|error| {
            eprintln!("Failed to run database seeds: {error}");
            std::io::Error::other(format!("Failed to run database seeds: {error}"))
        })?;
        println!("Database seeds completed.");
    } else {
        println!("Database seeds skipped.");
    }

    println!("Creating Redis client...");
    let redis_client = redis::Client::open(config.redis.url()).map_err(|error| {
        std::io::Error::other(format!("Failed to create Redis client: {error}"))
    })?;
    println!("Redis client created.");

    let http_client = reqwest::Client::builder()
        .user_agent("raid-composition-backend/0.1")
        .build()
        .map_err(|error| std::io::Error::other(format!("Failed to create HTTP client: {error}")))?;
    let geoip = GeoIp::open(&config.geoip.database_path);

    let state = web::Data::new(AppState {
        config,
        db_pool,
        redis_client,
        http_client,
        geoip,
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&state.config.frontend.base_url)
            .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::HeaderName::from_static("x-csrf-token"),
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(state.clone())
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
            .wrap(Logger::default())
            .wrap(cors)
            .service(routes::api_v1())
    })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}

fn mask_secret(_value: &str) -> &'static str {
    "***"
}

#[cfg(test)]
mod warcraftlogs_tests {
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    use crate::{
        config::Config,
        warcraftlogs::{
            client::{WarcraftLogsClient, build_basic_auth_value, extract_graphql_data},
            dto::{Actor, ZoneRankingsSummary},
            normalize::normalize_zone_rankings,
            queries::GET_CHARACTER_RANKINGS,
            service::{build_snapshot, find_player_actor_id},
        },
    };

    const WCL_TEST_CHARACTER_NAME: &str = "Kravá";

    #[test]
    fn warcraftlogs_basic_auth_header_uses_client_credentials() {
        assert_eq!(
            build_basic_auth_value("client", "secret"),
            "Basic Y2xpZW50OnNlY3JldA=="
        );
    }

    #[test]
    fn warcraftlogs_graphql_errors_are_rejected() {
        let error = extract_graphql_data::<serde_json::Value>(json!({
            "errors": [{ "message": "rate limited" }]
        }))
        .expect_err("graphql errors should fail");

        assert!(error.to_string().contains("rate limited"));
    }

    #[test]
    fn warcraftlogs_zone_rankings_are_normalized_and_sorted() {
        let raw = json!({
            "bestPerformanceAverage": 88.4,
            "medianPerformanceAverage": 72.2,
            "totalKills": 9,
            "rankings": [
                {
                    "encounter": { "id": 3, "name": "Third Boss" },
                    "rankPercent": 91.1,
                    "medianPercent": 73.0,
                    "totalKills": 2,
                    "fastestKill": 123000,
                    "bestAmount": 456.7,
                    "spec": "Fire",
                    "allStars": {
                        "points": 100.5,
                        "rank": "hidden",
                        "rankPercent": 95.0,
                        "total": 1000
                    }
                },
                {
                    "encounter": { "id": 1, "name": "First Boss" },
                    "rankPercent": null,
                    "medianPercent": null,
                    "totalKills": 7,
                    "fastestKill": 89000,
                    "bestAmount": 222.0,
                    "spec": "Fire",
                    "allStars": null
                }
            ]
        });

        let normalized = normalize_zone_rankings(Some(&raw));

        assert_eq!(normalized.best_performance_average, Some(88.4));
        assert_eq!(normalized.median_performance_average, Some(72.2));
        assert_eq!(normalized.total_kills, 9);
        assert_eq!(normalized.rankings[0].encounter_id, 1);
        assert_eq!(normalized.rankings[1].encounter_id, 3);
        assert_eq!(
            normalized.rankings[1].all_stars.as_ref().unwrap().rank,
            None
        );
    }

    #[test]
    fn warcraftlogs_actor_matching_uses_player_type_and_case_insensitive_name() {
        let actors = vec![
            Actor {
                id: 10,
                name: WCL_TEST_CHARACTER_NAME.to_string(),
                r#type: "Pet".to_string(),
            },
            Actor {
                id: 20,
                name: "KRAVÁ".to_string(),
                r#type: "Player".to_string(),
            },
        ];

        assert_eq!(
            find_player_actor_id(&actors, &WCL_TEST_CHARACTER_NAME.to_lowercase()),
            Some(20)
        );
    }

    #[test]
    fn warcraftlogs_snapshot_is_partial_when_gear_fetch_fails() {
        let snapshot = build_snapshot(
            uuid::Uuid::nil(),
            Some(1001),
            json!({ "rankings": [] }),
            ZoneRankingsSummary {
                rankings: Vec::new(),
                best_performance_average: Some(80.0),
                median_performance_average: Some(70.0),
                total_kills: 4,
            },
            None,
            Some("no recent report".to_string()),
            json!({ "character": "raw" }),
        );

        assert_eq!(snapshot.character_id, uuid::Uuid::nil());
        assert_eq!(snapshot.warcraftlogs_character_id, Some(1001));
        assert_eq!(snapshot.best_performance_average, Some(80.0));
        assert_eq!(snapshot.median_performance_average, Some(70.0));
        assert_eq!(snapshot.total_kills, 4);
        assert!(snapshot.partial);
        assert_eq!(snapshot.error.as_deref(), Some("no recent report"));
        assert_eq!(snapshot.gear, None);
    }

    #[actix_web::test]
    #[ignore = "requires real WCL credentials, database access, and external Warcraft Logs API access"]
    async fn warcraftlogs_live_rankings_fetches_krava_from_db_character_data() {
        dotenv::dotenv().ok();

        let config = Config::from_env().expect("backend .env should include WCL credentials");
        let db_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&config.database.url())
            .await
            .expect("database should be reachable");
        let (server_slug, server_region) = sqlx::query_as::<_, (String, String)>(
            r#"
            select server, region
            from characters
            where lower(name) = lower($1)
              and deleted_at is null
            order by created_at desc
            limit 1
            "#,
        )
        .bind(WCL_TEST_CHARACTER_NAME)
        .fetch_optional(&db_pool)
        .await
        .expect("character lookup should query")
        .expect("Kravá should exist in characters table");
        let http_client = reqwest::Client::builder()
            .user_agent("raid-composition-backend-test/0.1")
            .build()
            .expect("http client");
        let client = WarcraftLogsClient::new(&http_client, &config.warcraftlogs);
        let token = client.access_token().await.expect("WCL token");

        let response = client
            .query::<serde_json::Value>(
                &token,
                GET_CHARACTER_RANKINGS,
                json!({
                    "name": WCL_TEST_CHARACTER_NAME,
                    "serverSlug": server_slug,
                    "serverRegion": server_region,
                    "zoneID": config.warcraftlogs.zone_id,
                }),
            )
            .await
            .expect("WCL rankings response");

        assert_eq!(
            response["characterData"]["character"]["name"].as_str(),
            Some(WCL_TEST_CHARACTER_NAME)
        );
    }
}

#[cfg(test)]
mod battlenet_tests {
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    use crate::{
        battlenet::{
            client::{
                BattleNetClient, build_battlenet_basic_auth_value, build_character_media_url,
                build_character_url, build_oauth_token_url, profile_namespace,
            },
            dto::BattleNetCharacterSummary,
            normalize::normalize_character,
            service::{build_error_snapshot, build_snapshot},
        },
        config::Config,
    };

    const BNET_TEST_CHARACTER_NAME: &str = "Kravá";

    #[test]
    fn battlenet_oauth_and_profile_urls_match_baker_behavior() {
        assert_eq!(
            build_oauth_token_url("eu"),
            "https://eu.battle.net/oauth/token"
        );
        assert_eq!(
            build_battlenet_basic_auth_value("client", "secret"),
            "Basic Y2xpZW50OnNlY3JldA=="
        );
        assert_eq!(profile_namespace("eu", "retail"), "profile-eu");
        assert_eq!(profile_namespace("eu", "classic1x"), "profile-classic1x-eu");
        assert_eq!(profile_namespace("eu", "classic"), "profile-classicann-eu");
        assert_eq!(profile_namespace("us", "classic"), "profile-classicann-us");
        assert_eq!(
            build_character_url("eu", "classic", "Gehennas", BNET_TEST_CHARACTER_NAME),
            "https://eu.api.blizzard.com/profile/wow/character/gehennas/krav%C3%A1?namespace=profile-classicann-eu&locale=en_GB"
        );
        assert_eq!(
            build_character_media_url("eu", "classic", "Gehennas", BNET_TEST_CHARACTER_NAME),
            "https://eu.api.blizzard.com/profile/wow/character/gehennas/krav%C3%A1/character-media?namespace=profile-classicann-eu&locale=en_US"
        );
    }

    #[test]
    fn battlenet_client_uses_call_site_region_for_oauth_profile_and_media() {
        assert_eq!(
            build_oauth_token_url("us"),
            "https://us.battle.net/oauth/token"
        );
        assert_eq!(
            build_character_url("us", "classic", "Atiesh", BNET_TEST_CHARACTER_NAME),
            "https://us.api.blizzard.com/profile/wow/character/atiesh/krav%C3%A1?namespace=profile-classicann-us&locale=en_GB"
        );
        assert_eq!(
            build_character_media_url("us", "classic", "Atiesh", BNET_TEST_CHARACTER_NAME),
            "https://us.api.blizzard.com/profile/wow/character/atiesh/krav%C3%A1/character-media?namespace=profile-classicann-us&locale=en_US"
        );
    }

    #[test]
    fn battlenet_character_profile_and_media_are_normalized() {
        let profile = json!({
            "id": 12345,
            "name": BNET_TEST_CHARACTER_NAME,
            "gender": { "type": "FEMALE", "name": "Female" },
            "faction": { "type": "HORDE", "name": "Horde" },
            "race": { "id": 10, "name": "Blood Elf" },
            "character_class": { "id": 8, "name": "Mage" },
            "active_spec": { "id": 63, "name": "Fire" },
            "realm": { "id": 1, "name": "Gehennas", "slug": "gehennas" },
            "level": 80,
            "average_item_level": 226,
            "equipped_item_level": 224
        });
        let media = json!({
            "assets": [
                { "key": "avatar", "value": "https://example.test/avatar.jpg" },
                { "key": "main-raw", "value": "https://example.test/main.png" }
            ]
        });

        let normalized = normalize_character(&profile, Some(&media), "gehennas", "eu");

        assert_eq!(normalized.character_name, BNET_TEST_CHARACTER_NAME);
        assert_eq!(normalized.realm_slug, "gehennas");
        assert_eq!(normalized.region, "eu");
        assert_eq!(normalized.battlenet_character_id, Some(12345));
        assert_eq!(normalized.race_name.as_deref(), Some("Blood Elf"));
        assert_eq!(normalized.class_name.as_deref(), Some("Mage"));
        assert_eq!(normalized.gender.as_deref(), Some("female"));
        assert_eq!(normalized.faction.as_deref(), Some("horde"));
        assert_eq!(normalized.level, Some(80));
        assert_eq!(normalized.average_item_level, Some(226));
        assert_eq!(normalized.equipped_item_level, Some(224));
        assert_eq!(
            normalized.avatar_url.as_deref(),
            Some("https://example.test/avatar.jpg")
        );
        assert_eq!(
            normalized.main_raw_url.as_deref(),
            Some("https://example.test/main.png")
        );
    }

    #[test]
    fn battlenet_snapshot_uses_normalized_profile_values() {
        let summary = BattleNetCharacterSummary {
            character_name: "Kravá".to_string(),
            realm_slug: "gehennas".to_string(),
            region: "eu".to_string(),
            battlenet_character_id: Some(12345),
            race_id: Some(10),
            race_name: Some("Blood Elf".to_string()),
            class_id: Some(8),
            class_name: Some("Mage".to_string()),
            gender: Some("female".to_string()),
            faction: Some("horde".to_string()),
            level: Some(80),
            active_spec_id: Some(63),
            active_spec_name: Some("Fire".to_string()),
            average_item_level: Some(226),
            equipped_item_level: Some(224),
            avatar_url: Some("https://example.test/avatar.jpg".to_string()),
            main_raw_url: None,
            raw: json!({ "profile": { "id": 12345 }, "media": null }),
        };

        let snapshot = build_snapshot(uuid::Uuid::nil(), summary);

        assert_eq!(snapshot.character_id, uuid::Uuid::nil());
        assert_eq!(snapshot.battlenet_character_id, Some(12345));
        assert_eq!(snapshot.race_name.as_deref(), Some("Blood Elf"));
        assert_eq!(snapshot.class_name.as_deref(), Some("Mage"));
        assert_eq!(snapshot.gender.as_deref(), Some("female"));
        assert_eq!(snapshot.faction.as_deref(), Some("horde"));
        assert_eq!(snapshot.error, None);
    }

    #[test]
    fn battlenet_error_snapshot_records_character_not_found() {
        let snapshot = build_error_snapshot(
            uuid::Uuid::nil(),
            BNET_TEST_CHARACTER_NAME,
            "gehennas",
            "eu",
            "Character not found on gehennas-eu",
        );

        assert_eq!(snapshot.character_id, uuid::Uuid::nil());
        assert_eq!(snapshot.battlenet_character_id, None);
        assert_eq!(
            snapshot.error.as_deref(),
            Some("Character not found on gehennas-eu")
        );
        assert_eq!(
            snapshot.raw["characterName"].as_str(),
            Some(BNET_TEST_CHARACTER_NAME)
        );
        assert_eq!(snapshot.raw["realmSlug"].as_str(), Some("gehennas"));
        assert_eq!(snapshot.raw["region"].as_str(), Some("eu"));
    }

    #[actix_web::test]
    #[ignore = "requires real Battle.net credentials and external Blizzard API access"]
    async fn battlenet_live_profile_fetches_krava_from_db_character_data() {
        dotenv::dotenv().ok();

        let config =
            Config::from_env().expect("backend .env should include Battle.net credentials");
        let db_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&config.database.url())
            .await
            .expect("database should be reachable");
        let (realm_slug, region) = sqlx::query_as::<_, (String, String)>(
            r#"
            select server, region
            from characters
            where lower(name) = lower($1)
              and deleted_at is null
            order by created_at desc
            limit 1
            "#,
        )
        .bind(BNET_TEST_CHARACTER_NAME)
        .fetch_optional(&db_pool)
        .await
        .expect("character lookup should query")
        .expect("Kravá should exist in characters table");

        let http_client = reqwest::Client::builder()
            .user_agent("raid-composition-backend-test/0.1")
            .build()
            .expect("http client");
        let client = BattleNetClient::new(&http_client, &config.battlenet);
        let token = client
            .access_token(&region)
            .await
            .expect("Battle.net token");
        let profile = client
            .character(&token, &region, &realm_slug, BNET_TEST_CHARACTER_NAME)
            .await
            .expect("Battle.net profile response")
            .expect("Battle.net profile should exist");

        assert_eq!(profile["name"].as_str(), Some(BNET_TEST_CHARACTER_NAME));
    }
}
