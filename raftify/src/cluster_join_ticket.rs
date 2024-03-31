use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClusterJoinTicket {
    pub reserved_id: u64,
    pub raft_addr: String,
    pub leader_addr: String,
    pub peers: HashMap<u64, SocketAddr>,
}
