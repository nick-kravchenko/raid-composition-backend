use serde::{ Serialize, Deserialize };
use sqlx::FromRow;
use sqlx::types::Uuid;
use chrono::{ DateTime, Utc };

#[derive(Deserialize, Serialize, Debug, Clone, FromRow)]
pub struct _UserDto {
  pub id: Uuid,
  pub discord_user_id: String,
  pub username: String,
  pub global_name: Option<String>,
  pub avatar: Option<String>,
  pub email: String,
  pub locale: Option<String>,
  pub verified: bool,
  pub mfa_enabled: bool,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}
