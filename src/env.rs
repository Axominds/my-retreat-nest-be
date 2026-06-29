use dotenvy::dotenv;
use std::{env, path::PathBuf};

pub struct Env {
    pub server_host: String,
    pub server_port: String,
    pub database_url: String,
    pub password_salt: String,
    pub jwt_access_key: String,
    pub jwt_access_lifetime_in_min: u64,
    pub jwt_refresh_key: String,
    pub jwt_refresh_lifetime_in_min: u64,
    pub upload_dir: PathBuf,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub app_url: String,
}

impl Env {
    pub fn load() -> Self {
        // Load .env file (if present)
        dotenv().ok();

        Self {
            server_host: env::var("SERVER_HOST").expect("SERVER_HOST not set"),
            server_port: env::var("SERVER_PORT").expect("SERVER_PORT not set"),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL not set"),
            password_salt: env::var("PASSWORD_SALT").expect("PASSWORD_SALT not set"),
            jwt_access_key: env::var("JWT_ACCESS_KEY").expect("JWT_ACCESS_KEY not set"),
            jwt_access_lifetime_in_min: env::var("JWT_ACCESS_LIFETIME_IN_MIN")
                .expect("JWT_ACCESS_LIFETIME_IN_MIN not set")
                .parse::<u64>()
                .expect("JWT_ACCESS_LIFETIME_IN_MIN must be a valid integer"),
            jwt_refresh_key: env::var("JWT_REFRESH_KEY").expect("JWT_REFRESH_KEY not set"),
            jwt_refresh_lifetime_in_min: env::var("JWT_REFRESH_LIFETIME_IN_MIN")
                .expect("JWT_REFRESH_LIFETIME_IN_MIN not set")
                .parse::<u64>()
                .expect("JWT_REFRESH_LIFETIME_IN_MIN must be a valid integer"),
            upload_dir: env::var("UPLOAD_DIR")
                .expect("UPLOAD_DIR not set")
                .parse::<PathBuf>()
                .expect("UPLOAD_DIR must be a valid path"),
            smtp_host: env::var("SMTP_HOST").expect("SMTP_HOST not set"),
            smtp_port: env::var("SMTP_PORT")
                .expect("SMTP_PORT not set")
                .parse::<u16>()
                .expect("SMTP_PORT must be a valid port number"),
            smtp_username: env::var("SMTP_USERNAME").expect("SMTP_USERNAME not set"),
            smtp_password: env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set"),
            app_url: env::var("APP_URL").expect("APP_URL not set"),
        }
    }
}

pub static ENV: once_cell::sync::Lazy<Env> = once_cell::sync::Lazy::new(Env::load);
