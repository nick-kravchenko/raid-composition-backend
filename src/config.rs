use std::{env, error::Error, fmt, path::PathBuf};

use actix_web::cookie::SameSite;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub frontend: FrontendConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub discord: DiscordConfig,
    pub cookie: CookieConfig,
    pub security: SecurityConfig,
    pub geoip: GeoIpConfig,
    pub warcraftlogs: WarcraftLogsConfig,
    pub battlenet: BattleNetConfig,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct FrontendConfig {
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub domain: String,
    pub secure: bool,
    pub same_site: SameSite,
    pub session_name: String,
    pub csrf_name: String,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub discord_token_encryption_key: [u8; 32],
    pub session_hmac_secret: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GeoIpConfig {
    pub database_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WarcraftLogsConfig {
    pub client_id: String,
    pub client_secret: String,
    pub oauth_token_url: String,
    pub endpoint: String,
    pub zone_id: Option<u32>,
    pub skip_gear: bool,
    pub api_delay_ms: u64,
}

#[derive(Debug, Clone)]
pub struct BattleNetConfig {
    pub client_id: String,
    pub client_secret: String,
    pub game_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Missing {
        name: &'static str,
    },
    Empty {
        name: &'static str,
    },
    InvalidPort {
        name: &'static str,
        value: String,
    },
    ZeroPort {
        name: &'static str,
    },
    InvalidBool {
        name: &'static str,
        value: String,
    },
    InvalidSameSite {
        value: String,
    },
    InvalidUrl {
        name: &'static str,
        value: String,
    },
    InvalidUnsignedInteger {
        name: &'static str,
        value: String,
    },
    WeakSecret {
        name: &'static str,
        minimum_bytes: usize,
    },
    InvalidEncryptionKey {
        name: &'static str,
    },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_source(|name| env::var(name).ok())
    }

    fn from_source<F>(get: F) -> Result<Self, ConfigError>
    where
        F: Fn(&'static str) -> Option<String>,
    {
        let frontend_base_url = required_url(&get, "FRONTEND_BASE_URL")?;

        Ok(Self {
            app: AppConfig {
                port: required_port(&get, "APP_PORT")?,
            },
            frontend: FrontendConfig {
                base_url: frontend_base_url,
            },
            database: DatabaseConfig {
                host: required_string(&get, "DB_HOST")?,
                port: required_port(&get, "DB_PORT")?,
                user: required_string(&get, "DB_USER")?,
                password: required_string(&get, "DB_PASSWORD")?,
                name: required_string(&get, "DB_NAME")?,
            },
            redis: RedisConfig {
                host: required_string(&get, "REDIS_HOST")?,
                port: required_port(&get, "REDIS_PORT")?,
                password: required_string(&get, "REDIS_PASSWORD")?,
            },
            discord: DiscordConfig {
                client_id: required_string(&get, "DISCORD_CLIENT_ID")?,
                client_secret: required_string(&get, "DISCORD_CLIENT_SECRET")?,
                redirect_url: required_url(&get, "DISCORD_REDIRECT_URL")?,
            },
            cookie: CookieConfig {
                domain: required_string(&get, "COOKIE_DOMAIN")?,
                secure: optional_bool(&get, "COOKIE_SECURE", true)?,
                same_site: optional_same_site(&get, "COOKIE_SAME_SITE", SameSite::Lax)?,
                session_name: optional_string(&get, "SESSION_COOKIE_NAME", "session")?,
                csrf_name: optional_string(&get, "CSRF_COOKIE_NAME", "csrf")?,
            },
            security: SecurityConfig {
                discord_token_encryption_key: required_encryption_key(
                    &get,
                    "DISCORD_TOKEN_ENCRYPTION_KEY",
                )?,
                session_hmac_secret: required_secret(&get, "SESSION_HMAC_SECRET", 32)?,
            },
            geoip: GeoIpConfig {
                database_path: PathBuf::from(required_string(&get, "GEOIP_DATABASE_PATH")?),
            },
            warcraftlogs: WarcraftLogsConfig {
                client_id: required_string(&get, "WCL_CLIENT_ID")?,
                client_secret: required_string(&get, "WCL_CLIENT_SECRET")?,
                oauth_token_url: optional_url(
                    &get,
                    "WCL_OAUTH_TOKEN_URL",
                    "https://www.warcraftlogs.com/oauth/token",
                )?,
                endpoint: optional_url(
                    &get,
                    "WCL_ENDPOINT",
                    "https://fresh.warcraftlogs.com/api/v2/client",
                )?,
                zone_id: optional_u32(&get, "WCL_ZONE_ID")?,
                skip_gear: optional_bool(&get, "WCL_SKIP_GEAR", false)?,
                api_delay_ms: optional_u64(&get, "WCL_API_DELAY_MS", 300)?,
            },
            battlenet: BattleNetConfig {
                client_id: required_string(&get, "BNET_CLIENT_ID")?,
                client_secret: required_string(&get, "BNET_CLIENT_SECRET")?,
                game_version: optional_string(&get, "BNET_GAME_VERSION", "classic")?,
            },
        })
    }
}

impl DatabaseConfig {
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            percent_encode_url_component(&self.user),
            percent_encode_url_component(&self.password),
            self.host,
            self.port,
            self.name
        )
    }
}

