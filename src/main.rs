use std::{fs::create_dir_all, time::Duration};

use artisan_middleware::{
    aggregator::{BilledUsageSummary, BillingCosts}, cli::clean_screen, dusa_collection_utils::{
        functions::current_timestamp,
        log,
        logger::{set_log_level, LogLevel},
    }, portal::{
        ApiResponse, CommandResponse, InstanceLogResponse, NodeDetails, NodeInfo, RunnerDetails,
        RunnerHealth, RunnerSummary,
    }, timestamp::format_unix_timestamp
};
use auth::{discover, login, whoami};
use clap::Parser;
use cli::{AuthCmd, Cli, InstanceCmd, NodeCmd, RunnerCmd, TopLevelCommand};
use defs::{BillingEntry, NodeRow, NodeSummaryRow, RunnerInstanceRow, RunnerRow, UsageRow};
use file::get_token;
use formatting::{display_table, format_bytes, print_logs, strip_ansi_codes, style_table};
use owo_colors::OwoColorize;
use reqwest::Client;
use tabled::Table;
use tokio::time::sleep;

mod auth;
mod cli;
mod defs;
mod file;
mod formatting;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    set_log_level(LogLevel::Debug);
    let home_dir = dirs::home_dir().ok_or("Failed to get home directory")?;
    let our_dir = home_dir.join(".artisan_cli");
    let env_file = our_dir.join(".env");

    create_dir_all(&our_dir)?;
    dotenv::from_path(env_file).ok();

    let cli = Cli::parse();

    loop {
        match cli.command {
            TopLevelCommand::Node(ref node_cmd) => match node_cmd {
                NodeCmd::List => list_nodes().await?,
                NodeCmd::Get { node_id } => get_node(&node_id).await?,
            },
            TopLevelCommand::Runner(ref runner_cmd) => match runner_cmd {
                RunnerCmd::List => list_runners().await?,
                RunnerCmd::Details { runner_id } => get_runner_details(&runner_id).await?,
                RunnerCmd::Usage { runner_id } => get_runner_usage(&runner_id).await?,
                RunnerCmd::Control { runner_id, command } => {
                    control_runner(&runner_id, &command).await?
                }
                RunnerCmd::Bill { runner_id } => calculate_billing(&runner_id).await?,
            },
            TopLevelCommand::Instance(ref instance_cmd) => match instance_cmd {
                InstanceCmd::Usage { instance_id } => get_instance_usage(&instance_id).await?,
            },
            TopLevelCommand::Auth(ref auth_cmd) => match auth_cmd {
                AuthCmd::Whoami => whoami().await?,
                AuthCmd::Discover => discover().await?,
                AuthCmd::Login { email, password } => login(email, password).await?,
            },
            TopLevelCommand::Logs { ref instance_id, lines } => show_logs(lines, &instance_id).await?,
        }

        // Only loop if --watch is set
        if let Some(interval) = cli.watch {
            if interval < 30 {
                println!("Woah woah woah, we're fast but we're not that fast!");
                break;
            }
            sleep(Duration::from_millis(interval)).await;
            clean_screen();
        } else {
            break;
        }
    }

    Ok(())
}

