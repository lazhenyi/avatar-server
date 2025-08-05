#![allow(dead_code)]

pub mod config;
pub mod auth;
pub mod handlers;

use actix_web::{middleware, web, App, HttpServer};
use handlebars::Handlebars;
use std::{env, fs};
use auth::AuthMiddleware;
use config::AppConfig;
use handlers::*;
const INDEX:&str = include_str!("index.html");
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let auth_token = env::var("AUTH_TOKEN").expect("AUTH_TOKEN must be set");
    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    fs::create_dir_all(&upload_dir)?;
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string(
            "stats",
            INDEX,
        )
        .expect("Failed to register template");

    println!("Starting server at http://{}:{}", host, port);
    println!("Using authentication token from environment variable");
    println!("Statistics page available at http://{}:{}/stats", host, port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppConfig {
                auth_token: auth_token.clone(),
                upload_dir: upload_dir.clone(),
            }))
            .app_data(web::Data::new(handlebars.clone()))
            .wrap(middleware::Logger::default())
            .wrap(AuthMiddleware::new(auth_token.clone()))  // 添加认证中间件
            .service(get_avatar)
            .service(upload_avatar)
            .service(get_stats)
    })
        .bind((host, port))?
        .run()
        .await
}