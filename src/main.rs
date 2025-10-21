use std::env;

fn main() {
    let app_port: String = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string());

    println!("Server will start on port: {}", app_port);
}