async fn show_logs(lines: u64, instance_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let mut line_array = Vec::new();

    let response = client
        .get(&format!("{}logs/{}/{}", get_base_url(), instance_id, lines))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<InstanceLogResponse> = response.json().await?;
        if let Some(log_data) = api_response.data {
            let sorted = log_data.lines;
            let mut e = 1;

            log!(LogLevel::Info, "Runner: {}", log_data.runner_id);
            log!(LogLevel::Info, "Instance: {}", log_data.instance_id);
            sorted.iter().for_each(|entry| {
                let line = format!(
                    "[{:03} of {:03}] @ {} -> {}",
                    e, lines, entry.timestamp, entry.message
                );
                line_array.push(line);
                e += 1;
            });

            print_logs(line_array, format!("{} Logs ('q' to quit)", instance_id))?;
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to list logs: {}",
            response.text().await?
        );
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
            println!();

            let rows = nodes
                .into_iter()
                .map(|node| NodeRow {
                    id: node.identity.id.to_string(),
                    status: strip_ansi_codes(&node.status.to_string()),
                    ip: node.ip_address.to_string(),
                    runner_count: node.runners.len().to_string(),
                    updated: node.last_updated.to_string(),
                })
                .collect::<Vec<_>>();

            let mut table = Table::new(rows);
            table = style_table(&mut table, Some(2), true);
            display_table(&table);
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
            let row = NodeSummaryRow {
                node_id: node.identity.id.to_string(),
                status: strip_ansi_codes(&node.status.to_string()),
                client_apps: node.manager_data.client_apps,
                system_apps: node.manager_data.system_apps,
                hostname: node.manager_data.hostname.to_string(),
                ip_address: node.manager_data.address.to_string(),
                warnings: node.manager_data.warning,
                last_updated: node.last_updated.to_string(),
            };

            let mut table = Table::new(vec![row]);
            table = style_table(&mut table, Some(1), true); // Color status column, center align
            display_table(&table);
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

async fn get_instance_usage(instance_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let url = format!("{}usage/single/{}", get_base_url(), instance_id);

    let response = client.get(&url).bearer_auth(token).send().await?;

    if response.status().is_success() {
        let api_response: ApiResponse<BilledUsageSummary> = response.json().await?;

        // log!(LogLevel::Debug, "{:?}", api_response);

        if let Some(summary) = api_response.data {
            let row = UsageRow {
                runner_id: summary.runner_id.to_string(),
                instance_id: summary.instance_id.to_string(),
                total_cpu: format!("{:.2}", summary.total_cpu),
                peak_cpu: format!("{:.2}%", summary.peak_cpu),
                avg_ram: format!("{:.2} MB", summary.avg_memory),
                peak_ram: format!("{:.2} MB", summary.peak_memory),
                rx: format_bytes(summary.total_rx),
                tx: format_bytes(summary.total_tx),
                samples: summary.total_samples.to_string(),
            };

            let mut table = Table::new(vec![row]);
            table = style_table(&mut table, Some(1), true);
            display_table(&table);
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

async fn get_runner_usage(runner_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let url = format!("{}usage/group/{}", get_base_url(), runner_id);

    let response = client.get(&url).bearer_auth(token).send().await?;

    if response.status().is_success() {
        let api_response: ApiResponse<BilledUsageSummary> = response.json().await?;

        // log!(LogLevel::Debug, "{:?}", api_response);

        if let Some(summary) = api_response.data {
            let row = UsageRow {
                runner_id: summary.runner_id.to_string(),
                instance_id: summary.instance_id.to_string(),
                total_cpu: format!("{:.2}", summary.total_cpu),
                peak_cpu: format!("{:.2}%", summary.peak_cpu),
                avg_ram: format!("{:.2} MB", summary.avg_memory),
                peak_ram: format!("{:.2} MB", summary.peak_memory),
                rx: format_bytes(summary.total_rx),
                tx: format_bytes(summary.total_tx),
                samples: summary.total_samples.to_string(),
            };

            let mut table = Table::new(vec![row]);
            table = style_table(&mut table, Some(1), true);
            display_table(&table);
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

async fn calculate_billing(runner_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let url = format!("{}usage/group/{}", get_base_url(), runner_id);
    let response = client.get(&url).bearer_auth(token).send().await?;

    if response.status().is_success() {
        let api_response: ApiResponse<BilledUsageSummary> = response.json().await?;
        if let Some(summary) = api_response.data {
            log!(LogLevel::Debug, "{:?}", summary);

            match client
                .post(&format!(
                    "{}billing/calculate?instances={}",
                    get_base_url(),
                    summary.instances
                ))
                .json(&summary)
                .send()
                .await
            {
                Ok(response) => {
                    let text = response.text().await?;
                    // println!("Raw response body: {:?}", text);

                    // Optional: try parsing only if it's not empty
                    let api_response: ApiResponse<BillingCosts> = if !text.trim().is_empty() {
                        serde_json::from_str(&text)?
                    } else {
                        log!(LogLevel::Error, "Empty response body from server");
                        std::process::exit(0)
                    };

                    match api_response.data {
                        Some(data) => {
                            let rows = vec![
                                BillingEntry {
                                    label: "RAM Usage".to_string(),
                                    value: format!("${:.2}", data.ram_cost),
                                },
                                BillingEntry {
                                    label: "CPU Usage".to_string(),
                                    value: format!("${:.2}", data.cpu_cost),
                                },
                                BillingEntry {
                                    label: "Bandwidth".to_string(),
                                    value: format!("${:.2}", data.bandwidth_cost),
                                },
                                BillingEntry {
                                    label: "Base Hosting".to_string(),
                                    value: format!("${:.2}", (data.instances * 5)),
                                },
                                // BillingEntry { label: "Total".to_string(), value: format!("{}", format!("${:.2}", data.total_cost).bold().green()) },
                            ];

                            let mut table = Table::new(rows);
                            table = style_table(&mut table, None, true);
                            display_table(&table);
                            log!(LogLevel::Info, "Total: ${:.2}", data.total_cost);
                        }
                        None => {
                            log!(
                                LogLevel::Error,
                                "Invalid response recieved: {}:{:?}",
                                api_response.status,
                                api_response.errors
                            );
                            std::process::exit(0);
                        }
                    }
                }
                Err(err) => log!(
                    LogLevel::Error,
                    "Failed to get bill data: {}",
                    err.to_string()
                ),
            }
        } else {
            log!(LogLevel::Warn, "The server didn't give us usage data.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to fetch usage: {}",
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
            if runners.is_empty() {
                println!("{}", "No runners found.".yellow());
            } else {
                let rows = runners
                    .into_iter()
                    .map(|r| RunnerRow {
                        name: strip_ansi_codes(r.name.replace("ais_", "").trim_ascii()),
                        status: strip_ansi_codes(r.status.to_string().trim_ascii()),
                        uptime: strip_ansi_codes(r.uptime.unwrap_or(0).to_string().trim_ascii()),
                        instances: strip_ansi_codes(r.nodes.len().to_string().trim_ascii()),
                    })
                    .collect::<Vec<_>>();

                let mut table = Table::new(rows);
                table = style_table(&mut table, Some(1), true);
                display_table(&table);
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
            // log!(LogLevel::Info, "Information on {} runner group", runner_id);

            let rows = runners
                .into_iter()
                .map(|runner| {
                    let health = runner.health.unwrap_or_else(|| RunnerHealth {
                        uptime: 0,
                        last_check: current_timestamp(),
                        cpu_usage: "-".into(),
                        ram_usage: "-".into(),
                        tx_bytes: 0,
                        rx_bytes: 0,
                    });

                    let log_len = runner.logs.as_ref().map_or(0, |logs| logs.recent.len());

                    RunnerInstanceRow {
                        id: runner.id.to_string(),
                        status: strip_ansi_codes(&runner.status.to_string()),
                        uptime: health.uptime.to_string(),
                        cpu: health.cpu_usage.to_string(),
                        ram: health.ram_usage.to_string(),
                        rx: format_bytes(health.rx_bytes),
                        tx: format_bytes(health.tx_bytes),
                        log_len: log_len.to_string(),
                    }
                })
                .collect::<Vec<_>>();

            let mut table = Table::new(rows);
            table = style_table(&mut table, Some(1), false);
            display_table(&table);
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
