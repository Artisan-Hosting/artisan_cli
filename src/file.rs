use std::fs::create_dir_all;
use std::{env, fs};

use artisan_middleware::dusa_collection_utils::log;
use artisan_middleware::dusa_collection_utils::logger::LogLevel;
use artisan_middleware::encryption::{simple_decrypt, simple_encrypt};
use artisan_middleware::timestamp::current_timestamp;
use reqwest::Client;
use serde_json::json;

use crate::auth::login;
use crate::get_base_url;

pub fn save_credentials(email: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let credentials_dir = home_dir.join(".artisan_cli");
    create_dir_all(&credentials_dir)?;

    let credentials_file = credentials_dir.join("credentials.ejson");
    let credentials = serde_json::json!({ "email": email, "password": password }).to_string();
    let encrypted_credentials = simple_encrypt(credentials.as_bytes())
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
    fs::write(credentials_file, encrypted_credentials.to_string())?;

    log!(LogLevel::Info, "Credentials saved successfully.");
    Ok(())
}

fn load_credentials() -> Result<(String, String), Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let mut credentials_file = home_dir.join(".artisan_cli");
    credentials_file = credentials_file.join("credentials.ejson");

    if credentials_file.exists() {
        let encrypted_data = fs::read_to_string(credentials_file)?;
        let data = simple_decrypt(encrypted_data.as_bytes())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        let json: serde_json::Value = serde_json::from_slice(&data)?;
        let email = json
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let password = json
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        Ok((email, password))
    } else {
        Err("Credentials file not found".into())
    }
}

pub async fn get_token() -> Result<String, Box<dyn std::error::Error>> {
    let token = env::var("API_TOKEN").unwrap_or_default();
    if token.is_empty() {
        log!(LogLevel::Warn, "Token not found, please log in.");
        return Err("Token not found.".into());
    }

    // Decode the token to check expiration
    let token_data: Vec<&str> = token.split('.').collect();
    if token_data.len() == 3 {
        let claims = base64::decode_config(token_data[1], base64::URL_SAFE)?;
        let claims: serde_json::Value = serde_json::from_slice(&claims)?;
        if let Some(exp) = claims.get("exp").and_then(|v| v.as_u64()) {
            let current_time = current_timestamp();
            if exp < current_time {
                log!(LogLevel::Info, "Token expired, refreshing...");
                let refresh_token = env::var("REFRESH_TOKEN").unwrap_or_default();
                let request_body = json!({
                    "expired_token": token,
                    "refresh_token": refresh_token
                });

                let response = Client::new()
                    .post(&format!("{}auth/refresh", get_base_url()))
                    .json(&request_body)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let json: serde_json::Value = response.json().await?;
                    if let Some(new_token) = json.get("auth").and_then(|t| t.as_str()) {
                        update_env_file("API_TOKEN", new_token)?;
                        env::set_var("API_TOKEN", new_token);
                        return env::var("API_TOKEN")
                            .map_err(|_| "Failed to refresh token.".into());
                    }
                } else {
                    log!(LogLevel::Warn, "Failed to refresh session, logging back in");
                    let (email, password) = load_credentials()?;
                    login(&email, &password).await?;
                    return env::var("API_TOKEN").map_err(|_| "Failed to refresh token.".into());
                }
            }
        }
    }

    Ok(token)
}

pub fn update_env_file(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let mut env_path = home_dir.join(".artisan_cli");
    env_path = env_path.join(".env");

    let mut content = if std::path::Path::new(&env_path).exists() {
        fs::read_to_string(env_path.clone())?
    } else {
        String::new()
    };

    let new_line = format!("{}={}\n", key, value);
    if content.contains(&format!("{}=", key)) {
        content = content
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{}=", key)) {
                    new_line.clone()
                } else {
                    line.to_string() + "\n"
                }
            })
            .collect();
    } else {
        content.push_str(&new_line);
    }

    fs::write(env_path, content)?;
    log!(LogLevel::Info, "{} updated in .env file.", key);
    Ok(())
}
