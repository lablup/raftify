use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::{Result, SendMessageError};
use crate::raft_service::{ClusterBootstrapReadyArgs, ResultCode};
use crate::request_message::{LocalRequestMsg, ServerRequestMsg};
use crate::response_message::{
    LocalResponseMsg, ResponseMessage, ServerConfChangeResponseResult, ServerResponseMsg,
    ServerResponseResult,
};
use crate::storage::heed::{HeedStorage, LogStore};
use crate::storage::utils::get_storage_path;
use crate::utils::OneShotMutex;
use crate::{AbstractLogEntry, Error, Peers};
use crate::{AbstractStateMachine, Config};

use bincode::{deserialize, serialize};
use prost::Message as PMessage;
use raft::derializer::{format_confchangev2, format_message, format_snapshot};
use raft::eraftpb::{
    ConfChange, ConfChangeSingle, ConfChangeType, ConfChangeV2, Entry, EntryType,
    Message as RaftMessage, Snapshot,
};
use raft::raw_node::RawNode;
use tokio::select;
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex};
use tokio::time::interval;
use tonic::Request;

#[derive(Clone)]
pub struct RaftNode<
    LogEntry: AbstractLogEntry + Send + 'static,
    FSM: AbstractStateMachine<LogEntry> + Clone + 'static,
> {
    // The lock of RaftNodeCore is locked when RaftNode.run is called and is never released until program terminates.
    // However, to implement the Clone trait in RaftNode, Arc and Mutex are necessary. That's why we use OneShotMutex here.
    inner: Arc<OneShotMutex<RaftNodeCore<LogEntry, FSM>>>,
    local_sender: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
}

