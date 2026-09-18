use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "artisan_cli")]
#[command(version = "1.0")]
#[command(about = "Management tool for ArtisanHosting")]
#[command(author = "Darrion Whitfield as Artisan Hosting")]
pub struct Cli {
    #[command(subcommand)]
    pub command: TopLevelCommand,

    // we have 'watch -n.25 artisan_cli' at home
    #[arg(long, short, value_name = "1")]
    pub watch: Option<u64>,
}

#[derive(Subcommand)]
pub enum TopLevelCommand {
    #[command(subcommand)]
    Node(NodeCmd),
    #[command(subcommand)]
    Runner(RunnerCmd),
    #[command(subcommand)]
    Instance(InstanceCmd),
    #[command(subcommand)]
    Auth(AuthCmd),
    #[command(subcommand)]
    GitConfig(GitConfigCmd),
    Logs {
        instance_id: String,
        #[arg(short, long, default_value = "100")]
        lines: u64,
    },
}

#[derive(Subcommand)]
pub enum NodeCmd {
    List,
    Get { node_id: u64 },
    Reload { node_id: u64 },
    WatchdogGet {
        node_id: u64,
        #[arg(long, short)]
        application: String,
        #[arg(long, short)]
        kind: String,
        #[arg(long, short)]
        create_if_missing: bool,
    },
    WatchdogSet {
        node_id: u64,
        #[arg(long, short)]
        application: String,
        #[arg(long, short)]
        kind: String,
        #[arg(long, short)]
        file: Option<String>,
        #[arg(long, short)]
        content: Option<String>,
        #[arg(long, short, default_value = "")]
        expected_previous_sha256: String,
    },
}

#[derive(Subcommand)]
pub enum GitConfigCmd {
    Get {
        node_id: u64,
    },
    Set {
        node_id: u64,
        #[arg(long, short)]
        file: Option<String>,
        #[arg(long, short)]
        json: Option<String>,
    },
    Add {
        node_id: u64,
        #[arg(long)]
        user: String,
        #[arg(long)]
        repo: String,
        #[arg(long)]
        branch: String,
        #[arg(long)]
        server: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        reload: bool,
    },
    Update {
        node_id: u64,
        id: String,
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        reload: bool,
    },
    Remove {
        node_id: u64,
        id: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        reload: bool,
    },
    /// Force a resync/clean of every configured checkout on a node, plus a
    /// purge of any stale `/opt/artisan/tmp` state file left over from a repo
    /// no longer in git.cf. The same thing a write already triggers as a
    /// side effect, just on demand -- doesn't touch git.cf itself.
    Audit {
        node_id: u64,
    },
}

#[derive(Subcommand)]
pub enum RunnerCmd {
    List,
    Details { runner_id: String },
    Usage { runner_id: String },
    Control { runner_id: String, command: String },
    Bill { runner_id: String },
}

#[derive(Subcommand)]
pub enum InstanceCmd {
    Usage { instance_id: String },
}

#[derive(Subcommand)]
pub enum AuthCmd {
    Whoami,
    Discover,
    Login { email: String, password: String },
}
