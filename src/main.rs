use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use dotenv::dotenv;

use crate::api::routes;
use crate::config::Config;
use crate::state::AppState;

mod api;
mod config;
mod db;
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
    println!("Cookie Domain: {}", config.cookie.domain);

    let app_port: u16 = config.app.port;
    println!("Starting server on port: {}", app_port);

    println!("Connecting to the database...");
    let db_pool = db::connect(&config.database)
        .await
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    println!("db_pool.size: {}", db_pool.size());
    println!("Database connection established.");

    println!("Creating Redis client...");
    let redis_client = redis::Client::open(config.redis.url()).map_err(|error| {
        std::io::Error::other(format!("Failed to create Redis client: {error}"))
    })?;
    println!("Redis client created.");

    let state = web::Data::new(AppState {
        config,
        db_pool,
        redis_client,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .service(routes::api_v1())
    })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}

fn mask_secret(_value: &str) -> &'static str {
    "***"
}
