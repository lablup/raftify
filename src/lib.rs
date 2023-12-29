#[macro_use]
extern crate async_trait;

mod config;
mod deserializer;
mod error;
mod follower_role;
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
pub use deserializer::MyDeserializer;
pub use raft;
pub use tonic;

pub use crate::raft_facade::ClusterJoinTicket;
pub use raft::Config as RaftConfig;

pub use crate::config::Config;
pub use crate::error::{Error, Result};
pub use crate::follower_role::FollowerRole;
pub use crate::log_entry::AbstractLogEntry;
pub use crate::peer::Peer;
pub use crate::peers::Peers;
pub use crate::raft_client::create_client;
pub use crate::raft_facade::Raft;
pub use crate::raft_node::RaftNode;
pub use crate::raft_service::raft_service_client::RaftServiceClient;
pub use crate::state_machine::AbstractStateMachine;
pub use crate::storage::heed::HeedStorage;
pub use crate::storage::heed::LogStore;
pub use async_trait::async_trait;
pub use tonic::transport::Channel;
pub(crate) use utils::get_filesize;
