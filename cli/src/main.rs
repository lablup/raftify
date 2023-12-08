// main.rs 또는 다른 파일
include!(concat!(env!("OUT_DIR"), "/built.rs"));

use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("raftify")
        .version(PKG_VERSION)
        .author(PKG_AUTHORS)
        .about(PKG_DESCRIPTION)
        .subcommand(
            SubCommand::with_name("debug")
                .about("Debug tools")
                .subcommand(
                    SubCommand::with_name("entries")
                        .about("List persisted log entries")
                        .arg(
                            Arg::with_name("address")
                                .help("The address to connect to")
                                .required(true)
                                .index(1),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("node").about("Manage nodes").arg(
                        Arg::with_name("address")
                            .help("The address to connect to for node")
                            .required(true)
                            .index(1),
                    ),
                ),
        )
        .get_matches();

    if let Some(debug_matches) = matches.subcommand_matches("debug") {
        if let Some(entries_matches) = debug_matches.subcommand_matches("entries") {
            if let Some(address) = entries_matches.value_of("address") {
                println!("Running 'entries' for address: {}", address);
            }
        } else if let Some(node_matches) = debug_matches.subcommand_matches("node") {
            if let Some(address) = node_matches.value_of("address") {
                println!("Running 'node' for address: {}", address);
            }
        }
    }
}
