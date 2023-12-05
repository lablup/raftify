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
    pub fn new<A: ToSocketAddrs>(addr: A) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();

        return Peer {
            addr,
            client: None,
            state: PeerState::Preparing,
        };
    }

    pub async fn connect(&mut self) -> Result<()> {
        println!("Connect!!!!!!!!");

        let client = RaftServiceClient::connect(format!("http://{}", self.addr)).await?;
        self.client = Some(client);
        self.state = PeerState::Connected;
        Ok(())
    }
}
