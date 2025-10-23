use dotenv::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
  pub app_port: u16,
  pub frontend_base_url: String,
  pub db_host: String,
  pub db_port: u16,
  pub db_user: String,
  pub db_password: String,
  pub db_name: String,
  pub discord_client_id: String,
  pub discord_client_secret: String,
  pub discord_redirect_uri: String,
  pub cookie_domain: String,
  pub jwt_secret: String,
  pub jwt_expiration: u64,
}

pub fn config() -> Config {
  dotenv().ok();

  Config {
    app_port: env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("APP_PORT must be a valid u16"),
    frontend_base_url: env::var("FRONTEND_BASE_URL").expect("FRONTEND_BASE_URL must be set"),
    db_host: env::var("DB_HOST").expect("DB_HOST must be set"),
    db_port: env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()).parse().expect("DB_PORT must be a valid u16"),
    db_user: env::var("DB_USER").expect("DB_USER must be set"),
    db_password: env::var("DB_PASSWORD").expect("DB_PASSWORD must be set"),
    db_name: env::var("DB_NAME").expect("DB_NAME must be set"),
    discord_client_id: env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID must be set"),
    discord_client_secret: env::var("DISCORD_CLIENT_SECRET").expect("DISCORD_CLIENT_SECRET must be set"),
    discord_redirect_uri: env::var("DISCORD_REDIRECT_URI").expect("DISCORD_REDIRECT_URI must be set"),
    cookie_domain: env::var("COOKIE_DOMAIN").expect("COOKIE_DOMAIN must be set"),
    jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
    jwt_expiration: env::var("JWT_EXPIRATION").unwrap_or_else(|_| "3600".to_string()).parse().expect("JWT_EXPIRATION must be a valid u64"),
  }
}