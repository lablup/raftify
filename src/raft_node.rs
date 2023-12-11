use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::{Result, SendMessageError};
use crate::request_message::RequestMessage;
use crate::response_message::ResponseMessage;
use crate::storage::heed::{HeedStorage, LogStore};
use crate::storage::utils::get_storage_path;
use crate::{AbstractStateMachine, Config};
use crate::{Error, Peers};

use bincode::{deserialize, serialize};
use prost::Message as PMessage;
use raft::derializer::{format_confchangev2, format_message};
use raft::eraftpb::{
    ConfChange, ConfChangeType, ConfChangeV2, Entry, EntryType, Message as RaftMessage, Snapshot,
};
use raft::raw_node::RawNode;
use tokio::sync::{mpsc, Mutex, RwLockReadGuard, RwLockWriteGuard};
use tokio::sync::{oneshot, RwLock};
use tokio::time::timeout;
use tonic::Request;

#[derive(Clone)]
pub struct RaftNode<FSM: AbstractStateMachine + Clone + 'static>(Arc<RwLock<RaftNodeCore<FSM>>>);

impl<FSM: AbstractStateMachine + Clone + Send + 'static> RaftNode<FSM> {
    async fn wl(&mut self) -> RwLockWriteGuard<RaftNodeCore<FSM>> {
        self.0.write().await
    }

    async fn rl(&self) -> RwLockReadGuard<RaftNodeCore<FSM>> {
        self.0.read().await
    }
}

impl<FSM: AbstractStateMachine + Clone + Send + 'static> RaftNode<FSM> {
    pub fn bootstrap_cluster(
        rcv: mpsc::Receiver<RequestMessage>,
        snd: mpsc::Sender<RequestMessage>,
        fsm: FSM,
        config: Config,
        initial_peers: Peers,
        logger: slog::Logger,
    ) -> Result<Self> {
        RaftNodeCore::bootstrap_cluster(rcv, snd, fsm, config, initial_peers, logger)
            .map(|core| Self(Arc::new(RwLock::new(core))))
    }

    pub fn new_follower(
        rcv: mpsc::Receiver<RequestMessage>,
        snd: mpsc::Sender<RequestMessage>,
        id: u64,
        fsm: FSM,
        config: Config,
        peers: Peers,
        logger: slog::Logger,
    ) -> Result<Self> {
        RaftNodeCore::new_follower(rcv, snd, id, fsm, config, peers, logger)
            .map(|core| Self(Arc::new(RwLock::new(core))))
    }

    pub async fn is_leader(&self) -> bool {
        self.rl().await.is_leader()
    }

    pub async fn get_id(&self) -> u64 {
        self.rl().await.get_id()
    }

    pub async fn get_leader_id(&self) -> u64 {
        self.rl().await.get_leader_id()
    }

    // pub async fn get_peers(&self) -> Peers {
    //     self.rl().await.get_peers()
    // }

    pub async fn add_peer<A: ToSocketAddrs>(&mut self, id: u64, addr: A) {
        self.wl().await.add_peer(id, addr).await
    }

    pub async fn inspect(&self) -> Result<String> {
        self.rl().await.inspect().await
    }

    pub async fn store(&self) -> HeedStorage {
        self.rl().await.raw_node.store().clone()
    }

    pub async fn make_snapshot(&mut self, index: u64, term: u64) -> Result<()> {
        self.wl().await.make_snapshot(index, term).await
    }

    pub async fn run(mut self) -> Result<()> {
        self.wl().await.run().await
    }
}

pub struct RaftNodeCore<FSM: AbstractStateMachine + Clone + 'static> {
    pub raw_node: RawNode<HeedStorage>,
    pub rcv: mpsc::Receiver<RequestMessage>,
    pub fsm: FSM,
    pub peers: Arc<Mutex<Peers>>,
    pub snd: mpsc::Sender<RequestMessage>,
    response_seq: AtomicU64,
    config: Config,
    should_exit: bool,
    last_snapshot_created: Instant,
    logger: slog::Logger,
}

