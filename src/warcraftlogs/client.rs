use std::{error::Error, fmt};

use base64::{Engine as _, engine::general_purpose};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::config::WarcraftLogsConfig;

#[derive(Debug)]
pub enum WarcraftLogsError {
    HttpStatus { status: StatusCode, body: String },
    Http(reqwest::Error),
    Graphql(String),
    MissingData,
    InvalidResponse(serde_json::Error),
}

impl fmt::Display for WarcraftLogsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpStatus { status, body } => {
                write!(formatter, "WCL HTTP {status}: {body}")
            }
            Self::Http(error) => write!(formatter, "WCL HTTP client error: {error}"),
            Self::Graphql(message) => write!(formatter, "WCL GraphQL error: {message}"),
            Self::MissingData => write!(formatter, "WCL response contained no data"),
            Self::InvalidResponse(error) => write!(formatter, "WCL response was invalid: {error}"),
        }
    }
}

impl Error for WarcraftLogsError {}

impl From<reqwest::Error> for WarcraftLogsError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for WarcraftLogsError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidResponse(error)
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphqlError>>,
}

pub struct WarcraftLogsClient<'a> {
    http_client: &'a Client,
    config: &'a WarcraftLogsConfig,
}

impl<'a> WarcraftLogsClient<'a> {
    pub fn new(http_client: &'a Client, config: &'a WarcraftLogsConfig) -> Self {
        Self {
            http_client,
            config,
        }
    }

    pub async fn access_token(&self) -> Result<String, WarcraftLogsError> {
        let response = self
            .http_client
            .post(&self.config.oauth_token_url)
            .header(
                reqwest::header::AUTHORIZATION,
                build_basic_auth_value(&self.config.client_id, &self.config.client_secret),
            )
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(WarcraftLogsError::HttpStatus {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        let token = response.json::<TokenResponse>().await?;
        Ok(token.access_token)
    }

    pub async fn query<T>(
        &self,
        token: &str,
        query: &str,
        variables: Value,
    ) -> Result<T, WarcraftLogsError>
    where
        T: DeserializeOwned,
    {
        let response = self
            .http_client
            .post(&self.config.endpoint)
            .bearer_auth(token)
            .json(&json!({
                "query": query,
                "variables": variables,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(WarcraftLogsError::HttpStatus {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        extract_graphql_data(response.json::<Value>().await?)
    }
}

pub fn build_basic_auth_value(client_id: &str, client_secret: &str) -> String {
    let credentials = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
    format!("Basic {credentials}")
}

pub fn extract_graphql_data<T>(json: Value) -> Result<T, WarcraftLogsError>
where
    T: DeserializeOwned,
{
    let response = serde_json::from_value::<GraphqlResponse<T>>(json)?;

    if let Some(errors) = response.errors.filter(|errors| !errors.is_empty()) {
        return Err(WarcraftLogsError::Graphql(
            errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }

    response.data.ok_or(WarcraftLogsError::MissingData)
}