impl RedisConfig {
    pub fn url(&self) -> String {
        format!(
            "redis://:{}@{}:{}/",
            percent_encode_url_component(&self.password),
            self.host,
            self.port
        )
    }
}

impl DiscordConfig {
    pub fn authorization_url(&self, state: &str) -> String {
        format!(
            "https://discord.com/oauth2/authorize?client_id={}&response_type=code&redirect_uri={}&scope=identify&state={}",
            percent_encode_url_component(&self.client_id),
            percent_encode_url_component(&self.redirect_url),
            percent_encode_url_component(state)
        )
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { name } => {
                write!(formatter, "required environment variable {name} is missing")
            }
            Self::Empty { name } => {
                write!(formatter, "required environment variable {name} is empty")
            }
            Self::InvalidPort { name, value } => write!(
                formatter,
                "environment variable {name} must be a valid u16 port, got {value:?}"
            ),
            Self::ZeroPort { name } => {
                write!(formatter, "environment variable {name} must be non-zero")
            }
            Self::InvalidBool { name, value } => write!(
                formatter,
                "environment variable {name} must be true or false, got {value:?}"
            ),
            Self::InvalidSameSite { value } => write!(
                formatter,
                "COOKIE_SAME_SITE must be Lax, Strict, or None, got {value:?}"
            ),
            Self::InvalidUrl { name, value } => write!(
                formatter,
                "environment variable {name} must be an absolute http(s) URL, got {value:?}"
            ),
            Self::InvalidUnsignedInteger { name, value } => write!(
                formatter,
                "environment variable {name} must be an unsigned integer, got {value:?}"
            ),
            Self::WeakSecret {
                name,
                minimum_bytes,
            } => write!(
                formatter,
                "environment variable {name} must be at least {minimum_bytes} bytes"
            ),
            Self::InvalidEncryptionKey { name } => write!(
                formatter,
                "environment variable {name} must decode to exactly 32 bytes"
            ),
        }
    }
}

impl Error for ConfigError {}

fn required_string<F>(get: &F, name: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.trim().is_empty() => Err(ConfigError::Empty { name }),
        Some(value) => Ok(value),
        None => Err(ConfigError::Missing { name }),
    }
}

fn optional_string<F>(get: &F, name: &'static str, default: &str) -> Result<String, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.trim().is_empty() => Err(ConfigError::Empty { name }),
        Some(value) => Ok(value),
        None => Ok(default.to_string()),
    }
}

