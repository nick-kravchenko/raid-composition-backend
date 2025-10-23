use std::sync::Arc;
use actix_web::{ App, HttpServer, web };
use actix_web::middleware::Logger;

use crate::api::routes;

mod config;
mod db;
mod api;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let config = config::config();
  println!("Configuration loaded.");
  println!("App Port: {}", config.app_port);
  println!("Frontend Base URL: {}", config.frontend_base_url);
  println!("Database Host: {}", config.db_host);
  println!("Database Port: {}", config.db_port);
  println!("Database User: {}", config.db_user);
  println!("Database Password: {}", config.db_password);
  println!("Database Name: {}", config.db_name);
  println!("Discord Client ID: {}", config.discord_client_id);
  println!("Discord Client Secret: {}", config.discord_client_secret);
  println!("Discord Redirect URI: {}", config.discord_redirect_uri);
  println!("Cookie Domain: {}", config.cookie_domain);
  println!("JWT Secret: {}", config.jwt_secret);
  println!("JWT Expiration: {}", config.jwt_expiration);

  let app_port: u16 = config.app_port;
  println!("Starting server on port: {}", app_port);

  println!("Connecting to the database...");
  let db_pool = db::connect().await.expect("Failed to connect to the database");
  println!("db_pool.size: {}", db_pool.size());
  println!("Database connection established.");

  HttpServer::new(|| {
    App::new()
      .app_data(web::Data::from(Arc::new(config::config())))
      .wrap(Logger::default())
      .service(routes::api_v1())
  })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}
