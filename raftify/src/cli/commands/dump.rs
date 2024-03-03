use bincode::{deserialize, serialize};
use jopemachine_raft::{
    eraftpb::{ConfChange, ConfChangeSingle, ConfChangeType, ConfChangeV2, Entry, EntryType},
    logger::Slogger,
    Storage,
};
use prost::Message as _;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::{utils::to_confchange_v2, Config, HeedStorage, Result, StableStorage};

/// Read all_entries and make the appropriate ConfChanges to make it peers compared to the peers given in json format.
pub fn dump_peers(path: &str, peers: HashMap<u64, SocketAddr>, logger: slog::Logger) -> Result<()> {
    let config = Config {
        log_dir: path.to_string(),
        ..Default::default()
    };

    let mut storage = HeedStorage::create(
        config.log_dir.as_str(),
        &config,
        Arc::new(Slogger {
            slog: logger.clone(),
        }),
    )?;

    let entries = storage.all_entries()?;

    let mut persisted_peers_configs: HashMap<u64, SocketAddr> = HashMap::new();

    for entry in entries.iter() {
        let conf_change_v2 = match entry.get_entry_type() {
            EntryType::EntryConfChange => to_confchange_v2(ConfChange::decode(entry.get_data())?),
            EntryType::EntryConfChangeV2 => ConfChangeV2::decode(entry.get_data())?,
            _ => continue,
        };

        let conf_changes = conf_change_v2.get_changes();
        let addrs: Vec<SocketAddr> = deserialize(conf_change_v2.get_context())?;

        for (cc_idx, conf_change) in conf_changes.iter().enumerate() {
            let addr = addrs[cc_idx];

            let node_id = conf_change.get_node_id();
            let change_type = conf_change.get_change_type();

            match change_type {
                ConfChangeType::AddNode | ConfChangeType::AddLearnerNode => {
                    persisted_peers_configs.insert(node_id, addr);
                }
                ConfChangeType::RemoveNode => {
                    persisted_peers_configs.remove(&node_id);
                }
            }
        }
    }

    // old - new = to be removed
    let diffs_to_remove: HashMap<_, _> = persisted_peers_configs
        .iter()
        .filter(|(k, v)| peers.get(k) != Some(v))
        .collect();

    // new - old = to be added
    let diffs_to_add: HashMap<_, _> = peers
        .iter()
        .filter(|(k, v)| persisted_peers_configs.get(k) != Some(v))
        .collect();

    let mut new_cc_v2 = ConfChangeV2::default();
    let mut conf_changes: Vec<ConfChangeSingle> = vec![];
    let mut cc_addrs: Vec<SocketAddr> = vec![];

    for (k, v) in diffs_to_remove.into_iter() {
        let mut cs = ConfChangeSingle::default();
        cs.set_node_id(*k);
        cs.set_change_type(ConfChangeType::RemoveNode);
        conf_changes.push(cs);
        cc_addrs.push(*v);
    }

    // TODO: Support AddLearnerNode
    for (k, v) in diffs_to_add.into_iter() {
        let mut cs = ConfChangeSingle::default();
        cs.set_node_id(*k);
        cs.set_change_type(ConfChangeType::AddNode);
        conf_changes.push(cs);
        cc_addrs.push(*v);
    }

    if !conf_changes.is_empty() {
        new_cc_v2.set_context(serialize(&cc_addrs)?);
        new_cc_v2.set_changes(conf_changes);

        let last_idx = storage.last_index()?;
        let last_term = storage.term(last_idx)?;

        let mut new_entry = Entry::default();
        new_entry.set_index(last_idx + 1);
        new_entry.set_term(last_term);
        new_entry.set_entry_type(EntryType::EntryConfChangeV2);
        new_entry.set_data(new_cc_v2.encode_to_vec());
        new_entry.set_context(vec![]);

        storage.append(vec![new_entry].as_slice())?;

        let mut snapshot = storage.snapshot(0, last_idx)?;
        let mut meta = snapshot.get_metadata().clone();
        meta.set_index(last_idx + 1);
        snapshot.set_metadata(meta);
        storage.apply_snapshot(snapshot)?;

        println!("Changes applied successfully");
    } else {
        println!("No changes to be made");
    }

    Ok(())
}
