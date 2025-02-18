use std::fs::create_dir_all;

use artisan_middleware::{
    dusa_collection_utils::{functions::current_timestamp, log, logger::LogLevel},
    portal::{
        ApiResponse, CommandResponse, NodeDetails, NodeInfo, RunnerDetails, RunnerHealth,
        RunnerLogs, RunnerSummary,
    },
    timestamp::really_format_unix_timestamp,
};
use auth::{discover, login, whoami};
use clap::{Parser, Subcommand};
use file::get_token;
use reqwest::Client;

mod auth;
mod file;

#[derive(Parser)]
#[clap(name = "artisan_cli", version = "1.0", author = "Artisan Hosting")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all nodes in the system
    ListNodes,
    /// Get details of a specific node
    GetNode { node_id: String },
    /// List all runners a user has access to
    ListRunners,
    /// Get details of a specific runner
    GetRunnerDetails { runner_id: String },
    /// Control a runner by sending a command (e.g., start, stop, restart)
    ControlRunner { runner_id: String, command: String },
    /// Discover the current node execution duration
    Discover,
    /// Identify the current user
    WhoAmI,
    /// Authenticate with the server
    Login { username: String, password: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    create_dir_all(&home_dir.join(".artisan_cli"))?;
    let env_file = home_dir.join(".artisan_cli/.env");

    dotenv::from_path(env_file).ok();
    
    let cli = Cli::parse();

    match cli.command {
        Commands::ListNodes => list_nodes().await?,
        Commands::GetNode { node_id } => get_node(&node_id).await?,
        Commands::ListRunners => list_runners().await?,
        Commands::GetRunnerDetails { runner_id } => get_runner_details(&runner_id).await?,
        Commands::ControlRunner { runner_id, command } => {
            control_runner(&runner_id, &command).await?
        }
        Commands::Login { username, password } => login(username, password).await?,
        Commands::Discover => discover().await?,
        Commands::WhoAmI => whoami().await?,
    }

    Ok(())
}


async fn list_nodes() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}nodes", get_base_url()))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<Vec<NodeInfo>> = response.json().await?;
        if let Some(nodes) = api_response.data {
            for node in nodes {
                let data = format!(
                    "Node Id: {}, Node Status: {}, Node Ip: {}, Runners in config: {}, Last Updated: {}",
                    node.identity.id,
                    node.status,
                    node.ip_address,
                    node.runners.len(),
                    node.last_updated,
                );
                log!(LogLevel::Info, "{}", data);
            }
        } else {
            log!(
                LogLevel::Error,
                "Get a drink. Currently there are no nodes registered"
            );
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to list nodes: {}",
            response.text().await?
        );
    }

    Ok(())
}

async fn get_node(node_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}node/{}", get_base_url(), node_id))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<NodeDetails> = response.json().await?;
        if let Some(node) = api_response.data {
            let data = format!(
                "Node Id: {}, Node Status: {}, Client Apps: {}, System Apps: {}, Hostname: {}, Ip Addr: {}, Runner Errors: {}, Last Updated @ {}",
                node.identity.id,
                node.status,
                node.manager_data.client_apps,
                node.manager_data.system_apps,
                node.manager_data.hostname,
                node.manager_data.address,
                node.manager_data.warning,
                node.last_updated
            );
            log!(LogLevel::Info, "{}", data);
        } else {
            log!(LogLevel::Info, "Node not found.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to get node details: {}",
            response.text().await?
        );
    }

    Ok(())
}

async fn list_runners() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}runners", get_base_url()))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<Vec<RunnerSummary>> = response.json().await?;
        if let Some(runners) = api_response.data {
            for runner in runners {
                let data: String = format!(
                    "Name: {}, Status: {}, Uptime: {}, Instances: {}",
                    runner.name.replace("ais_", ""),
                    runner.status,
                    runner.uptime.unwrap_or(0),
                    runner.nodes.len()
                );
                log!(LogLevel::Info, "{}", data);
            }
        } else {
            log!(LogLevel::Error, "No runners found");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to list runners: {}",
            response.text().await?
        );
    }

    Ok(())
}

async fn get_runner_details(runner_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}runner/{}", get_base_url(), runner_id))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<Vec<RunnerDetails>> = response.json().await?;
        if let Some(runners) = api_response.data {
            log!(LogLevel::Info, "Information on {} runner group", runner_id);
            for runner in runners {
                let health_data = if let Some(health) = runner.health {
                    health
                } else {
                    RunnerHealth {
                        uptime: 0,
                        last_check: current_timestamp(),
                        cpu_usage: "-".into(),
                        ram_usage: "-".into(),
                    }
                };

                let log_data = if let Some(log) = runner.logs {
                    log
                } else {
                    RunnerLogs { recent: Vec::new() }
                };

                let data = format!(
                    "Instance Id: {}, Status: {}, Uptime: {}, Cpu Usage: {}, Ram Usage: {}, Log Legnth: {}",
                    runner.id,
                    runner.status,
                    health_data.uptime,
                    health_data.cpu_usage,
                    health_data.ram_usage,
                    log_data.recent.len(),
                );
                log!(LogLevel::Info, "{}", data);
            }
        } else {
            log!(LogLevel::Error, "Runner not found.");
        }
    } else {
        log!(LogLevel::Error, "Failed to get runner details: {}", response.text().await?);
    }

    Ok(())
}

async fn control_runner(runner_id: &str, command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!(
            "{}control/{}/{}",
            get_base_url(),
            runner_id,
            command
        ))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<CommandResponse> = response.json().await?;
        if let Some(data) = api_response.data {
            let name = if data.runner_id == "general".to_owned() {
                format!("{} runner group", runner_id)
            } else {
                format!("{}", runner_id)
            };

            if api_response.errors.len() > 0 {
                for err in api_response.errors {
                    log!(LogLevel::Error, "{:?}: {}", err.code, err.message);
                }
            } else {
                log!(
                    LogLevel::Info,
                    "Executed: {} on {} @ {}",
                    data.command,
                    name,
                    really_format_unix_timestamp(data.queued_at)
                )
            }
        } else {
            log!(
                LogLevel::Warn,
                "Something may have went wrong, here's some json: {:?}",
                api_response
            );
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to control runner: {}",
            response.text().await?
        );
    }

    Ok(())
}

fn get_base_url() -> &'static str {
    "https://api.arhst.net/v1/"
}
