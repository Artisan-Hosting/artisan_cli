use std::env;

use crate::{file::{get_token, save_credentials, update_env_file}, get_base_url};
use artisan_middleware::{api::roles::Role, dusa_collection_utils::{log, logger::LogLevel}};
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

    let response_me = client
        .get(&format!("{}account/me", get_base_url()))
        .bearer_auth(token.clone())
        .send()
        .await?;

    
    let username = {
        if response_me.status().is_success() {
            let json: serde_json::Value = response_me.json().await?;
            if let Some(data) = json.get("user_id") {
                match data.as_str() {
                    Some(name) => name.to_string(),
                    None => {
                        // log!(LogLevel::Warn, "{:?}", response_me.text().await);
                        String::from("Unknown: failed to parse")
                    },
                }
            } else {
                // log!(LogLevel::Warn, "{:?}", response_me.text().await);
                String::from("Unknown: No userid")
            }
        } else {
            log!(LogLevel::Warn, "{:?}", response_me.text().await);
            String::from("Unknown")
        }
    };

    let response = client
        .post(&format!("{}whoami", get_base_url()))
        .bearer_auth(token.clone())
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        if let Some(data) = json.get("you") {
            let role = Role::from_str(data.get("roles").and_then(|v| v.as_str()).unwrap_or("none"));
            let expires = data.get("expires").and_then(|v| v.as_u64()).unwrap_or(0);

            if role == Role::Super {
                log!(LogLevel::Info, "Greetings: {}! For your security your sessions are only valid for 10 mins at a time. Good Luck", username);
            } else if role == Role::None {
                log!(LogLevel::Info, "YOU HAVE NO POWER HERE. Seriously tho.. you have an assigned role of: {}. You should talk to someone about that", role.to_str());
            } else {
                log!(LogLevel::Info, "Hi {}! for security your default role is: \"{}\" unless specified by a runner_group. Your token expires in: {} seconds", username, role.to_str(), expires);
            }
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

pub async fn login(email: String, password: String) -> Result<(), Box<dyn std::error::Error>> {
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
                log!(LogLevel::Error, "Failed to parse both refresh and auth token");
                return Err("Login failed".into())
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