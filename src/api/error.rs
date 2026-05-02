use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: &'static str,
    pub details: Map<String, Value>,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    request_id: Option<&'a str>,
    details: &'a Map<String, Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            details: Map::new(),
        }
    }

    pub fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthorized(message: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "auth.unauthorized", message)
    }

    pub fn forbidden(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    pub fn conflict(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn not_found(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub fn bad_gateway(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, code, message)
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal.error",
            "An internal error occurred.",
        )
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                request_id: None,
                details: &self.details,
            },
        })
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        eprintln!("database error: {error}");
        Self::internal()
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        eprintln!("http client error: {error}");
        Self::bad_gateway(
            "auth.discord_profile_fetch_failed",
            "Discord profile fetch failed.",
        )
    }
}

pub fn validation_error() -> ApiError {
    ApiError::bad_request("validation.invalid_request", "Request payload is invalid.")
}

pub fn json_error_handler(
    error: actix_web::error::JsonPayloadError,
    _req: &actix_web::HttpRequest,
) -> actix_web::Error {
    let mut api_error = validation_error();
    api_error
        .details
        .insert("reason".to_string(), json!(error.to_string()));
    api_error.into()
}
