use std::fs::create_dir_all;

use artisan_middleware::{
    aggregator::BilledUsageSummary, dusa_collection_utils::{functions::current_timestamp, log, logger::{set_log_level, LogLevel}}, portal::{
        ApiResponse, CommandResponse, NodeDetails, NodeInfo, RunnerDetails, RunnerHealth,
        RunnerLogs, RunnerSummary,
    }, timestamp::format_unix_timestamp
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
    /// Get summarized historic usage for a runner instance
    GetRunnerUsage {
        runner_id: String,
        instance_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    set_log_level(LogLevel::Debug);
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
        Commands::GetRunnerUsage {
            runner_id,
            instance_id,
        } => get_runner_usage(&runner_id, &instance_id).await?,
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

async fn get_runner_usage(
    runner_id: &str,
    instance_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let url = format!(
        "{}usage/single/{}/{}",
        get_base_url(),
        runner_id,
        instance_id
    );

    let response = client.get(&url).bearer_auth(token).send().await?;

    if response.status().is_success() {
        let api_response: ApiResponse<BilledUsageSummary> =
            response.json().await?;

        log!(LogLevel::Debug, "{:?}", api_response);

        if let Some(summary) = api_response.data {
            log!(LogLevel::Info, "Runner ID: {}", summary.runner_id);
            log!(LogLevel::Info, "Instance ID: {}", summary.instance_id);
            log!(LogLevel::Info, "Total CPU: {:.2}%", summary.total_cpu);
            log!(LogLevel::Info, "Peak CPU: {:.2}%", summary.peak_cpu);
            log!(LogLevel::Info, "Avg RAM: {:.2} MB", summary.avg_memory);
            log!(LogLevel::Info, "Peak RAM: {:.2} MB", summary.peak_memory);
            log!(
                LogLevel::Info,
                "Data In: {}",
                format_bytes(summary.total_rx)
            );
            log!(
                LogLevel::Info,
                "Data Out: {}",
                format_bytes(summary.total_tx)
            );
            log!(
                LogLevel::Info,
                "Samples Collected: {}",
                summary.total_samples
            );
        } else {
            log!(LogLevel::Warn, "No usage summary found.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to get usage: {}",
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
                        tx_bytes: 0,
                        rx_bytes: 0,
                    }
                };

                let log_data = if let Some(log) = runner.logs {
                    log
                } else {
                    RunnerLogs { recent: Vec::new() }
                };

                let data = format!(
                    "Instance Id: {}, Status: {}, Uptime: {}, Cpu Usage: {}, Ram Usage: {}, Data In: {}, Data Out: {}, Log Legnth: {}",
                    runner.id,
                    runner.status,
                    health_data.uptime,
                    health_data.cpu_usage,
                    health_data.ram_usage,
                    format_bytes(health_data.rx_bytes),
                    format_bytes(health_data.tx_bytes),
                    log_data.recent.len(),
                );
                log!(LogLevel::Info, "{}", data);
            }
        } else {
            log!(LogLevel::Error, "Runner not found.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to get runner details: {}",
            response.text().await?
        );
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
                    format_unix_timestamp(data.queued_at)
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
    "https://api.artisanhosting.net/v1/"
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let bytes_f64 = bytes as f64;

    if bytes_f64 >= TB {
        format!("{:.2} TB", bytes_f64 / TB)
    } else if bytes_f64 >= GB {
        format!("{:.2} GB", bytes_f64 / GB)
    } else if bytes_f64 >= MB {
        format!("{:.2} MB", bytes_f64 / MB)
    } else if bytes_f64 >= KB {
        format!("{:.2} KB", bytes_f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}
