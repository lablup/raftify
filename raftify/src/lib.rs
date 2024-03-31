#[macro_use]
extern crate async_trait;

mod config;
mod error;
mod formatter;
mod log_entry;
mod peer;
mod peers;
mod raft_bootstrapper;
mod raft_client;
mod raft_node;
mod raft_server;
mod state_machine;
mod storage;
mod utils;

mod request;
mod response;

pub mod cli;
pub mod cluster_join_ticket;
pub mod raft_service;

pub use {
    async_trait::async_trait, bincode, formatter::CustomFormatter, jopemachine_raft as raft,
    raft::Config as RaftConfig, tonic, tonic::transport::Channel,
};

pub use crate::{
    cluster_join_ticket::ClusterJoinTicket,
    config::Config,
    error::{Error, Result},
    log_entry::AbstractLogEntry,
    peer::Peer,
    peers::Peers,
    raft_bootstrapper::Raft,
    raft_client::create_client,
    raft_node::{role::InitialRole, RaftNode},
    raft_service::raft_service_client::RaftServiceClient,
    state_machine::AbstractStateMachine,
    storage::heed_storage::HeedStorage,
    storage::StableStorage,
    request::common::confchange_request::ConfChangeRequest,
};

pub(crate) use crate::utils::macros::macro_utils;
