use crate::raft::eraftpb::{ConfChange, ConfChangeSingle, ConfChangeV2};

pub fn to_confchange_v2(conf_change: ConfChange) -> ConfChangeV2 {
    let mut cc_v2 = ConfChangeV2::default();

    let mut cs = ConfChangeSingle::default();
    cs.set_node_id(conf_change.node_id);
    cs.set_change_type(conf_change.get_change_type());
    cc_v2.set_changes(vec![cs]);
    cc_v2.set_context(conf_change.context);

    cc_v2
}
