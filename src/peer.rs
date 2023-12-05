use std::net::{SocketAddr, ToSocketAddrs};

use crate::error::Result;
use crate::raft_service::raft_service_client::RaftServiceClient;

use serde::{Deserialize, Serialize};
use tonic::transport::Channel;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PeerState {
    Preparing,
    Connected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Peer {
    pub addr: SocketAddr,
    pub state: PeerState,
    #[serde(skip_serializing, skip_deserializing)]
    pub client: Option<RaftServiceClient<Channel>>,
}

impl Peer {
    pub async fn new<A: ToSocketAddrs>(addr: A) -> Result<Peer> {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let client = Some(RaftServiceClient::connect(format!("http://{}", addr.to_string())).await?);

        return Ok(Peer {
            addr,
            client,
            state: PeerState::Preparing,
        });
    }
}
