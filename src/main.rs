use std::{fs::create_dir_all, time::Duration};

use artisan_middleware::{
    aggregator::{BilledUsageSummary, BillingCosts}, cli::clean_screen, dusa_collection_utils::{
        core::functions::current_timestamp,
        log,
        core::logger::{set_log_level, LogLevel},
    }, portal::{
        ApiResponse, CommandResponse, ErrorInfo, InstanceLogResponse, NodeDetails, NodeInfo,
        RunnerDetails, RunnerHealth, RunnerSummary, NodeReloadResult,
    }, timestamp::format_unix_timestamp
};
use auth::{discover, login, whoami};
use clap::Parser;
use cli::{AuthCmd, Cli, InstanceCmd, NodeCmd, RunnerCmd, TopLevelCommand, GitConfigCmd};
use defs::{BillingEntry, NodeRow, NodeSummaryRow, RunnerInstanceRow, RunnerRow, UsageRow, GitRepoRow};
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
                NodeCmd::Get { node_id } => get_node(*node_id).await?,
                NodeCmd::Reload { node_id } => reload_node(*node_id).await?,
                NodeCmd::WatchdogGet { node_id, application, kind, create_if_missing } => {
                    get_watchdog_config(*node_id, application, kind, *create_if_missing).await?
                }
                NodeCmd::WatchdogSet { node_id, application, kind, file, content, expected_previous_sha256 } => {
                    set_watchdog_config(*node_id, application, kind, file.as_deref(), content.as_deref(), expected_previous_sha256).await?
                }
            },
            TopLevelCommand::GitConfig(ref git_cmd) => handle_git_config(git_cmd).await?,
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
        } else {
            report_empty_data(&api_response.errors, "No logs returned for that instance.");
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
            report_empty_data(
                &api_response.errors,
                "Get a drink. Currently there are no nodes registered",
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

async fn get_node(node_id: u64) -> Result<(), Box<dyn std::error::Error>> {
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
            report_empty_data(&api_response.errors, "Node not found.");
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
            report_empty_data(&api_response.errors, "No usage summary found.");
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
            report_empty_data(&api_response.errors, "No usage summary found.");
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
            report_empty_data(&api_response.errors, "No runners found.");
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
            report_empty_data(&api_response.errors, "Runner not found.");
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

/// Reports a response whose `data` came back null.
///
/// The portal answers a failure with HTTP 200 and an envelope carrying the
/// reason in `errors`, so a null `data` is as likely to be a real error as an
/// empty result. Printing `empty_message` unconditionally turned "the org
/// lookup timed out" into "no runners found", which sends you looking in the
/// wrong place; say what the server actually said whenever it said anything.
fn report_empty_data(errors: &[ErrorInfo], empty_message: &str) {
    if errors.is_empty() {
        log!(LogLevel::Warn, "{}", empty_message);
        return;
    }

    for error in errors {
        log!(LogLevel::Error, "{:?}: {}", error.code, error.message);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LocalRepoEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub user: String,
    pub repo: String,
    pub branch: String,
    pub server: artisan_middleware::git_actions::GitServer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LocalReposEnvelope {
    pub schema: u32,
    pub hostname: Option<String>,
    pub exported_at: Option<u64>,
    pub path: Option<String>,
    pub repos: Vec<LocalRepoEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LocalReloadOutcome {
    pub attempted: bool,
    pub ok: bool,
    pub method: String,
    pub message: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LocalMovedId {
    pub old_id: String,
    pub new_id: String,
    pub note: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LocalReposResponse {
    #[serde(flatten)]
    pub envelope: LocalReposEnvelope,
    pub reload: LocalReloadOutcome,
    pub moved: Option<LocalMovedId>,
}

async fn reload_node(node_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}node_reload/{}", get_base_url(), node_id))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<NodeReloadResult> = response.json().await?;
        if let Some(data) = api_response.data {
            if data.reloaded {
                log!(LogLevel::Info, "Node {} reloaded successfully.", node_id);
            } else {
                log!(LogLevel::Warn, "Node {} reload returned false.", node_id);
            }
        } else {
            report_empty_data(&api_response.errors, "Node reload response has no data.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to reload node: {}",
            response.text().await?
        );
    }
    Ok(())
}

async fn handle_git_config(git_cmd: &GitConfigCmd) -> Result<(), Box<dyn std::error::Error>> {
    match git_cmd {
        GitConfigCmd::Get { node_id } => {
            get_git_config(*node_id).await?;
        }
        GitConfigCmd::Set { node_id, file, json } => {
            set_git_config(*node_id, file.as_deref(), json.as_deref()).await?;
        }
        GitConfigCmd::Add { node_id, user, repo, branch, server, token, reload } => {
            add_git_config(*node_id, user, repo, branch, server, token.as_deref(), *reload).await?;
        }
        GitConfigCmd::Update { node_id, id, user, repo, branch, server, token, reload } => {
            update_git_config(*node_id, id, user.as_deref(), repo.as_deref(), branch.as_deref(), server.as_deref(), token.as_deref(), *reload).await?;
        }
        GitConfigCmd::Remove { node_id, id, reload } => {
            remove_git_config(*node_id, id, *reload).await?;
        }
        GitConfigCmd::Audit { node_id } => {
            audit_git_config(*node_id).await?;
        }
    }
    Ok(())
}

async fn get_git_config(node_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .get(&format!("{}node/{}/git-config", get_base_url(), node_id))
        .bearer_auth(token)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<LocalReposResponse> = response.json().await?;
        if let Some(res) = api_response.data {
            println!();
            if res.envelope.repos.is_empty() {
                println!("{}", "No git repos configured on this node.".yellow());
            } else {
                let rows = res.envelope.repos
                    .into_iter()
                    .map(|r| GitRepoRow {
                        id: r.id.unwrap_or_default(),
                        user: r.user,
                        repo: r.repo,
                        branch: r.branch,
                        server: format_git_server(&r.server),
                    })
                    .collect::<Vec<_>>();

                let mut table = Table::new(rows);
                table = style_table(&mut table, Some(1), true);
                display_table(&table);
            }
        } else {
            report_empty_data(&api_response.errors, "No git config data found.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to get git config: {}",
            response.text().await?
        );
    }
    Ok(())
}

async fn set_git_config(
    node_id: u64,
    file: Option<&str>,
    json_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let body_value: serde_json::Value = match (file, json_str) {
        (Some(f), _) => {
            let content = std::fs::read_to_string(f)?;
            serde_json::from_str(&content)?
        }
        (_, Some(j)) => {
            serde_json::from_str(j)?
        }
        _ => {
            log!(LogLevel::Error, "Either --file or --json must be provided.");
            return Ok(());
        }
    };

    let request_payload = serde_json::json!({
        "op": "set",
        "schema": body_value.get("schema").unwrap_or(&serde_json::json!(1)),
        "repos": body_value.get("repos").unwrap_or(&body_value),
    });

    send_git_config_request(node_id, request_payload).await
}

async fn add_git_config(
    node_id: u64,
    user: &str,
    repo: &str,
    branch: &str,
    server: &str,
    token: Option<&str>,
    reload: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let git_server = parse_git_server(server);
    let mut payload = serde_json::json!({
        "op": "add",
        "user": user,
        "repo": repo,
        "branch": branch,
        "server": git_server,
        "reload": reload,
    });

    if let Some(t) = token {
        payload["token"] = serde_json::json!(t);
    }

    send_git_config_request(node_id, payload).await
}

async fn update_git_config(
    node_id: u64,
    id: &str,
    user: Option<&str>,
    repo_name: Option<&str>,
    branch: Option<&str>,
    server: Option<&str>,
    token: Option<&str>,
    reload: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let bearer_token = get_token().await?;

    let response = client
        .get(&format!("{}node/{}/git-config", get_base_url(), node_id))
        .bearer_auth(bearer_token)
        .send()
        .await?;

    if !response.status().is_success() {
        log!(
            LogLevel::Error,
            "Failed to fetch current git config for update: {}",
            response.text().await?
        );
        return Ok(());
    }

    let api_response: ApiResponse<LocalReposResponse> = response.json().await?;
    let mut current_repos = match api_response.data {
        Some(res) => res.envelope.repos,
        None => {
            log!(LogLevel::Error, "No current git config found on this node.");
            return Ok(());
        }
    };

    let index = match current_repos.iter().position(|r| r.id.as_deref() == Some(id)) {
        Some(idx) => idx,
        None => {
            log!(LogLevel::Error, "No repository found with ID: {}", id);
            return Ok(());
        }
    };

    let mut repo_to_update = current_repos.remove(index);

    if let Some(u) = user {
        repo_to_update.user = u.to_string();
    }
    if let Some(r) = repo_name {
        repo_to_update.repo = r.to_string();
    }
    if let Some(b) = branch {
        repo_to_update.branch = b.to_string();
    }
    if let Some(s) = server {
        repo_to_update.server = parse_git_server(s);
    }
    if let Some(t) = token {
        repo_to_update.token = if t.is_empty() { None } else { Some(t.to_string()) };
    }

    let payload = serde_json::json!({
        "op": "update",
        "id": id,
        "repo": repo_to_update,
        "reload": reload,
    });

    send_git_config_request(node_id, payload).await
}

async fn remove_git_config(
    node_id: u64,
    id: &str,
    reload: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = serde_json::json!({
        "op": "remove",
        "id": id,
        "reload": reload,
    });

    send_git_config_request(node_id, payload).await
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LocalAuditOutcome {
    pub repos_considered: usize,
    pub stale_checkouts_removed: usize,
    pub stale_state_files_removed: usize,
    pub errors: Vec<String>,
}

async fn audit_git_config(node_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .post(&format!("{}node/{}/git-config", get_base_url(), node_id))
        .bearer_auth(token)
        .json(&serde_json::json!({ "op": "audit" }))
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<LocalAuditOutcome> = response.json().await?;
        if let Some(outcome) = api_response.data {
            log!(
                LogLevel::Info,
                "Audit complete: {} repo(s) considered, {} stale checkout(s) removed, {} stale state file(s) removed",
                outcome.repos_considered,
                outcome.stale_checkouts_removed,
                outcome.stale_state_files_removed
            );
            for err in &outcome.errors {
                log!(LogLevel::Warn, "Audit reported: {}", err);
            }
        } else {
            report_empty_data(&api_response.errors, "No audit result returned.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to run git repo audit: {}",
            response.text().await?
        );
    }
    Ok(())
}

async fn send_git_config_request(
    node_id: u64,
    payload: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let response = client
        .post(&format!("{}node/{}/git-config", get_base_url(), node_id))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<LocalReposResponse> = response.json().await?;
        if let Some(res) = api_response.data {
            log!(LogLevel::Info, "Git config operation completed successfully.");
            log!(
                LogLevel::Info,
                "Reload Attempted: {}, Success: {}, Method: {}, Message: {:?}",
                res.reload.attempted,
                res.reload.ok,
                res.reload.method,
                res.reload.message
            );
            if let Some(moved) = res.moved {
                log!(
                    LogLevel::Info,
                    "Moved ID: {} -> {}. Note: {}",
                    moved.old_id,
                    moved.new_id,
                    moved.note
                );
            }
        } else {
            report_empty_data(
                &api_response.errors,
                "Operation succeeded but response had no data.",
            );
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to execute git config command: {}",
            response.text().await?
        );
    }
    Ok(())
}

fn format_git_server(server: &artisan_middleware::git_actions::GitServer) -> String {
    match server {
        artisan_middleware::git_actions::GitServer::GitHub => "GitHub".to_string(),
        artisan_middleware::git_actions::GitServer::GitLab => "GitLab".to_string(),
        artisan_middleware::git_actions::GitServer::Custom(url) => format!("Custom({})", url),
    }
}

fn parse_git_server(server: &str) -> artisan_middleware::git_actions::GitServer {
    match server.to_lowercase().as_str() {
        "github" => artisan_middleware::git_actions::GitServer::GitHub,
        "gitlab" => artisan_middleware::git_actions::GitServer::GitLab,
        _ => artisan_middleware::git_actions::GitServer::Custom(server.to_string()),
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct GetWatchdogConfigQuery {
    pub application: String,
    pub kind: String,
    pub create_if_missing: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SetWatchdogConfigRequest {
    pub application: String,
    pub kind: String,
    pub content: String,
    pub expected_previous_sha256: String,
}

async fn get_watchdog_config(
    node_id: u64,
    application: &str,
    kind: &str,
    create_if_missing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let token = get_token().await?;

    let query = GetWatchdogConfigQuery {
        application: application.to_string(),
        kind: kind.to_string(),
        create_if_missing,
    };

    let response = client
        .get(&format!("{}node/{}/watchdog/config", get_base_url(), node_id))
        .bearer_auth(token)
        .query(&query)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<serde_json::Value> = response.json().await?;
        if let Some(data) = api_response.data {
            println!("{}", serde_json::to_string_pretty(&data)?);
        } else {
            report_empty_data(&api_response.errors, "Watchdog config response has no data.");
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to get watchdog config: {}",
            response.text().await?
        );
    }
    Ok(())
}

async fn set_watchdog_config(
    node_id: u64,
    application: &str,
    kind: &str,
    file: Option<&str>,
    content: Option<&str>,
    expected_previous_sha256: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw_content = match (file, content) {
        (Some(f), _) => std::fs::read_to_string(f)?,
        (_, Some(c)) => c.to_string(),
        _ => {
            log!(LogLevel::Error, "Either --file or --content must be provided.");
            return Ok(());
        }
    };

    let client = Client::new();
    let token = get_token().await?;

    let payload = SetWatchdogConfigRequest {
        application: application.to_string(),
        kind: kind.to_string(),
        content: raw_content,
        expected_previous_sha256: expected_previous_sha256.to_string(),
    };

    let response = client
        .post(&format!("{}node/{}/watchdog/config", get_base_url(), node_id))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let api_response: ApiResponse<serde_json::Value> = response.json().await?;
        if let Some(data) = api_response.data {
            log!(LogLevel::Info, "Watchdog config updated successfully.");
            println!("{}", serde_json::to_string_pretty(&data)?);
        } else {
            report_empty_data(
                &api_response.errors,
                "Watchdog config update returned success but had no data.",
            );
        }
    } else {
        log!(
            LogLevel::Error,
            "Failed to set watchdog config: {}",
            response.text().await?
        );
    }
    Ok(())
}