impl<FSM: AbstractStateMachine + Clone + Send + 'static> RaftNodeCore<FSM> {
    pub fn bootstrap_cluster(
        rcv: mpsc::Receiver<RequestMessage>,
        snd: mpsc::Sender<RequestMessage>,
        fsm: FSM,
        config: Config,
        initial_peers: Peers,
        logger: slog::Logger,
    ) -> Result<Self> {
        let mut raft_config = config.raft_config.clone();

        raft_config.id = 1;
        raft_config.validate()?;

        let mut storage = HeedStorage::create(
            get_storage_path(config.log_dir.as_str(), 1)?,
            &config,
            logger.clone(),
        )?;

        let mut snapshot = Snapshot::default();
        snapshot.mut_metadata().index = 0;
        snapshot.mut_metadata().term = 1;
        snapshot.mut_metadata().mut_conf_state().voters = vec![1];
        storage.apply_snapshot(snapshot).unwrap();

        let mut raw_node = RawNode::new(&raft_config, storage, &logger)?;
        let response_seq = AtomicU64::new(0);
        let last_snapshot_created = Instant::now();

        raw_node.raft.become_candidate();
        raw_node.raft.become_leader();

        Ok(RaftNodeCore {
            raw_node,
            rcv,
            fsm,
            response_seq,
            snd,
            config,
            logger,
            last_snapshot_created,
            should_exit: false,
            peers: Arc::new(Mutex::new(initial_peers)),
        })
    }

    pub fn new_follower(
        rcv: mpsc::Receiver<RequestMessage>,
        snd: mpsc::Sender<RequestMessage>,
        node_id: u64,
        fsm: FSM,
        config: Config,
        peers: Peers,
        logger: slog::Logger,
    ) -> Result<Self> {
        let mut raft_config = config.raft_config.clone();

        raft_config.id = node_id;
        raft_config.validate()?;

        let storage = HeedStorage::create(
            get_storage_path(config.log_dir.as_str(), node_id)?,
            &config,
            logger.clone(),
        )?;
        let raw_node = RawNode::new(&raft_config, storage, &logger)?;
        let response_seq = AtomicU64::new(0);
        let last_snapshot_created = Instant::now()
            .checked_sub(Duration::from_secs(1000))
            .unwrap();

        Ok(RaftNodeCore {
            raw_node,
            rcv,
            fsm,
            response_seq,
            snd,
            config,
            logger,
            last_snapshot_created,
            should_exit: false,
            peers: Arc::new(Mutex::new(peers)),
        })
    }

    pub fn is_leader(&self) -> bool {
        self.raw_node.raft.leader_id == self.raw_node.raft.id
    }

    pub fn get_id(&self) -> u64 {
        self.raw_node.raft.id
    }

    pub fn get_leader_id(&self) -> u64 {
        self.raw_node.raft.leader_id
    }

    // pub fn get_peers(&self) -> Peers {
    //     self.peers.to_owned()
    // }

    pub async fn add_peer<A: ToSocketAddrs>(&mut self, id: u64, addr: A) {
        // self.peers.add_peer(id, addr)
        self.peers.lock().await.add_peer(id, addr)
    }

    async fn send_wrongleader_response(
        &self,
        channel: oneshot::Sender<ResponseMessage>,
    ) -> Result<()> {
        let leader_id = self.get_leader_id();
        let peers = self.peers.lock().await;
        let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();

        let raft_response = ResponseMessage::WrongLeader {
            leader_id,
            leader_addr,
        };

        // TODO handle error here
        channel.send(raft_response).unwrap();
        Ok(())
    }

    async fn send_message(
        message: RaftMessage,
        peers: Arc<Mutex<Peers>>,
        snd: mpsc::Sender<RequestMessage>,
        logger: slog::Logger,
    ) {
        let client_id = message.get_to();

        let mut ok = std::result::Result::<(), SendMessageError>::Ok(());

        let client = match peers.lock().await.get_mut(&client_id) {
            Some(peer) => {
                if peer.client.is_none() {
                    if let Err(e) = peer.connect().await {
                        slog::trace!(logger, "Connection error: {:?}", e);
                        ok = Err(SendMessageError::ConnectionError(client_id.to_string()));
                    }
                }
                peer.client.clone()
            }
            None => {
                ok = Err(SendMessageError::PeerNotFoundError(client_id.to_string()));
                None
            }
        };

        if let Some(mut client) = client {
            let message = Request::new(message.clone());
            if let Err(e) = client.send_message(message).await {
                slog::trace!(logger, "Message transmission error: {:?}", e);
                ok = Err(SendMessageError::TransmissionError(client_id.to_string()));
            }
        }

        if let Err(e) = ok {
            slog::debug!(logger, "Error occurred while sending message: {}", e);
            snd.send(RequestMessage::ReportUnreachable { node_id: client_id })
                .await
                .unwrap();
        }
    }

    async fn send_messages(&mut self, messages: Vec<RaftMessage>) -> Result<()> {
        for message in messages {
            tokio::spawn(RaftNodeCore::<FSM>::send_message(
                message,
                self.peers.clone(),
                self.snd.clone(),
                self.logger.clone(),
            ));
        }
        Ok(())
    }

    async fn handle_committed_entries(
        &mut self,
        committed_entries: Vec<Entry>,
        client_send: &mut HashMap<u64, oneshot::Sender<ResponseMessage>>,
    ) -> Result<()> {
        for mut entry in committed_entries {
            if entry.get_data().is_empty() {
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
        let response_seq: u64 = deserialize(&entry.get_context())?;
        let data = self.fsm.apply(entry.get_data()).await?;
        if let Some(sender) = senders.remove(&response_seq) {
            sender.send(ResponseMessage::Response { data }).unwrap();
        }

        if Instant::now()
            > self.last_snapshot_created + Duration::from_secs_f32(self.config.snapshot_interval)
        {
            self.make_snapshot(entry.index, entry.term).await?;
        }
        Ok(())
    }

    pub async fn make_snapshot(&mut self, index: u64, term: u64) -> Result<()> {
        self.last_snapshot_created = Instant::now();
        let snapshot_data = self.fsm.snapshot().await?;

        let last_applied = self.raw_node.raft.raft_log.applied;
        let store = self.raw_node.mut_store();
        store.compact(last_applied)?;
        store.create_snapshot(snapshot_data, index, term)?;
        Ok(())
    }

    async fn handle_committed_config_change_entry(
        &mut self,
        entry: &Entry,
        senders: &mut HashMap<u64, oneshot::Sender<ResponseMessage>>,
    ) -> Result<()> {
        let conf_change_v2: ConfChangeV2 = PMessage::decode(entry.get_data())?;

        let conf_changes = conf_change_v2.get_changes();
        let addrs: Vec<SocketAddr> = deserialize(conf_change_v2.get_context())?;

        for (cc_idx, conf_change) in conf_changes.iter().enumerate() {
            let node_id = conf_change.get_node_id();
            let change_type = conf_change.get_change_type();

            match change_type {
                ConfChangeType::AddNode | ConfChangeType::AddLearnerNode => {
                    let addr = addrs[cc_idx];
                    slog::info!(
                        self.logger,
                        "Node {} ({}) joined the cluster.",
                        node_id,
                        addr
                    );
                    self.peers.lock().await.add_peer(node_id, &addr.to_string());
                }
                ConfChangeType::RemoveNode => {
                    if node_id == self.get_id() {
                        self.should_exit = true;
                        slog::info!(self.logger, "Node {} quit the cluster.", node_id);
                    } else {
                        slog::info!(self.logger, "Node {} removed from the cluster.", node_id);
                        self.peers.lock().await.remove(&node_id);
                    }
                }
            }
        }

        match self.raw_node.apply_conf_change(&conf_change_v2) {
            Ok(conf_state) => {
                let store = self.raw_node.mut_store();
                store.set_conf_state(&conf_state)?;
                self.make_snapshot(entry.index, entry.term).await?;
            }
            Err(e) => {
                slog::error!(self.logger, "Failed to apply configuration change: {}", e);
            }
        }

        let response_seq: AtomicU64 = deserialize(entry.get_context())?;
        let response_seq_value = response_seq.load(Ordering::Relaxed);

        if let Some(sender) = senders.remove(&response_seq_value) {
            let mut response: ResponseMessage = ResponseMessage::Error;

            // TODO: Handle other cases
            if conf_changes.iter().all(|cc| {
                cc.get_change_type() == ConfChangeType::AddNode
                    || cc.get_change_type() == ConfChangeType::AddLearnerNode
            }) {
                response = ResponseMessage::JoinSuccess {
                    assigned_id: conf_changes[0].get_node_id(),
                    peers: self.peers.lock().await.clone(),
                };
            }

            if conf_changes
                .iter()
                .all(|cc| cc.get_change_type() == ConfChangeType::RemoveNode)
            {
                response = ResponseMessage::Ok;
            }

            if sender.send(response).is_err() {
                slog::error!(self.logger, "error sending response")
            }
        }

        Ok(())
    }

    pub async fn inspect(&self) -> Result<String> {
        let raw_node = &self.raw_node;
        let prs = raw_node.raft.prs().get(self.get_id()).unwrap();
        let store = raw_node.store();

        let id = raw_node.raft.id;
        let leader_id = raw_node.raft.leader_id;
        let hard_state = store.hard_state()?;
        let conf_state = store.conf_state()?;
        let snapshot = store.snapshot(0, 0)?;
        let last_index = raw_node.raft.raft_log.last_index();

        let last_applied = raw_node.raft.raft_log.applied;
        let last_committed = raw_node.raft.raft_log.committed;
        let last_persisted = raw_node.raft.raft_log.persisted;

        let outline = format!(
            "========= Outline =========\n\
            node_id: {}\n\
            leader_id: {}\n",
            id, leader_id
        );

        let persistence_info = format!(
            "========= Persistence Info =========\n\
            hard_state: {:?}\n\
            conf_state: {:?}\n\
            last_index: {}\n\
            snapshot: {:?}\n",
            hard_state, conf_state, last_index, snapshot
        );

        let prs_info = format!(
            "========= Progresses =========\n\
            {:?}
            ",
            prs
        );

        let raftlog_metadata = format!(
            "========= RaftLog Metadata =========\n\
            last_applied: {}\n\
            last_committed: {}\n\
            last_persisted: {}\n",
            last_applied, last_committed, last_persisted
        );

        let result = format!("{outline:}\n{persistence_info:}\n{prs_info}\n{raftlog_metadata:}",);

        Ok(result)
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut heartbeat = Duration::from_secs_f32(self.config.tick_interval);
        let mut now = Instant::now();

        // A map to contain sender to client responses
        let mut client_send = HashMap::new();

        loop {
            if self.should_exit {
                slog::info!(self.logger, "Node {} quit the cluster.", self.get_id());
                return Ok(());
            }

            match timeout(heartbeat, self.rcv.recv()).await {
                Ok(Some(RequestMessage::ConfigChange { chan, conf_change })) => {
                    if self.raw_node.raft.has_pending_conf() {
                        slog::warn!(self.logger, "Reject the conf change because pending conf change exist! (pending_conf_index={}), try later...", self.raw_node.raft.pending_conf_index);
                        continue;
                    }

                    if !self.is_leader() {
                        // wrong leader send client cluster data
                        // TODO: retry strategy in case of failure
                        self.send_wrongleader_response(chan).await?;
                    } else {
                        let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                        client_send.insert(response_seq, chan);
                        slog::debug!(
                            self.logger,
                            "Proposed new config change..., seq={response_seq:}, conf_change_v2={}",
                            format_confchangev2(&conf_change)
                        );

                        self.raw_node
                            .propose_conf_change(serialize(&response_seq).unwrap(), conf_change)?;
                    }
                }
                Ok(Some(RequestMessage::RaftMessage { message })) => {
                    slog::debug!(
                        self.logger,
                        "Node {} received Raft message from the node {}, Message: {}",
                        self.raw_node.raft.id,
                        message.from,
                        format_message(&message)
                    );
                    self.raw_node.step(*message)?
                }
                Ok(Some(RequestMessage::Propose { proposal, chan })) => {
                    if !self.is_leader() {
                        // wrong leader send client cluster data
                        let leader_id = self.get_leader_id();
                        // leader can't be an empty node
                        let leader_addr = self
                            .peers
                            .lock()
                            .await
                            .get(&leader_id)
                            .unwrap()
                            .addr
                            .to_string();
                        let raft_response = ResponseMessage::WrongLeader {
                            leader_id,
                            leader_addr,
                        };
                        chan.send(raft_response).unwrap();
                    } else {
                        let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                        client_send.insert(response_seq, chan);
                        let response_seq = serialize(&response_seq).unwrap();
                        self.raw_node.propose(response_seq, proposal).unwrap();
                    }
                }
                Ok(Some(RequestMessage::RequestId { chan })) => {
                    if !self.is_leader() {
                        // TODO: retry strategy in case of failure
                        self.send_wrongleader_response(chan).await?;
                    } else {
                        let reserved_id = self.peers.lock().await.reserve_peer(self.get_id());
                        slog::info!(self.logger, "Reserved peer id, {}", reserved_id);

                        chan.send(ResponseMessage::IdReserved {
                            reserved_id,
                            leader_id: self.get_id(),
                            peers: self.peers.lock().await.clone(),
                        })
                        .unwrap();
                    }
                }
                Ok(Some(RequestMessage::ReportUnreachable { node_id })) => {
                    self.raw_node.report_unreachable(node_id);
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
                heartbeat = Duration::from_millis(100);
                self.raw_node.tick();
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
        if !self.raw_node.has_ready() {
            return Ok(());
        }
        let mut ready = self.raw_node.ready();

        if !ready.messages().is_empty() {
            self.send_messages(ready.take_messages()).await?;
        }

        if *ready.snapshot() != Snapshot::default() {
            slog::info!(
                self.logger,
                "Restoring a state machine from the snapshot..."
            );
            let snapshot = ready.snapshot();
            if !snapshot.get_data().is_empty() {
                self.fsm.restore(snapshot.get_data()).await?;
            }
            let store = self.raw_node.mut_store();
            store.apply_snapshot(snapshot.clone())?;
        }

        self.handle_committed_entries(ready.take_committed_entries(), client_send)
            .await?;

        if !ready.entries().is_empty() {
            let entries = &ready.entries()[..];
            let store = self.raw_node.mut_store();
            store.append(entries)?;
        }

        if let Some(hs) = ready.hs() {
            let store = self.raw_node.mut_store();
            store.set_hard_state(hs)?;
        }

        if !ready.persisted_messages().is_empty() {
            self.send_messages(ready.take_persisted_messages()).await?;
        }

        let mut light_rd = self.raw_node.advance(ready);

        if let Some(commit) = light_rd.commit_index() {
            let store = self.raw_node.mut_store();
            store.set_hard_state_commit(commit)?;
        }

        self.send_messages(light_rd.take_messages()).await?;
        self.handle_committed_entries(light_rd.take_committed_entries(), client_send)
            .await?;

        self.raw_node.advance_apply();

        Ok(())
    }
}