impl<
        LogEntry: AbstractLogEntry + Send + 'static,
        FSM: AbstractStateMachine<LogEntry> + Clone + Send + 'static,
    > RaftNode<LogEntry, FSM>
{
    pub fn bootstrap_cluster(
        fsm: FSM,
        config: Config,
        initial_peers: Peers,
        logger: slog::Logger,
        bootstrap_done: bool,
        server_rcv: mpsc::Receiver<ServerRequestMsg>,
        server_snd: mpsc::Sender<ServerRequestMsg>,
        local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
        local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
    ) -> Result<Self> {
        RaftNodeCore::<LogEntry, FSM>::bootstrap_cluster(
            fsm,
            config,
            initial_peers,
            logger,
            bootstrap_done,
            server_rcv,
            server_snd,
            local_rcv,
            local_snd.clone(),
        )
        .map(|core| Self {
            inner: Arc::new(OneShotMutex::new(core)),
            local_sender: local_snd.clone(),
        })
    }

    pub fn new_follower(
        id: u64,
        fsm: FSM,
        config: Config,
        peers: Peers,
        logger: slog::Logger,
        bootstrap_done: bool,
        server_rcv: mpsc::Receiver<ServerRequestMsg>,
        server_snd: mpsc::Sender<ServerRequestMsg>,
        local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
        local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
    ) -> Result<Self> {
        RaftNodeCore::<LogEntry, FSM>::new_follower(
            id,
            fsm,
            config,
            peers,
            logger,
            bootstrap_done,
            server_rcv,
            server_snd,
            local_rcv,
            local_snd.clone(),
        )
        .map(|core| Self {
            inner: Arc::new(OneShotMutex::new(core)),
            local_sender: local_snd.clone(),
        })
    }

    pub async fn is_leader(&self) -> bool {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::IsLeader { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::IsLeader { is_leader } => is_leader,
            _ => unreachable!(),
        }
    }

    pub async fn get_id(&self) -> u64 {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::IsLeader { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::GetId { id } => id,
            _ => unreachable!(),
        }
    }

    pub async fn get_leader_id(&self) -> u64 {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::GetLeaderId { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::GetLeaderId { leader_id } => leader_id,
            _ => unreachable!(),
        }
    }

    pub async fn get_peers(&self) -> Peers {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::GetPeers { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::GetPeers { peers } => peers,
            _ => unreachable!(),
        }
    }

    pub async fn add_peer<A: ToSocketAddrs>(&mut self, id: u64, addr: A) {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap().to_string();
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::AddPeer { id, addr, chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::AddPeer {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn inspect(&self) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Inspect { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::Inspect { result } => Ok(result),
            _ => unreachable!(),
        }
    }

    pub async fn store(&self) -> FSM {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Store { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::Store { store } => store,
            _ => unreachable!(),
        }
    }

    pub async fn storage(&self) -> HeedStorage {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Storage { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::Storage { storage } => storage,
            _ => unreachable!(),
        }
    }

    pub async fn propose(&self, proposal: Vec<u8>) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Propose { proposal, chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::Propose {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn get_cluster_size(&self) -> usize {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::GetClusterSize { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::GetClusterSize { size } => size,
            _ => unreachable!(),
        }
    }

    pub async fn quit(&mut self) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Quit { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::Quit {} => (),
            _ => unreachable!(),
        }
    }

    // pub async fn make_snapshot(&mut self, index: u64, term: u64) -> Result<()> {
    //     self.wl().await.make_snapshot(index, term).await
    // }

    pub async fn run(self) -> Result<()> {
        self.inner
            .lock()
            .await
            .expect("RaftNode mutex's owner should be only RaftNode.run!")
            .run()
            .await
    }
}

enum ResponseSender<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine<LogEntry>> {
    Local(oneshot::Sender<LocalResponseMsg<LogEntry, FSM>>),
    Server(oneshot::Sender<ServerResponseMsg>),
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine<LogEntry>>
    ResponseSender<LogEntry, FSM>
{
    fn send(self, response: ResponseMessage<LogEntry, FSM>) {
        match self {
            ResponseSender::Local(sender) => {
                if let ResponseMessage::Local(response) = response {
                    sender.send(response).unwrap()
                } else {
                    unreachable!()
                }
            }
            ResponseSender::Server(sender) => {
                if let ResponseMessage::Server(response) = response {
                    sender.send(response).unwrap()
                } else {
                    unreachable!()
                }
            }
        }
    }
}

pub struct RaftNodeCore<
    LogEntry: AbstractLogEntry + Send + 'static,
    FSM: AbstractStateMachine<LogEntry> + Clone + 'static,
> {
    pub raw_node: RawNode<HeedStorage>,
    pub fsm: FSM,
    pub peers: Arc<Mutex<Peers>>,
    response_seq: AtomicU64,
    config: Config,
    should_exit: bool,
    last_snapshot_created: Instant,
    logger: slog::Logger,
    bootstrap_done: bool,
    peers_bootstrap_ready: Option<HashMap<u64, bool>>,
    response_senders: HashMap<u64, ResponseSender<LogEntry, FSM>>,

    server_rcv: mpsc::Receiver<ServerRequestMsg>,
    server_snd: mpsc::Sender<ServerRequestMsg>,
    local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
    #[allow(dead_code)]
    local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,

    _phantom_log_entry_typ: PhantomData<LogEntry>,
}

impl<
        LogEntry: AbstractLogEntry + Send + 'static,
        FSM: AbstractStateMachine<LogEntry> + Clone + Send + 'static,
    > RaftNodeCore<LogEntry, FSM>
{
    pub fn bootstrap_cluster(
        fsm: FSM,
        config: Config,
        initial_peers: Peers,
        logger: slog::Logger,
        bootstrap_done: bool,
        server_rcv: mpsc::Receiver<ServerRequestMsg>,
        server_snd: mpsc::Sender<ServerRequestMsg>,
        local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
        local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
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
        snapshot.mut_metadata().term = 0;
        snapshot.mut_metadata().mut_conf_state().voters = vec![1];
        storage.apply_snapshot(snapshot).unwrap();

        let mut raw_node = RawNode::new(&raft_config, storage, &logger)?;
        let response_seq = AtomicU64::new(0);
        let last_snapshot_created = Instant::now();

        raw_node.raft.become_candidate();
        raw_node.raft.become_leader();

        let peers_bootstrap_ready = if !initial_peers.is_empty() {
            Some(HashMap::from([(1, true)]))
        } else {
            None
        };

        Ok(RaftNodeCore {
            raw_node,
            fsm,
            response_seq,
            config,
            logger,
            last_snapshot_created,
            should_exit: false,
            peers: Arc::new(Mutex::new(initial_peers)),
            bootstrap_done,
            peers_bootstrap_ready,
            response_senders: HashMap::new(),
            server_rcv,
            server_snd,
            local_rcv,
            local_snd,
            _phantom_log_entry_typ: PhantomData,
        })
    }

    pub fn new_follower(
        node_id: u64,
        fsm: FSM,
        config: Config,
        peers: Peers,
        logger: slog::Logger,
        bootstrap_done: bool,
        server_rcv: mpsc::Receiver<ServerRequestMsg>,
        server_snd: mpsc::Sender<ServerRequestMsg>,
        local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
        local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
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
            fsm,
            response_seq,
            config,
            logger,
            last_snapshot_created,
            should_exit: false,
            peers: Arc::new(Mutex::new(peers)),
            bootstrap_done,
            peers_bootstrap_ready: None,
            response_senders: HashMap::new(),
            server_rcv,
            server_snd,
            local_rcv,
            local_snd,
            _phantom_log_entry_typ: PhantomData,
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

    pub async fn get_peers(&self) -> Peers {
        self.peers.lock().await.to_owned()
    }

    pub async fn add_peer<A: ToSocketAddrs>(&mut self, id: u64, addr: A) {
        self.peers.lock().await.add_peer(id, addr)
    }

    async fn send_message(
        message: RaftMessage,
        peers: Arc<Mutex<Peers>>,
        snd: mpsc::Sender<ServerRequestMsg>,
        logger: slog::Logger,
    ) {
        let node_id = message.get_to();

        let mut ok = std::result::Result::<(), SendMessageError>::Ok(());

        let client = match peers.lock().await.get_mut(&node_id) {
            Some(peer) => {
                if peer.client.is_none() {
                    if let Err(e) = peer.connect().await {
                        slog::trace!(logger, "Connection error: {:?}", e);
                        ok = Err(SendMessageError::ConnectionError(node_id.to_string()));
                    }
                }
                peer.client.clone()
            }
            None => {
                ok = Err(SendMessageError::PeerNotFoundError(node_id.to_string()));
                None
            }
        };

        if let Some(mut client) = client {
            let message = Request::new(message.clone());
            if let Err(e) = client.send_message(message).await {
                slog::trace!(logger, "Message transmission error: {:?}", e);
                ok = Err(SendMessageError::TransmissionError(node_id.to_string()));
            }
        }

        if let Err(e) = ok {
            slog::debug!(logger, "Error occurred while sending message: {}", e);
            snd.send(ServerRequestMsg::ReportUnreachable { node_id })
                .await
                .unwrap();
        }
    }

    async fn send_messages(&mut self, messages: Vec<RaftMessage>) {
        if !self.bootstrap_done {
            return;
        }

        for message in messages {
            tokio::spawn(RaftNodeCore::<LogEntry, FSM>::send_message(
                message,
                self.peers.clone(),
                self.server_snd.clone(),
                self.logger.clone(),
            ));
        }
    }

    async fn handle_committed_entries(&mut self, committed_entries: Vec<Entry>) -> Result<()> {
        for mut entry in committed_entries {
            if entry.get_data().is_empty() {
                continue;
            }

            match entry.get_entry_type() {
                EntryType::EntryConfChange | EntryType::EntryConfChangeV2 => {
                    self.handle_committed_config_change_entry(&mut entry)
                        .await?;
                }
                EntryType::EntryNormal => {
                    self.handle_committed_normal_entry(&entry).await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_committed_normal_entry(&mut self, entry: &Entry) -> Result<()> {
        let response_seq: u64 = deserialize(&entry.get_context())?;
        let log_entry = LogEntry::decode(entry.get_data())?;
        let _data = self.fsm.apply(log_entry).await?;

        if let Some(sender) = self.response_senders.remove(&response_seq) {
            match sender {
                ResponseSender::Local(sender) => {
                    sender.send(LocalResponseMsg::Propose {}).unwrap();
                }
                ResponseSender::Server(sender) => {
                    sender
                        .send(ServerResponseMsg::Propose {
                            result: ServerResponseResult::Success,
                        })
                        .unwrap();
                }
            }
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
        let snapshot_data = snapshot_data.encode()?;

        let last_applied = self.raw_node.raft.raft_log.applied;
        let store = self.raw_node.mut_store();
        store.compact(last_applied)?;
        store.create_snapshot(snapshot_data, index, term)?;
        Ok(())
    }

    async fn handle_committed_config_change_entry(&mut self, entry: &Entry) -> Result<()> {
        let conf_change_v2 = match entry.get_entry_type() {
            EntryType::EntryConfChange => {
                let conf_change = ConfChange::decode(entry.get_data())?;
                let mut cc_v2 = ConfChangeV2::default();

                let mut cs = ConfChangeSingle::default();
                cs.set_node_id(conf_change.node_id);
                cs.set_change_type(conf_change.get_change_type());
                cc_v2.set_changes(vec![cs].into());
                cc_v2.set_context(conf_change.context);

                cc_v2
            }
            EntryType::EntryConfChangeV2 => ConfChangeV2::decode(entry.get_data())?,
            _ => unreachable!(),
        };

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

        let response_seq = if entry.get_context().is_empty() {
            AtomicU64::new(0)
        } else {
            deserialize(entry.get_context())?
        };

        let response_seq_value = response_seq.load(Ordering::Relaxed);

        if let Some(sender) = self.response_senders.remove(&response_seq_value) {
            let mut response = ServerConfChangeResponseResult::Error(Error::Unknown);

            // TODO: Handle other cases
            if conf_changes.iter().all(|cc| {
                cc.get_change_type() == ConfChangeType::AddNode
                    || cc.get_change_type() == ConfChangeType::AddLearnerNode
            }) {
                response = ServerConfChangeResponseResult::JoinSuccess {
                    assigned_id: conf_changes[0].get_node_id(),
                    peers: self.peers.lock().await.clone(),
                };
            }

            if conf_changes
                .iter()
                .all(|cc| cc.get_change_type() == ConfChangeType::RemoveNode)
            {
                response = ServerConfChangeResponseResult::RemoveSuccess;
            }

            sender.send(ServerResponseMsg::ConfigChange { result: response }.into());
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
            hard_state,
            conf_state,
            last_index,
            format_snapshot(&snapshot),
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

    pub async fn wait_for_initial_peers_bootstrap(&mut self) -> Result<()> {
        if !self.is_leader() {
            return Ok(());
        }

        let peers_bootstrap_ready = self.peers_bootstrap_ready.clone().unwrap();

        loop {
            {
                let peers = self.peers.lock().await;
                if peers.len() == peers_bootstrap_ready.len()
                    && peers_bootstrap_ready.iter().all(|v| *v.1 == true)
                {
                    break;
                }
                slog::trace!(
                    self.logger,
                    "Waiting for all follower nodes to be ready to join the cluster..."
                );
            }
            // tokio::time::sleep(Duration::from_secs(1)).await;
            return Ok(());
        }

        slog::info!(
            self.logger,
            "Received All follower nodes join requests, preparing to bootstrap the cluster..."
        );

        self.bootstrap_peers().await?;

        let mut peers = self.peers.lock().await;

        for node_id in peers_bootstrap_ready.keys() {
            if *node_id == 1 {
                continue;
            }

            let peer = peers.get_mut(node_id).expect("Peer not found!");

            // Connection error, but try to bootstrap.
            if let Err(err) = peer.connect().await {
                slog::error!(
                    self.logger,
                    "Failed to connect to node {}: {}",
                    node_id,
                    err
                );
            }

            let response = peer
                .client
                .as_mut()
                .unwrap()
                .cluster_bootstrap_ready(Request::new(ClusterBootstrapReadyArgs {}))
                .await?
                .into_inner();

            match response.code() {
                ResultCode::Ok => {}
                ResultCode::Error => {
                    slog::error!(self.logger, "Node {} failed to join the cluster.", node_id);
                }
                ResultCode::WrongLeader => {
                    slog::error!(
                        self.logger,
                        "Node {} failed to join the cluster because of wrong leader.",
                        node_id
                    );
                }
            }
        }

        Ok(())
    }

    /// Commit the configuration change to add all follower nodes to the cluster.
    async fn bootstrap_peers(&mut self) -> Result<()> {
        assert!(self.is_leader());

        self.make_snapshot(self.raw_node.store().last_index()?, 1)
            .await?;

        let initial_peers = self.peers.lock().await;

        let mut entries = vec![];
        let last_index = self.raw_node.store().last_index().unwrap();

        for (i, peer) in initial_peers.iter().enumerate() {
            let node_id = &peer.0;
            let node_addr = initial_peers.get(node_id).unwrap().addr;
            // Skip leader
            if *node_id == 1 {
                continue;
            }
            let mut conf_change = ConfChange::default();
            conf_change.set_node_id(*node_id);
            conf_change.set_change_type(ConfChangeType::AddNode);
            conf_change.set_context(serialize(&vec![node_addr]).unwrap());

            let conf_state = self.raw_node.apply_conf_change(&conf_change)?;
            self.raw_node.mut_store().set_conf_state(&conf_state)?;

            let mut entry = Entry::default();
            entry.set_entry_type(EntryType::EntryConfChange);
            entry.set_term(1);
            entry.set_index(last_index + i as u64);
            entry.set_data(conf_change.encode_to_vec());
            entry.set_context(vec![]);
            entries.push(entry);
        }

        let unstable = &mut self.raw_node.raft.raft_log.unstable;
        unstable.entries = entries.clone();
        let commit_index = last_index + entries.len() as u64;
        unstable.stable_entries(commit_index, 1);

        self.raw_node.mut_store().append(&entries)?;

        self.last_snapshot_created = Instant::now();

        let last_applied = self.raw_node.raft.raft_log.applied;
        let store = self.raw_node.mut_store();
        store.compact(last_applied)?;
        store.create_snapshot(vec![], commit_index, 1)?;

        self.raw_node.raft.raft_log.applied = commit_index;
        self.raw_node.raft.raft_log.committed = commit_index;
        self.raw_node.raft.raft_log.persisted = commit_index;

        let leader_id = self.get_leader_id();
        self.raw_node
            .raft
            .mut_prs()
            .get_mut(leader_id)
            .unwrap()
            .matched = commit_index;
        self.raw_node
            .raft
            .mut_prs()
            .get_mut(leader_id)
            .unwrap()
            .committed_index = commit_index;
        self.raw_node
            .raft
            .mut_prs()
            .get_mut(leader_id)
            .unwrap()
            .next_idx = commit_index + 1;

        self.bootstrap_done = true;

        Ok(())
    }

    pub async fn handle_local_request_msg(
        &mut self,
        message: LocalRequestMsg<LogEntry, FSM>,
    ) -> Result<()> {
        match message {
            LocalRequestMsg::IsLeader { chan } => {
                chan.send(LocalResponseMsg::IsLeader {
                    is_leader: self.is_leader(),
                })
                .unwrap();
            }
            LocalRequestMsg::GetId { chan } => {
                chan.send(LocalResponseMsg::GetId { id: self.get_id() })
                    .unwrap();
            }
            LocalRequestMsg::GetLeaderId { chan } => {
                chan.send(LocalResponseMsg::GetLeaderId {
                    leader_id: self.get_leader_id(),
                })
                .unwrap();
            }
            LocalRequestMsg::GetPeers { chan } => {
                chan.send(LocalResponseMsg::GetPeers {
                    peers: self.get_peers().await,
                })
                .unwrap();
            }
            LocalRequestMsg::AddPeer { id, addr, chan } => {
                self.add_peer(id, addr).await;
                chan.send(LocalResponseMsg::AddPeer {}).unwrap();
            }
            LocalRequestMsg::Store { chan } => {
                chan.send(LocalResponseMsg::Store {
                    store: self.fsm.clone(),
                })
                .unwrap();
            }
            LocalRequestMsg::Storage { chan } => {
                chan.send(LocalResponseMsg::Storage {
                    storage: self.raw_node.store().clone(),
                })
                .unwrap();
            }
            LocalRequestMsg::Inspect { chan } => {
                chan.send(LocalResponseMsg::Inspect {
                    result: self.inspect().await?,
                })
                .unwrap();
            }
            LocalRequestMsg::Propose { proposal, chan } => {
                let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                self.response_senders
                    .insert(response_seq, ResponseSender::Local(chan));
                let response_seq = serialize(&response_seq)?;
                self.raw_node.propose(response_seq, proposal)?;
            }
            LocalRequestMsg::GetClusterSize { chan } => {
                let size = self.raw_node.raft.prs().iter().collect::<Vec<_>>().len();
                chan.send(LocalResponseMsg::GetClusterSize { size })
                    .unwrap();
            }
            LocalRequestMsg::Quit { chan } => {
                self.should_exit = true;
                chan.send(LocalResponseMsg::Quit {}).unwrap();
            }
            // LocalRequestMsg::MakeSnapshot { index, term, chan } => {
            //     self.make_snapshot(index, term).await?;
            //     chan.send(LocalResponseMsg::Ok).unwrap();
            // }
            _ => unreachable!(),
        }

        Ok(())
    }

    pub async fn handle_server_request_msg(&mut self, message: ServerRequestMsg) -> Result<()> {
        match message {
            ServerRequestMsg::ClusterBootstrapReady { chan } => {
                slog::info!(
                    self.logger,
                    "All initial nodes (peers) requested to join cluster. Start to bootstrap process..."
                );
                chan.send(ServerResponseMsg::ClusterBootstrapReady {
                    result: ServerResponseResult::Success,
                })
                .unwrap();
                self.bootstrap_done = true;
            }
            ServerRequestMsg::MemberBootstrapReady { node_id, chan } => {
                assert!(self.is_leader());
                slog::info!(
                    self.logger,
                    "Node {} requested to join the cluster.",
                    node_id
                );
                self.peers_bootstrap_ready
                    .as_mut()
                    .unwrap()
                    .insert(node_id, true);

                chan.send(ServerResponseMsg::MemberBootstrapReady {
                    result: ServerResponseResult::Success,
                })
                .unwrap();
            }
            ServerRequestMsg::ConfigChange { chan, conf_change } => {
                if self.raw_node.raft.has_pending_conf() {
                    slog::warn!(self.logger, "Reject the conf change because pending conf change exist! (pending_conf_index={}), try later...", self.raw_node.raft.pending_conf_index);
                    return Ok(());
                }

                if !self.is_leader() {
                    // wrong leader send client cluster data
                    // TODO: retry strategy in case of failure
                    let leader_id = self.get_leader_id();
                    let peers = self.peers.lock().await;
                    let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();

                    chan.send(ServerResponseMsg::ConfigChange {
                        result: ServerConfChangeResponseResult::WrongLeader {
                            leader_id,
                            leader_addr,
                        },
                    })
                    .unwrap();
                } else {
                    let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                    self.response_senders
                        .insert(response_seq, ResponseSender::Server(chan));
                    slog::debug!(
                        self.logger,
                        "Proposed new config change..., seq={response_seq:}, conf_change_v2={}",
                        format_confchangev2(&conf_change)
                    );

                    self.raw_node
                        .propose_conf_change(serialize(&response_seq).unwrap(), conf_change)?;
                }
            }
            ServerRequestMsg::RaftMessage { message } => {
                slog::debug!(
                    self.logger,
                    "Node {} received Raft message from the node {}, Message: {}",
                    self.raw_node.raft.id,
                    message.from,
                    format_message(&message)
                );
                self.raw_node.step(*message)?
            }
            ServerRequestMsg::Propose { proposal, chan } => {
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
                    let raft_response = ServerResponseMsg::Propose {
                        result: ServerResponseResult::WrongLeader {
                            leader_id,
                            leader_addr,
                        },
                    };
                    chan.send(raft_response).unwrap();
                } else {
                    let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
                    self.response_senders
                        .insert(response_seq, ResponseSender::Server(chan));
                    let response_seq = serialize(&response_seq)?;
                    self.raw_node.propose(response_seq, proposal)?;
                }
            }
            ServerRequestMsg::RequestId { chan } => {
                if !self.is_leader() {
                    // TODO: retry strategy in case of failure
                    let leader_id = self.get_leader_id();
                    let peers = self.peers.lock().await;
                    let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();

                    chan.send(ServerResponseMsg::RequestId {
                        result: ServerResponseResult::WrongLeader {
                            leader_id,
                            leader_addr,
                        },
                        leader_addr: None,
                        leader_id: None,
                        reserved_id: None,
                        peers: None,
                    })
                    .unwrap();
                } else {
                    let reserved_id = self.peers.lock().await.reserve_peer(self.get_id());
                    slog::info!(self.logger, "Reserved node id, {}", reserved_id);

                    chan.send(ServerResponseMsg::RequestId {
                        result: ServerResponseResult::Success,
                        reserved_id: Some(reserved_id),
                        leader_id: Some(self.get_id()),
                        leader_addr: None,
                        peers: Some(self.peers.lock().await.clone()),
                    })
                    .unwrap();
                }
            }
            ServerRequestMsg::ReportUnreachable { node_id } => {
                self.raw_node.report_unreachable(node_id);
            }
            ServerRequestMsg::DebugNode { chan } => {
                chan.send(ServerResponseMsg::DebugNode {
                    result: self.inspect().await?,
                })
                .unwrap();
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tick_timer = interval(Duration::from_secs_f32(self.config.tick_interval));

        loop {
            if self.should_exit {
                slog::info!(self.logger, "Node {} quit the cluster.", self.get_id());
                return Ok(());
            }

            if !self.bootstrap_done {
                self.wait_for_initial_peers_bootstrap().await?;
            }

            select! {
                _ = tick_timer.tick() => {
                    self.raw_node.tick();
                }
                msg = self.server_rcv.recv() => {
                    if let Some(message) = msg {
                        self.handle_server_request_msg(message).await?;
                    }
                }
                msg = self.local_rcv.recv() => {
                    if let Some(message) = msg {
                        self.handle_local_request_msg(message).await?;
                    }
                }
            }

            // TODO: Remove this after investigating why tokio::join not failing when one of the Result is Err
            match self.on_ready().await {
                Ok(_) => {}
                Err(e) => {
                    panic!("Error occurred while processing ready: {:?}", e);
                }
            }
        }
    }

    async fn on_ready(&mut self) -> Result<()> {
        if !self.raw_node.has_ready() {
            return Ok(());
        }
        let mut ready = self.raw_node.ready();

        if !ready.messages().is_empty() {
            self.send_messages(ready.take_messages()).await;
        }

        if *ready.snapshot() != Snapshot::default() {
            slog::info!(
                self.logger,
                "Restoring state machine and snapshot metadata..."
            );
            let snapshot = ready.snapshot();
            if !snapshot.get_data().is_empty() {
                let snapshot = FSM::decode(snapshot.get_data()).unwrap();
                self.fsm.restore(snapshot).await?;
            }
            let store = self.raw_node.mut_store();
            store.apply_snapshot(snapshot.clone())?;
        }

        self.handle_committed_entries(ready.take_committed_entries())
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
            self.send_messages(ready.take_persisted_messages()).await;
        }

        let mut light_rd = self.raw_node.advance(ready);

        if let Some(commit) = light_rd.commit_index() {
            let store = self.raw_node.mut_store();
            store.set_hard_state_commit(commit)?;
        }

        self.send_messages(light_rd.take_messages()).await;
        self.handle_committed_entries(light_rd.take_committed_entries())
            .await?;

        self.raw_node.advance_apply();

        Ok(())
    }
}
