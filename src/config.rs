use std::env;

/// Application configuration (应用配置)
#[derive(Clone)]
pub struct AppConfig {
    pub auth_token: String,
    pub upload_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let auth_token = env::var("AUTH_TOKEN").expect("AUTH_TOKEN must be set");
        let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());
        
        Self {
            auth_token,
            upload_dir,
        }
    }
}