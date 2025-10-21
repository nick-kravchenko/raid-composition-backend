use std::env;

fn main() {
    let port: String = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    
    println!("Server will start on port: {}", port);
}
