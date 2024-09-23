use std::net::SocketAddr;

use crate::raft::eraftpb::{self, ConfChangeSingle, ConfChangeTransition, ConfChangeV2};
use crate::raft_service;

#[derive(Debug, Clone)]
pub struct ConfChangeRequest {
    pub changes: Vec<ConfChangeSingle>,
    pub addrs: Vec<SocketAddr>,
}

impl From<ConfChangeRequest> for ConfChangeV2 {
    fn from(conf_change_request: ConfChangeRequest) -> Self {
        if conf_change_request.addrs.len() != conf_change_request.changes.len() {
            panic!("addrs and changes must have the same length");
        }

        let mut conf_change_v2 = ConfChangeV2::default();
        conf_change_v2.set_changes(conf_change_request.changes);
        conf_change_v2.set_context(bincode::serialize(&conf_change_request.addrs).unwrap());

        if conf_change_request.addrs.len() > 1 {
            conf_change_v2.set_transition(ConfChangeTransition::Explicit);
        }

        conf_change_v2
    }
}

impl From<ConfChangeV2> for ConfChangeRequest {
    fn from(cc_v2: ConfChangeV2) -> Self {
        let changes = cc_v2
            .get_changes()
            .iter()
            .map(|change| ConfChangeSingle {
                node_id: change.node_id,
                change_type: change.change_type,
            })
            .collect();

        let addrs: Vec<SocketAddr> = bincode::deserialize(cc_v2.get_context()).unwrap();

        Self { changes, addrs }
    }
}

impl From<raft_service::ChangeConfigArgs> for ConfChangeRequest {
    fn from(conf_change_request: raft_service::ChangeConfigArgs) -> Self {
        let changes = conf_change_request
            .changes
            .into_iter()
            .map(|change| ConfChangeSingle {
                node_id: change.node_id,
                change_type: change.change_type,
            })
            .collect();

        let addrs = conf_change_request
            .addrs
            .into_iter()
            .map(|addr| addr.parse().unwrap())
            .collect();

        Self { changes, addrs }
    }
}

impl From<ConfChangeRequest> for raft_service::ChangeConfigArgs {
    fn from(conf_change_request: ConfChangeRequest) -> Self {
        let changes = conf_change_request
            .changes
            .into_iter()
            .map(|change| eraftpb::ConfChangeSingle {
                node_id: change.node_id,
                change_type: change.change_type,
            })
            .collect();

        let addrs = conf_change_request
            .addrs
            .into_iter()
            .map(|addr| addr.to_string())
            .collect();

        Self { changes, addrs }
    }
}
