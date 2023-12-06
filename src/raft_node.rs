use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::{Deref, DerefMut};
use std::os::macos::raw;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::raft_service::raft_service_client::RaftServiceClient;
use crate::request_message::RequestMessage;
use crate::response_message::ResponseMessage;
use crate::storage::heed::{HeedStorage, LogStore};
use crate::Peers;
use crate::{AbstractStateMachine, Config};

use bincode::{deserialize, serialize};
use log::*;
use prost::Message as PMessage;
use raft::eraftpb::{
    ConfChangeType, ConfChangeV2, Entry, EntryType, HardState, Message as RaftMessage, Snapshot,
};
use raft::raw_node::RawNode;
use raft::{LightReady, Ready};
use tokio::sync::{mpsc, Mutex};
use tokio::sync::{oneshot, RwLock};
use tokio::time::timeout;
use tonic::Request;

struct MessageSender {
    message: RaftMessage,
    client: RaftServiceClient<tonic::transport::channel::Channel>,
    client_id: u64,
    chan: mpsc::Sender<RequestMessage>,
    max_retries: usize,
    timeout: Duration,
}

impl MessageSender {
    /// attempt to send a message MessageSender::max_retries times at MessageSender::timeout
    /// interval.
    async fn send(mut self) {
        let mut current_retry = 0usize;
        loop {
            let message_request = Request::new(self.message.clone());
            match self.client.send_message(message_request).await {
                Ok(_) => {
                    return;
                }
                Err(e) => {
                    if current_retry < self.max_retries {
                        current_retry += 1;
                        tokio::time::sleep(self.timeout).await;
                    } else {
                        debug!(
                            "error sending message after {} retries: {}",
                            self.max_retries, e
                        );
                        let _ = self
                            .chan
                            .send(RequestMessage::ReportUnreachable {
                                node_id: self.client_id,
                            })
                            .await;
                        return;
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct RaftNode<FSM: AbstractStateMachine + Clone + 'static> {
    pub raw_node: Arc<RwLock<RawNode<HeedStorage>>>,
    pub rcv: Arc<Mutex<mpsc::Receiver<RequestMessage>>>,
    pub fsm: Arc<Mutex<FSM>>,
    pub peers: Arc<Mutex<Peers>>,
    pub snd: mpsc::Sender<RequestMessage>,
    pub storage: HeedStorage,
    response_seq: Arc<AtomicU64>,
    config: Config,
    should_exit: bool,
    bootstrap_done: bool,
    last_snapshot_created: Instant,
}

impl<FSM: AbstractStateMachine + Clone + Send + 'static> RaftNode<FSM> {
    pub fn bootstrap_cluster(
        rcv: mpsc::Receiver<RequestMessage>,
        snd: mpsc::Sender<RequestMessage>,
        fsm: FSM,
        config: Config,
        initial_peers: Peers,
        logger: &slog::Logger,
        bootstrap_done: bool,
    ) -> Result<Self> {
        let mut raft_config = config.raft_config.clone();

        raft_config.id = 1;
        raft_config.validate()?;

        let mut snapshot = Snapshot::default();
        // Because we don't use the same configuration to initialize every node, so we use
        // a non-zero index to force new followers catch up logs by snapshot first, which will
        // bring all nodes to the same initial state.
        snapshot.mut_metadata().index = 0;
        snapshot.mut_metadata().term = 0;
        snapshot.mut_metadata().mut_conf_state().voters = vec![1];

        let mut storage = HeedStorage::create(".", 1)?;
        storage.apply_snapshot(snapshot).unwrap();
        let mut raw_node = RawNode::new(&raft_config, storage.clone(), logger)?;
        let response_seq = AtomicU64::new(0);
        let last_snapshot_created = Instant::now();

        raw_node.raft.become_candidate();
        raw_node.raft.become_leader();

        let raw_node = Arc::new(RwLock::new(raw_node));
        let rcv = Arc::new(Mutex::new(rcv));
        let fsm = Arc::new(Mutex::new(fsm));
        let initial_peers = Arc::new(Mutex::new(initial_peers));
        let response_seq = Arc::new(response_seq);

        Ok(RaftNode {
            raw_node,
            rcv,
            fsm,
            response_seq,
            snd,
            config,
            storage,
            bootstrap_done,
            last_snapshot_created,
            peers: initial_peers,
            should_exit: false,
        })
    }

    pub fn new_follower(
        rcv: mpsc::Receiver<RequestMessage>,
        snd: mpsc::Sender<RequestMessage>,
        id: u64,
        fsm: FSM,
        config: Config,
        peers: Peers,
        logger: &slog::Logger,
        bootstrap_done: bool,
    ) -> Result<Self> {
        let mut raft_config = config.raft_config.clone();

        raft_config.id = id;
        raft_config.validate()?;

        let storage = HeedStorage::create(".", id)?;
        let raw_node = RawNode::new(&raft_config, storage.clone(), logger)?;
        let response_seq = AtomicU64::new(0);
        let last_snapshot_created = Instant::now()
            .checked_sub(Duration::from_secs(1000))
            .unwrap();

        let raw_node = Arc::new(RwLock::new(raw_node));
        let rcv = Arc::new(Mutex::new(rcv));
        let fsm = Arc::new(Mutex::new(fsm));
        let response_seq = Arc::new(response_seq);
        let peers = Arc::new(Mutex::new(peers));

        Ok(RaftNode {
            raw_node,
            rcv,
            peers,
            fsm,
            response_seq,
            snd,
            config,
            storage,
            last_snapshot_created,
            bootstrap_done,
            should_exit: false,
        })
    }

    pub fn is_leader(&self, raw_node: &RawNode<HeedStorage>) -> bool {
        raw_node.raft.leader_id == raw_node.raft.id
    }

    pub fn get_id(&self, raw_node: &RawNode<HeedStorage>) -> u64 {
        raw_node.raft.id
    }

    pub fn get_leader_id(&self, raw_node: &RawNode<HeedStorage>) -> u64 {
        raw_node.raft.leader_id
    }

    pub async fn add_peer<A: ToSocketAddrs>(&mut self, id: u64, addr: A) {
        let mut peers = self.peers.lock().await;
        peers.add_peer(id, addr)
    }

    async fn send_wrongleader_response(
        &self,
        raw_node: &RawNode<HeedStorage>,
        channel: oneshot::Sender<ResponseMessage>,
    ) {
        let leader_id = self.get_leader_id(raw_node);
        let peers = self.peers.lock().await.clone();
        let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();

        let raft_response = ResponseMessage::WrongLeader {
            leader_id,
            leader_addr,
        };
        // TODO handle error here
        let _ = channel.send(raft_response);
    }

    async fn send_messages(&mut self, messages: Vec<RaftMessage>) {
        if !self.bootstrap_done {
            return;
        }

        let mut peers = self.peers.lock().await;

        for message in messages {
            let client = match peers.get_mut(&message.get_to()) {
                Some(peer) => {
                    if peer.client.is_none() {
                        // TODO: Handle error here
                        let _ = peer.connect().await;
                    }
                    peer.client.clone()
                }
                None => continue,
            }
            .unwrap();

            let message_sender = MessageSender {
                client_id: message.get_to(),
                client: client.clone(),
                message,
                chan: self.snd.clone(),
                timeout: Duration::from_millis(100),
                max_retries: 5,
            };

            tokio::spawn(message_sender.send());
        }
    }

    async fn handle_committed_entries(
        &mut self,
        committed_entries: Vec<Entry>,
        client_send: &mut HashMap<u64, oneshot::Sender<ResponseMessage>>,
    ) -> Result<()> {
        for mut entry in committed_entries {
            if entry.get_data().is_empty() {
                // Empty entry, when the peer becomes Leader it will send an empty entry.
                continue;
            }
            match entry.get_entry_type() {
                EntryType::EntryConfChangeV2 => {
                    self.handle_committed_config_change_entry(&mut entry, client_send)
                        .await?;
                }
                _ => {
                    self.handle_committed_normal_entry(&entry, client_send)
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_committed_normal_entry(
        &mut self,
        entry: &Entry,
        senders: &mut HashMap<u64, oneshot::Sender<ResponseMessage>>,
    ) -> Result<()> {
        let mut fsm = self.fsm.lock().await;
        let response_seq: u64 = deserialize(&entry.get_context())?;
        let data = fsm.apply(entry.get_data()).await?;
        if let Some(sender) = senders.remove(&response_seq) {
            sender.send(ResponseMessage::Response { data }).unwrap();
        }

        if Instant::now()
            > self.last_snapshot_created + Duration::from_secs_f32(self.config.snapshot_interval)
        {
            let mut raw_node = self.raw_node.write().await;
            let raw_node = raw_node.deref_mut();

            self.last_snapshot_created = Instant::now();
            let fsm = self.fsm.lock().await;
            let snapshot_data = fsm.snapshot().await?;

            let last_applied = raw_node.raft.raft_log.applied;
            let store = raw_node.mut_store();
            store.compact(last_applied)?;
            let _ = store.create_snapshot(snapshot_data, entry.get_index(), entry.get_term());
        }
        Ok(())
    }

    async fn handle_committed_config_change_entry(
        &mut self,
        entry: &Entry,
        senders: &mut HashMap<u64, oneshot::Sender<ResponseMessage>>,
    ) -> Result<()> {
        let mut raw_node = self.raw_node.write().await;
        let raw_node = raw_node.deref_mut();

        let conf_change_v2: ConfChangeV2 = PMessage::decode(entry.get_data())?;

        let conf_changes = conf_change_v2.get_changes();
        let addrs: Vec<SocketAddr> = deserialize(conf_change_v2.get_context())?;
        let mut peers = self.peers.lock().await;

        for (cc_idx, conf_change) in conf_changes.iter().enumerate() {
            let node_id = conf_change.get_node_id();
            let change_type = conf_change.get_change_type();

            match change_type {
                ConfChangeType::AddNode => {
                    let addr = addrs[cc_idx];
                    log::info!("Node {} ({}) joined the cluster.", node_id, addr);
                    peers.add_peer(node_id, &addr.to_string());
                }
                ConfChangeType::RemoveNode => {
                    if node_id == self.get_id(raw_node) {
                        self.should_exit = true;
                        log::info!("Node {} quit the cluster.", node_id);
                    } else {
                        log::info!("Node {} removed from the cluster.", node_id);
                        peers.remove(&node_id);
                    }
                }
                _ => unimplemented!(),
            }
        }

        match raw_node.apply_conf_change(&conf_change_v2) {
            Ok(conf_state) => {
                let store = raw_node.mut_store();
                store.set_conf_state(&conf_state)?;

                self.last_snapshot_created = Instant::now();
                let fsm = self.fsm.lock().await;
                let snapshot_data = fsm.snapshot().await?;

                let last_applied = raw_node.raft.raft_log.applied;
                let store = raw_node.mut_store();
                store.compact(last_applied)?;
                let _ = store.create_snapshot(snapshot_data, entry.get_index(), entry.get_term());
            }
            Err(e) => {
                log::error!("Failed to apply configuration change: {}", e);
            }
        }

        let response_seq: AtomicU64 = deserialize(entry.get_context())?;
        let response_seq_value = response_seq.load(Ordering::Relaxed);

        if let Some(sender) = senders.remove(&response_seq_value) {
            let mut response: ResponseMessage = ResponseMessage::Error;

            if conf_changes.iter().all(|cc| {
                cc.get_change_type() == ConfChangeType::AddNode
                    || cc.get_change_type() == ConfChangeType::AddLearnerNode
            }) {
                response = ResponseMessage::JoinSuccess {
                    assigned_id: conf_changes[0].get_node_id(),
                    peers: peers.clone(),
                };
            }

            if conf_changes
                .iter()
                .all(|cc| cc.get_change_type() == ConfChangeType::RemoveNode)
            {
                response = ResponseMessage::Ok;
            }

            if sender.send(response).is_err() {
                error!("error sending response")
            }
        }

        Ok(())
    }

    pub async fn inspect(&self) -> Result<String> {
        // let prs = self.raft.prs().iter().map(|(k, v)| (k, v)).collect();

        let raw_node = self.raw_node.read().await;
        let id = raw_node.raft.id;
        let leader_id = raw_node.raft.leader_id;
        let hard_state = self.storage.hard_state()?;
        let conf_state = self.storage.conf_state()?;
        let snapshot = self.storage.snapshot(0, 0)?;
        let last_index = raw_node.raft.raft_log.last_index();

        let last_applied = raw_node.raft.raft_log.applied;
        let last_committed = raw_node.raft.raft_log.committed;
        let last_persisted = raw_node.raft.raft_log.persisted;

        let result = format!(
            "========= Outline =========\
node_id: {id}\
leader_id: {leader_id}\
\
========= Persistence Info =========\
hard_state: {hard_state:?}\
conf_state: {conf_state:?}\
last_index: {last_index}\
snapshot: {snapshot:?}\
\
========= RaftLog Metadata =========\
last_applied: {last_applied}\
last_committed: {last_committed}\
last_persisted: {last_persisted}\
",
        );

        Ok(result)
    }

    pub async fn run(mut self) -> Result<()> {
        let mut heartbeat = Duration::from_secs_f32(self.config.tick_interval);
        let mut now = Instant::now();

        // A map to contain sender to client responses
        let mut client_send = HashMap::new();

        loop {
            if self.should_exit {
                warn!("Quitting raft");
                return Ok(());
            }

            match timeout(heartbeat, self.rcv.lock().await.recv()).await {
                Ok(Some(RequestMessage::ClusterBootstrapReady { peers, chan })) => {
                    log::info!(
                        "All nodes are ready to join the cluster. Start to bootstrap process..."
                    )
                    // self.peers = peers;
                }
                Ok(Some(RequestMessage::ConfigChange { chan, conf_change })) => {
                    let mut raw_node = self.raw_node.write().await;
                    let raw_node = raw_node.deref_mut();

                    if raw_node.raft.has_pending_conf() {
                        log::warn!("Reject the conf change because pending conf change exist! (pending_conf_index={}), try later...", raw_node.raft.pending_conf_index);
                        continue;
                    }

                    if !self.is_leader(raw_node) {
                        // wrong leader send client cluster data
                        // TODO: retry strategy in case of failure
                        self.send_wrongleader_response(raw_node, chan).await;
                    } else {
                        let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                        client_send.insert(response_seq, chan);
                        log::debug!(
                            "Proposed new config change..., seq={}, conf_change_v2={:?}",
                            response_seq,
                            conf_change
                        );

                        raw_node
                            .propose_conf_change(serialize(&response_seq).unwrap(), conf_change)?;
                    }
                }
                Ok(Some(RequestMessage::Raft(m))) => {
                    let mut raw_node = self.raw_node.write().await;
                    debug!(
                        "Node {} received Raft message from the node {}, Message: {:?}",
                        raw_node.raft.id, m.from, m
                    );
                    if let Ok(_a) = raw_node.step(*m) {};
                }
                Ok(Some(RequestMessage::Propose { proposal, chan })) => {
                    let mut raw_node = self.raw_node.write().await;
                    let raw_node = raw_node.deref_mut();
                    let peers = self.peers.lock().await;

                    if !self.is_leader(raw_node) {
                        // wrong leader send client cluster data
                        let leader_id = self.get_leader_id(raw_node);
                        // leader can't be an empty node
                        let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();
                        let raft_response = ResponseMessage::WrongLeader {
                            leader_id,
                            leader_addr,
                        };
                        chan.send(raft_response).unwrap();
                    } else {
                        let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                        client_send.insert(response_seq, chan);
                        let response_seq = serialize(&response_seq).unwrap();
                        raw_node.propose(response_seq, proposal).unwrap();
                    }
                }
                Ok(Some(RequestMessage::RequestId { chan })) => {
                    let raw_node = self.raw_node.read().await;
                    let raw_node = raw_node.deref();
                    let mut peers = self.peers.lock().await;

                    if !self.is_leader(raw_node) {
                        // TODO: retry strategy in case of failure
                        self.send_wrongleader_response(raw_node, chan).await;
                    } else {
                        chan.send(ResponseMessage::IdReserved {
                            reserved_id: peers.reserve_peer(self.get_id(raw_node)).await,
                            leader_id: self.get_id(raw_node),
                            peers: peers.clone(),
                        })
                        .unwrap();
                    }
                }
                Ok(Some(RequestMessage::ReportUnreachable { node_id })) => {
                    let mut raw_node = self.raw_node.write().await;
                    raw_node.report_unreachable(node_id);
                }
                Ok(Some(RequestMessage::DebugNode { chan })) => {
                    chan.send(ResponseMessage::DebugNode {
                        result: self.inspect().await?,
                    })
                    .unwrap();
                }
                Ok(_) => unreachable!(),
                Err(_) => (),
            }

            let elapsed = now.elapsed();
            now = Instant::now();
            if elapsed > heartbeat {
                let mut raw_node = self.raw_node.write().await;
                heartbeat = Duration::from_millis(100);
                raw_node.tick();
            } else {
                heartbeat -= elapsed;
            }

            self.on_ready(&mut client_send).await?;
        }
    }

    async fn on_ready(
        &mut self,
        client_send: &mut HashMap<u64, oneshot::Sender<ResponseMessage>>,
    ) -> Result<()> {
        let mut ready: Ready = Ready::default();
        {
            let mut raw_node = self.raw_node.write().await;
            if !raw_node.has_ready() {
                return Ok(());
            }
            ready = raw_node.ready();
        }

        if !ready.messages().is_empty() {
            self.send_messages(ready.take_messages()).await;
        }

        if *ready.snapshot() != Snapshot::default() {
            let snapshot = ready.snapshot();
            let mut fsm = self.fsm.lock().await;
            fsm.restore(snapshot.get_data()).await?;
            let mut raw_node = self.raw_node.write().await;
            let store = raw_node.mut_store();
            store.apply_snapshot(snapshot.clone())?;
        }

        self.handle_committed_entries(ready.take_committed_entries(), client_send)
            .await?;

        {
            let mut raw_node = self.raw_node.write().await;
            if !ready.entries().is_empty() {
                let entries = &ready.entries()[..];
                let store = raw_node.mut_store();
                store.append(entries)?;
            }

            if let Some(hs) = ready.hs() {
                let store = raw_node.mut_store();
                store.set_hard_state(hs)?;
            }
        }

        if !ready.persisted_messages().is_empty() {
            self.send_messages(ready.take_persisted_messages()).await;
        }

        let mut light_rd: raft::LightReady = LightReady::default();

        {
            let mut raw_node = self.raw_node.write().await;
            light_rd = raw_node.advance(ready);

            if let Some(commit) = light_rd.commit_index() {
                let store = raw_node.mut_store();
                store.set_hard_state_comit(commit)?;
            }
        }

        self.send_messages(light_rd.take_messages()).await;
        self.handle_committed_entries(light_rd.take_committed_entries(), client_send)
            .await?;

        {
            let mut raw_node = self.raw_node.write().await;
            raw_node.advance_apply();
        }

        Ok(())
    }
}
