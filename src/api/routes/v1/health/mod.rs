use actix_web::{web, Scope};

pub fn scope() -> Scope {
  web::scope("/health")
    .route("", web::get().to(app::health))
    .route("/postgres", web::get().to(postgres::health))
    .route("/redis", web::get().to(redis::health))
}

fn percent_encode_url_component(value: &str) -> String {
  value.bytes().fold(String::new(), |mut encoded, byte| {
    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
      encoded.push(byte as char);
    } else {
      encoded.push_str(&format!("%{:02X}", byte));
    }

    encoded
  })
}

mod app;
mod postgres;
mod redis;
