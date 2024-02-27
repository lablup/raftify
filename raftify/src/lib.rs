#[macro_use]
extern crate async_trait;

mod config;
mod error;
mod formatter;
mod log_entry;
mod peer;
mod peers;
mod raft_client;
mod raft_facade;
mod raft_node;
mod raft_server;
mod request_message;
mod response_message;
mod state_machine;
mod storage;
mod utils;

pub mod cli;
pub mod custom_callbacks;
pub mod raft_service;

pub use {
    async_trait::async_trait, bincode, formatter::CustomFormatter, jopemachine_raft as raft,
    raft::Config as RaftConfig, tonic, tonic::transport::Channel,
};

pub use crate::{
    config::Config,
    error::{Error, Result},
    log_entry::AbstractLogEntry,
    peer::Peer,
    peers::Peers,
    raft_client::create_client,
    raft_facade::{ClusterJoinTicket, Raft},
    raft_node::{role::InitialRole, RaftNode},
    raft_service::raft_service_client::RaftServiceClient,
    state_machine::AbstractStateMachine,
    storage::heed::{HeedStorage, LogStore},
};

pub(crate) use utils::macro_utils;
