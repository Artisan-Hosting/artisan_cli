use std::env;

use crate::{file::{get_token, save_credentials, update_env_file}, get_base_url};
use artisan_middleware::dusa_collection_utils::{log, logger::LogLevel};
use reqwest::Client;

pub async fn discover() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}discover", get_base_url()))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        log!(LogLevel::Info, "Ok !");
    } else {
        log!(
            LogLevel::Error,
            "Failed to discover: {}",
            response.text().await?
        );
    }

    Ok(())
}

pub async fn whoami() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .post(&format!("{}whoami", get_base_url()))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;

        if let Some(data) = json.get("you") {
            let username = data
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let roles = data.get("roles").and_then(|v| v.as_str()).unwrap_or("none");
            let expires = data.get("expires").and_then(|v| v.as_u64()).unwrap_or(0);

            log!(LogLevel::Info, "Hi {}! for security your default role is: \"{}\" unless specified by a runner_group. Your token expires in: {} seconds", username, roles, expires);
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to identify user: {}",
            response.text().await?
        );
    }

    Ok(())
}

pub async fn login(username: String, password: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post(&format!("{}login", get_base_url()))
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        if let Some(token) = json.get("token").and_then(|t| t.as_str()) {
            env::set_var("API_TOKEN", token);
            update_env_file("API_TOKEN", token)?; // Update the .env file
            log!(LogLevel::Info, "Login successful, token acquired.");
            save_credentials(&username, &password)?;
            return Ok(());
        }
    }

    Err("Login failed".into())
}