fn required_port<F>(get: &F, name: &'static str) -> Result<u16, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    let value = required_string(get, name)?;
    let port = value
        .parse::<u16>()
        .map_err(|_| ConfigError::InvalidPort { name, value })?;

    if port == 0 {
        return Err(ConfigError::ZeroPort { name });
    }

    Ok(port)
}

fn optional_bool<F>(get: &F, name: &'static str, default: bool) -> Result<bool, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        Some(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        Some(value) if value.trim().is_empty() => Err(ConfigError::Empty { name }),
        Some(value) => Err(ConfigError::InvalidBool { name, value }),
        None => Ok(default),
    }
}

fn optional_same_site<F>(
    get: &F,
    name: &'static str,
    default: SameSite,
) -> Result<SameSite, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.eq_ignore_ascii_case("lax") => Ok(SameSite::Lax),
        Some(value) if value.eq_ignore_ascii_case("strict") => Ok(SameSite::Strict),
        Some(value) if value.eq_ignore_ascii_case("none") => Ok(SameSite::None),
        Some(value) if value.trim().is_empty() => Err(ConfigError::Empty { name }),
        Some(value) => Err(ConfigError::InvalidSameSite { value }),
        None => Ok(default),
    }
}

fn required_url<F>(get: &F, name: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    let value = required_string(get, name)?;
    validate_url(name, &value)?;
    Ok(value)
}

fn optional_url<F>(get: &F, name: &'static str, default: &str) -> Result<String, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    let value = optional_string(get, name, default)?;
    validate_url(name, &value)?;
    Ok(value)
}

fn validate_url(name: &'static str, value: &str) -> Result<(), ConfigError> {
    let parsed = url::Url::parse(value).map_err(|_| ConfigError::InvalidUrl {
        name,
        value: value.to_string(),
    })?;

    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(ConfigError::InvalidUrl {
            name,
            value: value.to_string(),
        });
    }

    Ok(())
}

fn optional_u32<F>(get: &F, name: &'static str) -> Result<Option<u32>, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => value
            .parse::<u32>()
            .map(Some)
            .map_err(|_| ConfigError::InvalidUnsignedInteger { name, value }),
        None => Ok(None),
    }
}

fn optional_u64<F>(get: &F, name: &'static str, default: u64) -> Result<u64, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.trim().is_empty() => Err(ConfigError::Empty { name }),
        Some(value) => value
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidUnsignedInteger { name, value }),
        None => Ok(default),
    }
}

fn required_secret<F>(
    get: &F,
    name: &'static str,
    minimum_bytes: usize,
) -> Result<Vec<u8>, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    let value = required_string(get, name)?;
    if value.len() < minimum_bytes {
        return Err(ConfigError::WeakSecret {
            name,
            minimum_bytes,
        });
    }
    Ok(value.into_bytes())
}

fn required_encryption_key<F>(get: &F, name: &'static str) -> Result<[u8; 32], ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    let value = required_string(get, name)?;
    let decoded = decode_key(&value).ok_or(ConfigError::InvalidEncryptionKey { name })?;
    decoded
        .try_into()
        .map_err(|_| ConfigError::InvalidEncryptionKey { name })
}

fn decode_key(value: &str) -> Option<Vec<u8>> {
    [
        general_purpose::STANDARD.decode(value).ok(),
        general_purpose::URL_SAFE_NO_PAD.decode(value).ok(),
        decode_hex(value).ok(),
    ]
    .into_iter()
    .flatten()
    .find(|decoded| decoded.len() == 32)
}

fn decode_hex(value: &str) -> Result<Vec<u8>, ()> {
    if !value.len().is_multiple_of(2) {
        return Err(());
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).map_err(|_| ()))
        .collect()
}

