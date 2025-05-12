use std::env;

use crate::{
    file::{get_token, save_credentials, update_env_file},
    get_base_url,
};
use artisan_middleware::{
    api::roles::Role,
    dusa_collection_utils::{log, core::logger::LogLevel},
};
use owo_colors::OwoColorize;
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

    // First: get user_id
    let response_me = client
        .get(&format!("{}account/me", get_base_url()))
        .bearer_auth(token.clone())
        .send()
        .await?;

    let username = {
        if response_me.status().is_success() {
            let json: serde_json::Value = response_me.json().await?;
            json.get("user_id")
                .and_then(|id| id.as_str())
                .unwrap_or("Unknown")
                .to_string()
        } else {
            log!(LogLevel::Warn, "{}", "Failed to get user ID".yellow());
            String::from("Unknown")
        }
    };

    // Then: get role and expiration
    let response = client
        .post(&format!("{}whoami", get_base_url()))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        if let Some(data) = json.get("you") {
            let role = Role::from_str(data.get("roles").and_then(|v| v.as_str()).unwrap_or("none"));
            let expires = data.get("expires").and_then(|v| v.as_u64()).unwrap_or(0);

            match role {
                Role::Super => {
                    let msg = format!(
                        "Greetings, {}! 🧙 Your role is {}. Your session is valid for 10 minutes.",
                        username.bold(),
                        role.to_str().bright_magenta()
                    );
                    log!(LogLevel::Info, "{}", msg);
                }
                Role::None => {
                    let msg = format!(
                        "YOU HAVE NO POWER HERE. 🧙 You are currently assigned: {}",
                        role.to_str().yellow().bold()
                    );
                    log!(LogLevel::Warn, "{}", msg);
                }
                _ => {
                    let msg = format!(
                        "Hello {}, your role is {} and your token expires in {} seconds.",
                        username.cyan().bold(),
                        role.to_str().green().bold(),
                        expires
                    );
                    log!(LogLevel::Info, "{}", msg);
                }
            }
        }
    } else {
        log!(
            LogLevel::Error,
            "{}",
            format!(
                "Failed to identify user: {}",
                response.text().await.unwrap_or_default()
            )
            .red()
        );
    }

    Ok(())
}

pub async fn login(email: &String, password: &String) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .post(&format!("{}auth/login", get_base_url()))
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        let token = json.get("auth").and_then(|t| t.as_str());
        let refresh = json.get("refresh").and_then(|t| t.as_str());

        let (auth, refresh): (&str, &str) = match (token, refresh) {
            (Some(token), Some(refresh)) => (token, refresh),
            _ => {
                log!(
                    LogLevel::Error,
                    "Failed to parse both refresh and auth token"
                );
                return Err("Login failed".into());
            }
        };

        env::set_var("API_TOKEN", auth);
        env::set_var("REFRESH_TOKEN", refresh);

        update_env_file("API_TOKEN", auth)?;
        update_env_file("REFRESH_TOKEN", refresh)?;

        log!(LogLevel::Info, "Login successful, token acquired.");
        save_credentials(&email, &password)?;
        return Ok(());
    }

    Err("Login failed".into())
}
