use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Peers;

#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseMessage {
    WrongLeader {
        leader_id: u64,
        leader_addr: String,
    },
    JoinSuccess {
        assigned_id: u64,
        peers: Peers,
    },
    IdReserved {
        leader_id: u64,
        reserved_id: u64,
        peers: Peers,
    },
    Error,
    Response {
        data: Vec<u8>,
    },
    Ok,
}
