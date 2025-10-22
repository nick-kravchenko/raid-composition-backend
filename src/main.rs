use actix_web::{ App, HttpServer };

mod routes;
mod config;
mod db;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let app_port: u16 = config::config().app_port;

  println!("Starting server on port: {}", app_port);

  HttpServer::new(|| {
    App::new()
      .service(routes::api_v1())
  })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}
