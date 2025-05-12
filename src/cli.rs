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
    Logs {
        instance_id: String,
        #[arg(short, long, default_value = "100")]
        lines: u64,
    },
}

#[derive(Subcommand)]
pub enum NodeCmd {
    List,
    Get { node_id: String },
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
