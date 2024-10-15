include!(concat!(env!("OUT_DIR"), "/built.rs"));

mod commands;

use clap::{Parser, Subcommand};
use commands::describe::{describe_entries, describe_metadata, describe_node};
use std::fmt::Debug;

use raftify::{
    raft::{default_logger, formatter::set_custom_formatter},
    AbstractLogEntry, AbstractStateMachine, CustomFormatter, Result, StableStorage,
};

use cfmt::formatcp;

const RAFTIFY_VERSION: &str = env!("RAFTIFY_VERSION");
const RAFTIFY_FEATURES: &str = env!("RAFTIFY_FEATURES");
const VERSION_TEXT: &'static str = formatcp!(
    "{PKG_VERSION}
(Built with raftify {}, Enabled features: {})",
    RAFTIFY_VERSION,
    RAFTIFY_FEATURES
);

#[derive(Parser)]
#[command(name = PKG_NAME)]
#[command(author = PKG_AUTHORS)]
#[command(about = PKG_DESCRIPTION)]
#[command(version = VERSION_TEXT)]
#[command(long_version = VERSION_TEXT)]
struct App {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Describe logs, metadata, and raft node information
    #[command(subcommand)]
    Describe(DescribeSubcommands),
}

#[derive(Subcommand)]
enum DescribeSubcommands {
    /// List persisted log entries
    Logs {
        /// The log directory path
        path: String,
        /// Print the output in raw format
        #[arg(long, default_value_t = false)]
        raw: bool,
    },
    /// List persisted metadata
    Metadata {
        /// The log directory path
        path: String,
        /// Print the output in raw format
        #[arg(long, default_value_t = false)]
        raw: bool,
    },
    /// Inspect RaftNode
    Node {
        /// The address of the RaftNode
        address: String,
    },
}

pub async fn cli_handler<
    LogEntry: AbstractLogEntry + Debug + Send + 'static,
    LogStorage: StableStorage + Send + Sync + Clone + 'static,
    FSM: AbstractStateMachine + Debug + Clone + Send + Sync + 'static,
>(
    args: Option<Vec<String>>,
) -> Result<()> {
    let app: App = match args {
        Some(args) => App::parse_from(args),
        None => App::parse(),
    };
    let logger = default_logger();
    set_custom_formatter(CustomFormatter::<LogEntry, FSM>::new());

    match app.command {
        Commands::Describe(x) => match x {
            DescribeSubcommands::Logs {
                path,
                raw: print_raw_format,
            } => {
                describe_entries::<LogStorage>(path.as_str(), logger.clone(), print_raw_format)?;
            }
            DescribeSubcommands::Metadata {
                path,
                raw: print_raw_format,
            } => {
                describe_metadata::<LogStorage>(path.as_str(), logger.clone(), print_raw_format)?;
            }
            DescribeSubcommands::Node { address } => {
                describe_node(address.as_str()).await?;
            }
        },
    }

    Ok(())
}
