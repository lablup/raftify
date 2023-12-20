include!(concat!(env!("OUT_DIR"), "/built.rs"));

use crate::{AbstractLogEntry, AbstractStateMachine, MyDeserializer, Result};
use clap::{App, Arg, SubCommand};
use commands::debug::debug_node;
use commands::debug::debug_persisted;
use raft::default_logger;
use raft::derializer::set_custom_deserializer;
use std::fmt::Debug;

mod commands;

pub async fn cli_handler<
    LogEntry: AbstractLogEntry + Debug + Send + 'static,
    FSM: AbstractStateMachine<LogEntry> + Debug + Clone + Send + Sync + 'static,
>() -> Result<()> {
    let matches = App::new("raftify")
        .version(PKG_VERSION)
        .author(PKG_AUTHORS)
        .about(PKG_DESCRIPTION)
        .subcommand(
            SubCommand::with_name("debug")
                .about("Debug tools")
                .subcommand(
                    SubCommand::with_name("persisted")
                        .about("List persisted log entries and metadata")
                        .arg(
                            Arg::with_name("path")
                                .help("The log directory path")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("entries")
                        .about("List all log entries")
                        .arg(
                            Arg::with_name("address")
                                .help("The address of the RaftNode")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("node").about("Inspect RaftNode").arg(
                        Arg::with_name("address")
                            .help("The address of the RaftNode")
                            .required(true)
                            .index(1),
                    ),
                ),
        )
        // .subcommand(SubCommand::with_name("health").about("Check health"))
        .get_matches();

    let logger = default_logger();
    set_custom_deserializer(MyDeserializer::<LogEntry, FSM>::new());

    match matches.subcommand() {
        Some(("debug", debug_matches)) => match debug_matches.subcommand() {
            Some(("entries", entries_matches)) => {
                if let Some(address) = entries_matches.value_of("address") {
                    debug_node(address).await?;
                }
            }
            Some(("node", node_matches)) => {
                if let Some(address) = node_matches.value_of("address") {
                    debug_node(address).await?;
                }
            }
            Some(("persisted", node_matches)) => {
                if let Some(path) = node_matches.value_of("path") {
                    debug_persisted(path, logger)?;
                }
            }
            _ => {}
        },
        _ => {}
    };

    Ok(())
}
