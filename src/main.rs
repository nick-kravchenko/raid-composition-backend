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
mod config;
mod db;
mod guilds;
mod state;

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
