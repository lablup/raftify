use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, ToSocketAddrs};
use tonic::transport::Channel;

use crate::InitialRole;

use super::{create_client, error::Result, raft_service::raft_service_client::RaftServiceClient};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Peer {
    pub addr: SocketAddr,
    pub role: InitialRole,
    #[serde(skip_serializing, skip_deserializing)]
    pub client: Option<RaftServiceClient<Channel>>,
}

impl Peer {
    pub fn new<A: ToSocketAddrs>(addr: A, initial_role: InitialRole) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        Peer {
            addr,
            role: initial_role,
            client: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let client = create_client(&self.addr).await?;
        self.client = Some(client);
        Ok(())
    }
}
