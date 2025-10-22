use actix_web::{App, HttpServer };
use std::env;

mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let app_port: u16 = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080);

  println!("Starting server on port: {}", app_port);

  HttpServer::new(|| {
    App::new()
      .service(routes::api_v1())
  })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}
