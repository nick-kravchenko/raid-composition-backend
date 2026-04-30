use std::{env, error::Error, fmt};

#[derive(Debug, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub frontend: FrontendConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub discord: DiscordConfig,
    pub cookie: CookieConfig,
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
}

#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Missing { name: &'static str },
    Empty { name: &'static str },
    InvalidPort { name: &'static str, value: String },
    ZeroPort { name: &'static str },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_source(|name| env::var(name).ok())
    }

    fn from_source<F>(get: F) -> Result<Self, ConfigError>
    where
        F: Fn(&'static str) -> Option<String>,
    {
        Ok(Self {
            app: AppConfig {
                port: required_port(&get, "APP_PORT")?,
            },
            frontend: FrontendConfig {
                base_url: required_string(&get, "FRONTEND_BASE_URL")?,
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
            },
            cookie: CookieConfig {
                domain: required_string(&get, "COOKIE_DOMAIN")?,
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
    pub fn authorization_url(&self, frontend_base_url: &str) -> String {
        format!(
            "https://discord.com/oauth2/authorize?client_id={}&response_type=code&redirect_uri={}&scope=identify",
            percent_encode_url_component(&self.client_id),
            percent_encode_url_component(frontend_base_url)
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
        }
    }
}

impl Error for ConfigError {}

fn required_string<F>(get: &F, name: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&'static str) -> Option<String>,
{
    match get(name) {
        Some(value) if value.is_empty() => Err(ConfigError::Empty { name }),
        Some(value) => Ok(value),
        None => Err(ConfigError::Missing { name }),
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
            ("COOKIE_DOMAIN", "localhost".to_string()),
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
        assert_eq!(config.cookie.domain, "localhost");
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
    fn discord_authorization_url_escapes_query_values() {
        let config = DiscordConfig {
            client_id: "client id".to_string(),
            client_secret: "secret".to_string(),
        };

        assert_eq!(
            config.authorization_url("http://localhost:4200/auth?next=/raid"),
            "https://discord.com/oauth2/authorize?client_id=client%20id&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A4200%2Fauth%3Fnext%3D%2Fraid&scope=identify"
        );
    }

    #[test]
    fn complete_source_without_callback_specific_redirect_uri_loads() {
        let env = valid_env();

        let config = config_from(&env).expect("config should load");

        assert_eq!(config.frontend.base_url, "http://localhost:4200");
    }
}
