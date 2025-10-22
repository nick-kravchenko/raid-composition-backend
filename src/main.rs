use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder };
use std::env;

#[get("/")]
async fn hello() -> impl Responder {
  HttpResponse::Ok().body("Hello, world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
  HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
  HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let app_port: u16 = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080);

  println!("Starting server on port: {}", app_port);

  HttpServer::new(|| {
    App::new()
      .service(hello)
      .service(echo)
      .route("/manual_hello", web::get().to(manual_hello))
  })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}
