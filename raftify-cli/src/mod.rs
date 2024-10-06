include!(concat!(env!("OUT_DIR"), "/built.rs"));

mod commands;

use clap::{Args, Parser, Subcommand};
use commands::debug::{debug_entries, debug_node, debug_persisted, debug_persisted_all};
use std::fmt::Debug;

use raftify::{
    raft::{default_logger, formatter::set_custom_formatter},
    AbstractLogEntry, AbstractStateMachine, CustomFormatter, Result, StableStorage,
};

#[derive(Parser)]
#[command(name = "raftify")]
#[command(version = PKG_VERSION)]
#[command(author = PKG_AUTHORS)]
#[command(about = PKG_DESCRIPTION)]
struct App {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Debug tools
    #[command(subcommand)]
    Debug(DebugSubcommands),
}

#[derive(Subcommand)]
enum DebugSubcommands {
    /// List persisted log entries and metadata
    Persisted {
        /// The log directory path
        path: String,
        /// Print the output in a table format
        #[arg(long, default_value_t = false)]
        table: bool,
    },
    /// List persisted log entries and metadata for all local nodes
    PersistedAll {
        /// The log directory path
        path: String,
        /// Print the output in a table format
        #[arg(long, default_value_t = false)]
        table: bool,
    },
    /// List all log entries
    Entries {
        /// The address of the RaftNode
        address: String,
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
        Commands::Debug(x) => match x {
            DebugSubcommands::Persisted { path, table } => {
                debug_persisted::<LogStorage>(path.as_str(), logger.clone(), table)?;
            }
            DebugSubcommands::PersistedAll { path, table } => {
                debug_persisted_all::<LogStorage>(path.as_str(), logger.clone(), table)?;
            }
            DebugSubcommands::Entries { address } => {
                debug_entries(address.as_str()).await?;
            }
            DebugSubcommands::Node { address } => {
                debug_node(address.as_str()).await?;
            }
        },
    }

    Ok(())
}
