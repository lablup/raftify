use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, ToSocketAddrs};
use tonic::transport::Channel;

use crate::{config::TlsConfig, InitialRole};

use super::{create_client, error::Result, raft_service::raft_service_client::RaftServiceClient};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Peer {
    pub addr: SocketAddr,
    pub initial_role: InitialRole,
    pub client_tls_config: Option<TlsConfig>,
    #[serde(skip)]
    pub client: Option<RaftServiceClient<Channel>>,
}

// TODO: Implement From<Peer> for raft_service::Peer
// impl From<Peer> for raft_service::Peer {
//     fn from(peer: Peer) -> Self {
//         raft_service::Peer {
//             addr: peer.addr.to_string(),
//             role: peer.role.into(),
//         }
//     }
// }

impl Peer {
    pub fn new<A: ToSocketAddrs>(
        addr: A,
        initial_role: InitialRole,
        client_tls_config: Option<TlsConfig>,
    ) -> Self {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        Peer {
            addr,
            client_tls_config,
            initial_role,
            client: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let client = create_client(&self.addr, self.client_tls_config.clone()).await?;
        self.client = Some(client);
        Ok(())
    }
}