pub fn percent_encode_url_component(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }

        encoded
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn valid_env() -> HashMap<&'static str, String> {
        HashMap::from([
            ("APP_PORT", "8000".to_string()),
            ("FRONTEND_BASE_URL", "http://localhost:4200".to_string()),
            ("DB_HOST", "localhost".to_string()),
            ("DB_PORT", "5432".to_string()),
            ("DB_USER", "user".to_string()),
            ("DB_PASSWORD", "password".to_string()),
            ("DB_NAME", "app_db".to_string()),
            ("REDIS_HOST", "localhost".to_string()),
            ("REDIS_PORT", "6379".to_string()),
            ("REDIS_PASSWORD", "password".to_string()),
            ("DISCORD_CLIENT_ID", "discord-client".to_string()),
            ("DISCORD_CLIENT_SECRET", "discord-secret".to_string()),
            (
                "DISCORD_REDIRECT_URL",
                "http://localhost:4200/auth/discord/callback".to_string(),
            ),
            ("COOKIE_DOMAIN", "localhost".to_string()),
            (
                "DISCORD_TOKEN_ENCRYPTION_KEY",
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
            (
                "SESSION_HMAC_SECRET",
                "01234567890123456789012345678901".to_string(),
            ),
            ("GEOIP_DATABASE_PATH", "/tmp/GeoLite2-City.mmdb".to_string()),
            ("WCL_CLIENT_ID", "wcl-client".to_string()),
            ("WCL_CLIENT_SECRET", "wcl-secret".to_string()),
            ("BNET_CLIENT_ID", "bnet-client".to_string()),
            ("BNET_CLIENT_SECRET", "bnet-secret".to_string()),
        ])
    }

    fn config_from(env: &HashMap<&'static str, String>) -> Result<Config, ConfigError> {
        Config::from_source(|name| env.get(name).cloned())
    }

    #[test]
    fn loads_config_from_complete_source() {
        let config = config_from(&valid_env()).expect("config should load");

        assert_eq!(config.app.port, 8000);
        assert_eq!(config.frontend.base_url, "http://localhost:4200");
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.redis.port, 6379);
        assert_eq!(config.discord.client_id, "discord-client");
        assert_eq!(
            config.discord.redirect_url,
            "http://localhost:4200/auth/discord/callback"
        );
        assert_eq!(config.cookie.domain, "localhost");
        assert_eq!(config.cookie.session_name, "session");
        assert_eq!(config.cookie.csrf_name, "csrf");
        assert_eq!(config.warcraftlogs.client_id, "wcl-client");
        assert_eq!(config.warcraftlogs.client_secret, "wcl-secret");
        assert_eq!(
            config.warcraftlogs.oauth_token_url,
            "https://www.warcraftlogs.com/oauth/token"
        );
        assert_eq!(
            config.warcraftlogs.endpoint,
            "https://fresh.warcraftlogs.com/api/v2/client"
        );
        assert_eq!(config.warcraftlogs.zone_id, None);
        assert!(!config.warcraftlogs.skip_gear);
        assert_eq!(config.warcraftlogs.api_delay_ms, 300);
        assert_eq!(config.battlenet.client_id, "bnet-client");
        assert_eq!(config.battlenet.client_secret, "bnet-secret");
        assert_eq!(config.battlenet.game_version, "classic");
    }

    #[test]
    fn missing_battlenet_client_id_is_an_error() {
        let mut env = valid_env();
        env.remove("BNET_CLIENT_ID");

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::Missing {
                name: "BNET_CLIENT_ID"
            }
        );
    }

    #[test]
    fn loads_optional_battlenet_values() {
        let mut env = valid_env();
        env.insert("BNET_GAME_VERSION", "classic1x".to_string());

        let config = config_from(&env).expect("config should load");

        assert_eq!(config.battlenet.game_version, "classic1x");
    }

    #[test]
    fn missing_warcraftlogs_client_id_is_an_error() {
        let mut env = valid_env();
        env.remove("WCL_CLIENT_ID");

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::Missing {
                name: "WCL_CLIENT_ID"
            }
        );
    }

    #[test]
    fn loads_optional_warcraftlogs_values() {
        let mut env = valid_env();
        env.insert("WCL_ZONE_ID", "1012".to_string());
        env.insert("WCL_SKIP_GEAR", "true".to_string());
        env.insert("WCL_API_DELAY_MS", "750".to_string());
        env.insert("WCL_ENDPOINT", "https://example.test/graphql".to_string());
        env.insert(
            "WCL_OAUTH_TOKEN_URL",
            "https://example.test/oauth/token".to_string(),
        );

        let config = config_from(&env).expect("config should load");

        assert_eq!(config.warcraftlogs.zone_id, Some(1012));
        assert!(config.warcraftlogs.skip_gear);
        assert_eq!(config.warcraftlogs.api_delay_ms, 750);
        assert_eq!(config.warcraftlogs.endpoint, "https://example.test/graphql");
        assert_eq!(
            config.warcraftlogs.oauth_token_url,
            "https://example.test/oauth/token"
        );
    }

    #[test]
    fn empty_optional_warcraftlogs_zone_id_is_unset() {
        let mut env = valid_env();
        env.insert("WCL_ZONE_ID", String::new());

        let config = config_from(&env).expect("config should load");

        assert_eq!(config.warcraftlogs.zone_id, None);
    }

    #[test]
    fn missing_required_variable_is_an_error() {
        let mut env = valid_env();
        env.remove("DB_HOST");

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::Missing { name: "DB_HOST" }
        );
    }

    #[test]
    fn empty_required_variable_is_an_error() {
        let mut env = valid_env();
        env.insert("DB_HOST", String::new());

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::Empty { name: "DB_HOST" }
        );
    }

    #[test]
    fn invalid_non_numeric_port_is_an_error() {
        let mut env = valid_env();
        env.insert("DB_PORT", "postgres".to_string());

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::InvalidPort {
                name: "DB_PORT",
                value: "postgres".to_string()
            }
        );
    }

    #[test]
    fn zero_port_is_an_error() {
        let mut env = valid_env();
        env.insert("APP_PORT", "0".to_string());

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::ZeroPort { name: "APP_PORT" }
        );
    }

    #[test]
    fn rejects_weak_hmac_secret() {
        let mut env = valid_env();
        env.insert("SESSION_HMAC_SECRET", "short".to_string());

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::WeakSecret {
                name: "SESSION_HMAC_SECRET",
                minimum_bytes: 32
            }
        );
    }

    #[test]
    fn rejects_malformed_encryption_key() {
        let mut env = valid_env();
        env.insert("DISCORD_TOKEN_ENCRYPTION_KEY", "too-short".to_string());

        assert_eq!(
            config_from(&env).expect_err("config should fail"),
            ConfigError::InvalidEncryptionKey {
                name: "DISCORD_TOKEN_ENCRYPTION_KEY"
            }
        );
    }

    #[test]
    fn database_url_escapes_user_and_password() {
        let config = DatabaseConfig {
            host: "db.local".to_string(),
            port: 5432,
            user: "user name".to_string(),
            password: "p@ss/word?".to_string(),
            name: "raid".to_string(),
        };

        assert_eq!(
            config.url(),
            "postgres://user%20name:p%40ss%2Fword%3F@db.local:5432/raid"
        );
    }

    #[test]
    fn redis_url_escapes_password() {
        let config = RedisConfig {
            host: "redis.local".to_string(),
            port: 6379,
            password: "p@ss/word?".to_string(),
        };

        assert_eq!(config.url(), "redis://:p%40ss%2Fword%3F@redis.local:6379/");
    }

    #[test]
    fn discord_authorization_url_escapes_query_values_and_includes_state() {
        let config = DiscordConfig {
            client_id: "client id".to_string(),
            client_secret: "secret".to_string(),
            redirect_url: "http://localhost:4200/auth?next=/raid".to_string(),
        };

        assert_eq!(
            config.authorization_url("state value"),
            "https://discord.com/oauth2/authorize?client_id=client%20id&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A4200%2Fauth%3Fnext%3D%2Fraid&scope=identify&state=state%20value"
        );
    }
}
