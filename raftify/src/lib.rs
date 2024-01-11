#[macro_use]
extern crate async_trait;

mod config;
mod error;
mod follower_role;
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
pub mod raft_service;

pub use async_trait::async_trait;
pub use formatter::CustomFormatter;
pub use jopemachine_raft as raft;
pub use raft::Config as RaftConfig;
pub use tonic;
pub use tonic::transport::Channel;

pub use crate::config::Config;
pub use crate::error::{Error, Result};
pub use crate::follower_role::FollowerRole;
pub use crate::log_entry::AbstractLogEntry;
pub use crate::peer::Peer;
pub use crate::peers::Peers;
pub use crate::raft_client::create_client;
pub use crate::raft_facade::ClusterJoinTicket;
pub use crate::raft_facade::Raft;
pub use crate::raft_node::RaftNode;
pub use crate::raft_service::raft_service_client::RaftServiceClient;
pub use crate::state_machine::AbstractStateMachine;
pub use crate::storage::heed::{HeedStorage, LogStore};

// pub(crate) use utils::get_filesize;
// pub(crate) use utils::is_near_zero;
pub(crate) use utils::macro_utils;
