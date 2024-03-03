mod bootstrap;
mod response_sender;
pub mod role;
pub mod utils;

use bincode::{deserialize, serialize};
use jopemachine_raft::{eraftpb::ConfChangeTransition, logger::Logger, Storage};
use prost::Message as PMessage;
use std::{
    collections::HashMap,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};
use tonic::Request;

use response_sender::ResponseSender;
use utils::inspect_raftnode;

use crate::{
    create_client,
    error::{Result, SendMessageError},
    raft::{
        eraftpb::{
            ConfChange, ConfChangeSingle, ConfChangeType, ConfChangeV2, Entry, EntryType,
            Message as RaftMessage, Snapshot,
        },
        formatter::{format_confchangev2, format_message},
        raw_node::RawNode,
    },
    raft_service::{self, ChangeConfigResultType, ProposeArgs},
    request_message::{LocalRequestMsg, SelfMessage, ServerRequestMsg},
    response_message::{
        ConfChangeResponseResult, LocalResponseMsg, RequestIdResponseResult, ResponseMessage,
        ResponseResult, ServerResponseMsg,
    },
    storage::{
        heed::HeedStorage,
        utils::{clear_storage_path, ensure_directory_exist, get_data_mdb_path, get_storage_path},
    },
    utils::{to_confchange_v2, OneShotMutex},
    AbstractLogEntry, AbstractStateMachine, ClusterJoinTicket, Config, Error, InitialRole,
    LogStore, Peers, RaftServiceClient,
};

#[derive(Clone)]
pub struct RaftNode<
    LogEntry: AbstractLogEntry + Send + 'static,
    FSM: AbstractStateMachine + Clone + 'static,
> {
    // The lock of RaftNodeCore is locked when RaftNode.run is called and is never released until program terminates.
    // However, to implement the Clone trait in RaftNode, Arc and Mutex are necessary. That's why we use OneShotMutex here.
    inner: Arc<OneShotMutex<RaftNodeCore<LogEntry, FSM>>>,
    local_sender: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
}

impl<
        LogEntry: AbstractLogEntry + Send + 'static,
        FSM: AbstractStateMachine + Clone + Send + 'static,
    > RaftNode<LogEntry, FSM>
{
    #[allow(clippy::too_many_arguments)]
    pub fn bootstrap(
        node_id: u64,
        should_be_leader: bool,
        fsm: FSM,
        config: Config,
        raft_addr: SocketAddr,
        logger: Arc<dyn Logger>,
        server_rcv: mpsc::Receiver<ServerRequestMsg>,
        server_snd: mpsc::Sender<ServerRequestMsg>,
    ) -> Result<Self> {
        let (local_snd, local_rcv) = mpsc::channel(100);

        RaftNodeCore::<LogEntry, FSM>::bootstrap(
            node_id,
            should_be_leader,
            fsm,
            config,
            raft_addr,
            logger,
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
            .send(LocalRequestMsg::GetId { chan: tx })
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

    pub async fn add_peer<A: ToSocketAddrs>(&self, id: u64, addr: A, role: Option<InitialRole>) {
        let addr = addr.to_socket_addrs().unwrap().next().unwrap().to_string();
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::AddPeer {
                id,
                addr,
                role,
                chan: tx,
            })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::AddPeer {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn add_peers(&self, peers: HashMap<u64, SocketAddr>) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::AddPeers { peers, chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::AddPeers {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn inspect(&self) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::DebugNode { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();

        match resp {
            LocalResponseMsg::DebugNode { result_json } => Ok(result_json),
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

    pub async fn propose(&self, proposal: Vec<u8>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Propose {
                proposal: proposal.clone(),
                chan: tx,
            })
            .await
            .unwrap();

        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::Propose { result } => match result {
                ResponseResult::Success => (),
                ResponseResult::Error(e) => return Err(e),
                ResponseResult::WrongLeader { leader_addr, .. } => {
                    let mut client = create_client(leader_addr).await?;
                    client
                        .propose(Request::new(ProposeArgs { msg: proposal }))
                        .await?;
                }
            },
            _ => unreachable!(),
        }
        Ok(())
    }

    pub async fn change_config(&self, conf_change: ConfChangeV2) -> ConfChangeResponseResult {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::ChangeConfig {
                conf_change: conf_change.clone(),
                chan: tx,
            })
            .await
            .unwrap();

        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::ConfigChange { result } => match result {
                ConfChangeResponseResult::WrongLeader { leader_addr, .. } => {
                    let mut client = create_client(leader_addr).await.unwrap();
                    let res = client.change_config(conf_change.clone()).await.unwrap();

                    let result = res.into_inner();

                    if result.result_type
                        == raft_service::ChangeConfigResultType::ChangeConfigSuccess as i32
                    {
                        ConfChangeResponseResult::JoinSuccess {
                            assigned_ids: result.assigned_ids,
                            peers: deserialize(result.peers.as_slice()).unwrap(),
                        }
                    } else {
                        ConfChangeResponseResult::Error(Error::Unknown)
                    }
                }
                _ => result,
            },
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

    pub async fn quit(&self) {
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

    pub async fn transfer_leader(&self, node_id: u64) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::TransferLeader { node_id, chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::TransferLeader {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn campaign(&self) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Campaign { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::Campaign {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn demote(&self, term: u64, leader_id: u64) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Demote {
                term,
                leader_id,
                chan: tx,
            })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::Demote {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn leave(&self) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::Leave { chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::ConfigChange { result: _result } => (),
            _ => unreachable!(),
        }
    }

    pub async fn leave_joint(&self) {
        self.local_sender
            .send(LocalRequestMsg::LeaveJoint {})
            .await
            .unwrap();
    }

    pub async fn send_message(&self, message: RaftMessage) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::SendMessage {
                message: Box::new(message),
                chan: tx,
            })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::SendMessage {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn make_snapshot(&self, index: u64, term: u64) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::MakeSnapshot {
                index,
                term,
                chan: tx,
            })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::MakeSnapshot {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn join_cluster(&self, tickets: Vec<ClusterJoinTicket>) {
        let (tx, rx) = oneshot::channel();
        self.local_sender
            .send(LocalRequestMsg::JoinCluster { tickets, chan: tx })
            .await
            .unwrap();
        let resp = rx.await.unwrap();
        match resp {
            LocalResponseMsg::JoinCluster {} => (),
            _ => unreachable!(),
        }
    }

    pub async fn run(self) -> Result<()> {
        self.inner
            .lock()
            .await
            .expect("RaftNode mutex's owner should be only RaftNode.run!")
            .run()
            .await
    }
}

pub struct RaftNodeCore<
    LogEntry: AbstractLogEntry + Send + 'static,
    FSM: AbstractStateMachine + Clone + 'static,
> {
    pub raw_node: RawNode<HeedStorage>,
    pub fsm: FSM,
    pub peers: Arc<Mutex<Peers>>,
    response_seq: AtomicU64,
    raft_addr: SocketAddr,
    config: Config,
    should_exit: bool,
    last_snapshot_created: Instant,
    logger: Arc<dyn Logger>,
    response_senders: HashMap<u64, ResponseSender<LogEntry, FSM>>,

    server_rcv: mpsc::Receiver<ServerRequestMsg>,
    #[allow(dead_code)]
    server_snd: mpsc::Sender<ServerRequestMsg>,
    local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
    #[allow(dead_code)]
    local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
    self_snd: mpsc::Sender<SelfMessage>,
    self_rcv: mpsc::Receiver<SelfMessage>,

    _phantom_log_entry_typ: PhantomData<LogEntry>,
}

impl<
        LogEntry: AbstractLogEntry + Send + 'static,
        FSM: AbstractStateMachine + Clone + Send + 'static,
    > RaftNodeCore<LogEntry, FSM>
{
    #[allow(clippy::too_many_arguments)]
    pub fn bootstrap(
        node_id: u64,
        should_be_leader: bool,
        fsm: FSM,
        mut config: Config,
        raft_addr: SocketAddr,
        logger: Arc<dyn Logger>,
        server_rcv: mpsc::Receiver<ServerRequestMsg>,
        server_snd: mpsc::Sender<ServerRequestMsg>,
        local_rcv: mpsc::Receiver<LocalRequestMsg<LogEntry, FSM>>,
        local_snd: mpsc::Sender<LocalRequestMsg<LogEntry, FSM>>,
    ) -> Result<Self> {
        config.raft_config.id = node_id;
        config.validate()?;

        let storage_pth = get_storage_path(config.log_dir.as_str(), node_id);

        if let (None, None) = (config.restore_wal_from, config.restore_wal_snapshot_from) {
            clear_storage_path(storage_pth.as_str())?;
            ensure_directory_exist(storage_pth.as_str())?;
        };

        let mut storage = HeedStorage::create(storage_pth.as_str(), &config, logger.clone())?;
        let mut snapshot = storage.snapshot(0, storage.last_index()?)?;
        let conf_state = snapshot.mut_metadata().mut_conf_state();

        let peers = config
            .initial_peers
            .clone()
            .unwrap_or(Peers::new(node_id, raft_addr));

        let voters = peers
            .clone()
            .inner
            .into_iter()
            .filter(|(_, peer)| peer.role == InitialRole::Voter || peer.role == InitialRole::Leader)
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        let learners = peers
            .clone()
            .inner
            .into_iter()
            .filter(|(_, peer)| peer.role == InitialRole::Learner)
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        if conf_state.voters.is_empty() {
            conf_state.set_voters(voters);
            conf_state.set_learners(learners);
        }

        match (config.restore_wal_from, config.restore_wal_snapshot_from) {
            (Some(restore_wal_from), None) => {
                if restore_wal_from != node_id {
                    std::fs::copy(
                        get_data_mdb_path(config.log_dir.as_str(), restore_wal_from),
                        get_data_mdb_path(config.log_dir.as_str(), node_id),
                    )?;
                }
            }
            (None, Some(restore_wal_snapshot_from)) => {
                if restore_wal_snapshot_from != node_id {
                    std::fs::copy(
                        get_data_mdb_path(config.log_dir.as_str(), restore_wal_snapshot_from),
                        get_data_mdb_path(config.log_dir.as_str(), node_id),
                    )?;
                }
                storage.apply_snapshot(snapshot)?;
            }
            (Some(_), Some(_)) => {
                unreachable!()
            }
            _ => {
                storage.apply_snapshot(snapshot)?;
            }
        }

        let mut raw_node = RawNode::new(&config.raft_config, storage, logger.clone())?;
        let response_seq = AtomicU64::new(0);
        let last_snapshot_created = Instant::now();

        let (self_snd, self_rcv) = mpsc::channel(100);

        if should_be_leader {
            raw_node.raft.become_candidate();
            raw_node.raft.become_leader();
        }

        Ok(RaftNodeCore {
            raw_node,
            fsm,
            response_seq,
            config,
            raft_addr,
            logger,
            last_snapshot_created,
            should_exit: false,
            peers: Arc::new(Mutex::new(peers)),
            response_senders: HashMap::new(),
            server_rcv,
            server_snd,
            local_rcv,
            local_snd,
            self_snd,
            self_rcv,
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

    pub async fn add_peer<A: ToSocketAddrs>(
        &mut self,
        id: u64,
        addr: A,
        role: Option<InitialRole>,
    ) -> Result<()> {
        let mut peers = self.peers.lock().await;
        peers.add_peer(id, addr, role);
        peers.connect(id).await
    }

    pub async fn add_peers(&mut self, peers: HashMap<u64, SocketAddr>) -> Result<()> {
        for (id, peer_addr) in peers.iter() {
            self.add_peer(id.to_owned(), *peer_addr, None).await?;
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

    pub async fn inspect(&self) -> Result<String> {
        inspect_raftnode(&self.raw_node)
    }

    async fn send_message(
        message: RaftMessage,
        peers: Arc<Mutex<Peers>>,
        snd: mpsc::Sender<SelfMessage>,
        logger: Arc<dyn Logger>,
    ) {
        let node_id = message.get_to();

        let mut ok = std::result::Result::<(), SendMessageError>::Ok(());

        let client = match peers.lock().await.get_mut(&node_id) {
            Some(peer) => {
                if peer.client.is_none() {
                    if let Err(e) = peer.connect().await {
                        logger.debug(format!("Connection error: {:?}", e).as_str());
                        ok = Err(SendMessageError::ConnectionError(node_id.to_string()));
                    }
                }
                peer.client.clone()
            }
            None => {
                ok = Err(SendMessageError::PeerNotFound(node_id.to_string()));
                None
            }
        };

        if let Some(mut client) = client {
            let message = Request::new(message.clone());
            if let Err(e) = client.send_message(message).await {
                logger.trace(&format!("Message transmission error: {:?}", e));
                ok = Err(SendMessageError::TransmissionError(node_id.to_string()));
            }
        }

        if let Err(e) = ok {
            logger.debug(&format!("Error occurred while sending message: {}", e));
            snd.send(SelfMessage::ReportUnreachable { node_id })
                .await
                .unwrap();
        }
    }

    async fn send_messages(&mut self, messages: Vec<RaftMessage>) {
        for message in messages {
            tokio::spawn(RaftNodeCore::<LogEntry, FSM>::send_message(
                message,
                self.peers.clone(),
                self.self_snd.clone(),
                self.logger.clone(),
            ));
        }
    }

    async fn handle_committed_entries(&mut self, committed_entries: Vec<Entry>) -> Result<()> {
        for entry in committed_entries {
            match entry.get_entry_type() {
                EntryType::EntryNormal => {
                    if entry.get_data().is_empty() {
                        continue;
                    }

                    self.handle_committed_normal_entry(&entry).await?;
                }
                EntryType::EntryConfChange | EntryType::EntryConfChangeV2 => {
                    self.handle_committed_config_change_entry(&entry).await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_join(&mut self, tickets: Vec<ClusterJoinTicket>) -> Result<()> {
        let mut cc_v2 = ConfChangeV2::default();
        let mut changes = vec![];
        let mut addrs = vec![];
        // TODO: Find more wise way to do this.
        let peer_addr = tickets[0].leader_addr.clone();

        if tickets.len() > 1 {
            cc_v2.set_transition(ConfChangeTransition::Explicit);
        }

        for ticket in tickets {
            let mut cs = ConfChangeSingle::default();
            cs.set_change_type(ConfChangeType::AddNode);
            cs.set_node_id(ticket.reserved_id);
            changes.push(cs);
            addrs.push(
                SocketAddr::from_str(ticket.raft_addr.as_str()).expect("Invalid socket address!"),
            );
        }

        cc_v2.set_changes(changes);
        cc_v2.set_context(serialize(&addrs)?);

        let mut leader_client = RaftServiceClient::connect(format!("http://{}", peer_addr)).await?;
        let response = leader_client
            .change_config(cc_v2.clone())
            .await?
            .into_inner();

        match response.result_type() {
            ChangeConfigResultType::ChangeConfigSuccess => Ok(()),
            ChangeConfigResultType::ChangeConfigUnknownError => Err(Error::JoinError),
            ChangeConfigResultType::ChangeConfigRejected => {
                Err(Error::Rejected("Confchange request rejected".to_string()))
            }
            ChangeConfigResultType::ChangeConfigTimeoutError => Err(Error::Timeout),
            ChangeConfigResultType::ChangeConfigWrongLeader => {
                // Should be handled in RaftServiceClient
                unreachable!()
            }
        }
    }

    async fn handle_committed_normal_entry(&mut self, entry: &Entry) -> Result<()> {
        let response_seq: u64 = deserialize(entry.get_context())?;
        let _data = self.fsm.apply(entry.get_data().to_vec()).await?;

        if let Some(sender) = self.response_senders.remove(&response_seq) {
            match sender {
                ResponseSender::Local(sender) => {
                    sender
                        .send(LocalResponseMsg::Propose {
                            result: ResponseResult::Success,
                        })
                        .unwrap();
                }
                ResponseSender::Server(sender) => {
                    sender
                        .send(ServerResponseMsg::Propose {
                            result: ResponseResult::Success,
                        })
                        .unwrap();
                }
            }
        }

        if let Some(snapshot_interval) = self.config.snapshot_interval {
            if Instant::now()
                > self.last_snapshot_created + Duration::from_secs_f32(snapshot_interval)
            {
                self.make_snapshot(entry.index, entry.term).await?;
            }
        }

        Ok(())
    }

    async fn handle_committed_config_change_entry(&mut self, entry: &Entry) -> Result<()> {
        if entry.get_context().is_empty() {
            let conf_change_v2 = match entry.get_entry_type() {
                EntryType::EntryConfChange => {
                    to_confchange_v2(ConfChange::decode(entry.get_data())?)
                }
                EntryType::EntryConfChangeV2 => ConfChangeV2::decode(entry.get_data())?,
                _ => unreachable!(),
            };

            let cs = self.raw_node.apply_conf_change(&conf_change_v2)?;
            let store = self.raw_node.mut_store();
            store.set_conf_state(&cs)?;
            return Ok(());
        }

        let conf_change_v2 = match entry.get_entry_type() {
            EntryType::EntryConfChange => to_confchange_v2(ConfChange::decode(entry.get_data())?),
            EntryType::EntryConfChangeV2 => ConfChangeV2::decode(entry.get_data())?,
            _ => unreachable!(),
        };

        let conf_changes = conf_change_v2.get_changes();
        let addrs: Vec<SocketAddr> = deserialize(conf_change_v2.get_context())?;

        for (cc_idx, conf_change) in conf_changes.iter().enumerate() {
            let node_id = conf_change.get_node_id();
            let change_type = conf_change.get_change_type();

            match change_type {
                ConfChangeType::AddNode => {
                    let addr = addrs[cc_idx];
                    self.logger.info(&format!(
                        "Node {} ({}) joined the cluster as voter.",
                        node_id, addr
                    ));
                    self.peers.lock().await.add_peer(
                        node_id,
                        &addr.to_string(),
                        Some(InitialRole::Voter),
                    );
                }
                ConfChangeType::AddLearnerNode => {
                    let addr = addrs[cc_idx];
                    self.logger.info(&format!(
                        "Node {} ({}) joined the cluster as learner.",
                        node_id, addr
                    ));
                    self.peers.lock().await.add_peer(
                        node_id,
                        &addr.to_string(),
                        Some(InitialRole::Learner),
                    );
                }
                ConfChangeType::RemoveNode => {
                    if node_id == self.get_id() {
                        self.should_exit = true;
                        self.logger
                            .info(&format!("Node {} quit the cluster.", node_id));
                    } else {
                        self.logger
                            .info(&format!("Node {} removed from the cluster.", node_id));
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
                self.logger.error(&format!(
                    "Failed to apply the configuration change. Error: {:?}",
                    e
                ));
            }
        }

        let response_seq: AtomicU64 = deserialize(entry.get_context())?;

        let response_seq_value = response_seq.load(Ordering::Relaxed);

        if let Some(sender) = self.response_senders.remove(&response_seq_value) {
            #[allow(unused_assignments)]
            let mut response = ConfChangeResponseResult::Error(Error::Unknown);

            if conf_changes.iter().all(|cc| {
                cc.get_change_type() == ConfChangeType::AddNode
                    || cc.get_change_type() == ConfChangeType::AddLearnerNode
            }) {
                response = ConfChangeResponseResult::JoinSuccess {
                    assigned_ids: conf_changes.iter().map(|cc| cc.get_node_id()).collect(),
                    peers: self.peers.lock().await.clone(),
                };
            } else if conf_changes
                .iter()
                .all(|cc| cc.get_change_type() == ConfChangeType::RemoveNode)
            {
                response = ConfChangeResponseResult::RemoveSuccess;
            } else {
                // TODO: Handle mixed cases
                unimplemented!();
            }

            match sender {
                ResponseSender::Local(sender) => {
                    sender
                        .send(LocalResponseMsg::ConfigChange { result: response })
                        .unwrap();
                }
                ResponseSender::Server(sender) => {
                    sender
                        .send(ServerResponseMsg::ConfigChange { result: response })
                        .unwrap();
                }
            }
        }

        Ok(())
    }

    async fn handle_propose_request(
        &mut self,
        proposal: Vec<u8>,
        chan: ResponseSender<LogEntry, FSM>,
    ) -> Result<()> {
        if !self.is_leader() {
            let leader_id = self.get_leader_id();
            if leader_id == 0 {
                self.logger
                    .error("There is no leader in the cluster at the time. try later...");
                return Ok(());
            }

            // leader can't be an empty node
            let leader_addr = self
                .peers
                .lock()
                .await
                .get(&leader_id)
                .unwrap()
                .addr
                .to_string();

            let raft_response: ResponseMessage<LogEntry, FSM> = match chan {
                ResponseSender::Local(_) => LocalResponseMsg::Propose {
                    result: ResponseResult::WrongLeader {
                        leader_id,
                        leader_addr,
                    },
                }
                .into(),
                ResponseSender::Server(_) => ServerResponseMsg::Propose {
                    result: ResponseResult::WrongLeader {
                        leader_id,
                        leader_addr,
                    },
                }
                .into(),
            };

            chan.send(raft_response);
        } else {
            let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);
            match chan {
                ResponseSender::Local(chan) => {
                    self.response_senders
                        .insert(response_seq, ResponseSender::Local(chan));
                }
                ResponseSender::Server(chan) => {
                    self.response_senders
                        .insert(response_seq, ResponseSender::Server(chan));
                }
            };

            self.raw_node.propose(serialize(&response_seq)?, proposal)?;
        }

        Ok(())
    }

    async fn handle_confchange_request(
        &mut self,
        conf_change: ConfChangeV2,
        chan: ResponseSender<LogEntry, FSM>,
    ) -> Result<()> {
        if self.raw_node.raft.has_pending_conf() {
            self.logger.warn(&format!("Reject the conf change because pending conf change exist! (pending_conf_index={}), try later...", self.raw_node.raft.pending_conf_index));
            return Ok(());
        }

        if !self.is_leader() {
            let leader_id = self.get_leader_id();
            if leader_id == 0 {
                self.logger
                    .error("There is no leader in the cluster at the time. try later...");
                return Ok(());
            }

            let peers = self.peers.lock().await;
            let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();

            let wrong_leader_result = ConfChangeResponseResult::WrongLeader {
                leader_id,
                leader_addr,
            };

            match chan {
                ResponseSender::Local(chan) => chan
                    .send(LocalResponseMsg::ConfigChange {
                        result: wrong_leader_result,
                    })
                    .unwrap(),
                ResponseSender::Server(chan) => chan
                    .send(ServerResponseMsg::ConfigChange {
                        result: wrong_leader_result,
                    })
                    .unwrap(),
            }
        } else {
            let response_seq = self.response_seq.fetch_add(1, Ordering::Relaxed);

            match chan {
                ResponseSender::Local(chan) => {
                    self.response_senders
                        .insert(response_seq, ResponseSender::Local(chan));
                }
                ResponseSender::Server(chan) => {
                    self.response_senders
                        .insert(response_seq, ResponseSender::Server(chan));
                }
            };

            self.logger.debug(&format!(
                "Proposed new config change..., seq={}, conf_change_v2={}",
                response_seq,
                format_confchangev2(&conf_change)
            ));

            self.raw_node
                .propose_conf_change(serialize(&response_seq).unwrap(), conf_change)?;
        }

        Ok(())
    }

    async fn handle_local_request_msg(
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
            LocalRequestMsg::AddPeer {
                id,
                addr,
                role,
                chan,
            } => {
                self.add_peer(id, addr, role).await?;
                chan.send(LocalResponseMsg::AddPeer {}).unwrap();
            }
            LocalRequestMsg::AddPeers { peers, chan } => {
                self.add_peers(peers).await?;
                chan.send(LocalResponseMsg::AddPeers {}).unwrap();
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
            LocalRequestMsg::DebugNode { chan } => {
                chan.send(LocalResponseMsg::DebugNode {
                    result_json: self.inspect().await?,
                })
                .unwrap();
            }
            LocalRequestMsg::Propose { proposal, chan } => {
                self.handle_propose_request(proposal, ResponseSender::Local(chan))
                    .await?;
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
            LocalRequestMsg::Campaign { chan } => {
                self.raw_node.campaign()?;
                chan.send(LocalResponseMsg::Campaign {}).unwrap();
            }
            LocalRequestMsg::Demote {
                chan,
                term,
                leader_id,
            } => {
                self.raw_node.raft.become_follower(term, leader_id);
                chan.send(LocalResponseMsg::Demote {}).unwrap();
            }
            LocalRequestMsg::Leave { chan } => {
                let mut conf_change = ConfChange::default();
                conf_change.set_node_id(self.get_id());
                conf_change.set_change_type(ConfChangeType::RemoveNode);
                conf_change.set_context(serialize(&vec![self.raft_addr]).unwrap());

                self.handle_confchange_request(
                    to_confchange_v2(conf_change),
                    ResponseSender::Local(chan),
                )
                .await?;
            }
            LocalRequestMsg::ChangeConfig { conf_change, chan } => {
                self.handle_confchange_request(conf_change, ResponseSender::Local(chan))
                    .await?;
            }
            LocalRequestMsg::MakeSnapshot { index, term, chan } => {
                self.make_snapshot(index, term).await?;
                chan.send(LocalResponseMsg::MakeSnapshot {}).unwrap();
            }
            LocalRequestMsg::JoinCluster { tickets, chan } => {
                self.handle_join(tickets).await?;
                chan.send(LocalResponseMsg::JoinCluster {}).unwrap();
            }
            LocalRequestMsg::SendMessage { message, chan } => {
                self.logger.debug(&format!(
                    "Node {} received Raft message from itself, Message: {}",
                    self.raw_node.raft.id,
                    format_message(&message)
                ));
                self.raw_node.step(*message)?;
                chan.send(LocalResponseMsg::SendMessage {}).unwrap();
            }
            LocalRequestMsg::LeaveJoint {} => {
                let zero = ConfChangeV2::default();
                self.raw_node.propose_conf_change(vec![], zero)?;
            }
            LocalRequestMsg::TransferLeader { node_id, chan } => {
                self.raw_node.transfer_leader(node_id);
                chan.send(LocalResponseMsg::TransferLeader {}).unwrap();
            }
        }

        Ok(())
    }

    async fn handle_self_message(&mut self, message: SelfMessage) -> Result<()> {
        match message {
            SelfMessage::ReportUnreachable { node_id } => {
                self.raw_node.report_unreachable(node_id);
            }
        }

        Ok(())
    }

    async fn handle_server_request_msg(&mut self, message: ServerRequestMsg) -> Result<()> {
        match message {
            ServerRequestMsg::ChangeConfig { conf_change, chan } => {
                self.handle_confchange_request(conf_change, ResponseSender::Server(chan))
                    .await?;
            }
            ServerRequestMsg::SendMessage { message } => {
                self.logger.debug(&format!(
                    "Node {} received Raft message from the node {}, {}",
                    self.raw_node.raft.id,
                    message.from,
                    format_message(&message)
                ));
                self.raw_node.step(*message)?
            }
            ServerRequestMsg::Propose { proposal, chan } => {
                self.handle_propose_request(proposal, ResponseSender::Server(chan))
                    .await?;
            }
            ServerRequestMsg::RequestId { raft_addr, chan } => {
                if !self.is_leader() {
                    let leader_id = self.get_leader_id();
                    if leader_id == 0 {
                        self.logger
                            .error("There is no leader in the cluster at the time. try later...");
                        return Ok(());
                    }

                    let peers = self.peers.lock().await;
                    let leader_addr = peers.get(&leader_id).unwrap().addr.to_string();

                    chan.send(ServerResponseMsg::RequestId {
                        result: RequestIdResponseResult::WrongLeader {
                            leader_id,
                            leader_addr,
                        },
                    })
                    .unwrap();
                } else {
                    let mut peers = self.peers.lock().await;

                    let reserved_id =
                        if let Some(existing_node_id) = peers.get_node_id_by_addr(raft_addr) {
                            self.logger
                                .info(&format!("Node {} connection restored.", existing_node_id));
                            existing_node_id
                        } else {
                            let reserved = peers.reserve_id();
                            self.logger.info(&format!(
                                "Node {} reserved new node_id {}.",
                                self.get_id(),
                                reserved
                            ));
                            reserved
                        };

                    chan.send(ServerResponseMsg::RequestId {
                        result: RequestIdResponseResult::Success {
                            reserved_id,
                            leader_id: self.get_id(),
                            peers: peers.clone(),
                        },
                    })
                    .unwrap();
                }
            }
            ServerRequestMsg::LeaveJoint { chan } => {
                self.raw_node
                    .propose_conf_change(vec![], ConfChangeV2::default())?;
                chan.send(ServerResponseMsg::LeaveJoint {}).unwrap();
            }
            ServerRequestMsg::DebugNode { chan } => {
                chan.send(ServerResponseMsg::DebugNode {
                    result_json: self.inspect().await?,
                })
                .unwrap();
            }
            ServerRequestMsg::GetPeers { chan } => {
                chan.send(ServerResponseMsg::GetPeers {
                    peers: self.get_peers().await,
                })
                .unwrap();
            }
            ServerRequestMsg::CreateSnapshot { chan } => {
                let last_index = self.raw_node.store().last_index()?;
                let last_term = self.raw_node.store().hard_state()?.term;
                self.make_snapshot(last_index, last_term).await?;
                chan.send(ServerResponseMsg::CreateSnapshot {}).unwrap();
            }
            ServerRequestMsg::SetPeers { chan, peers } => {
                self.peers.lock().await.replace(peers);
                chan.send(ServerResponseMsg::SetPeers {}).unwrap();
            }
        }

        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let mut tick_timer = Duration::from_secs_f32(self.config.tick_interval);
        let fixed_tick_timer = tick_timer;
        let mut now = Instant::now();

        loop {
            if self.should_exit {
                self.logger
                    .info(&format!("Node {} quit the cluster.", self.get_id()));
                return Ok(());
            }

            tokio::select! {
                msg = timeout(fixed_tick_timer, self.self_rcv.recv()) => {
                    if let Ok(Some(msg)) = msg {
                        self.handle_self_message(msg).await?;
                    }
                }
                msg = timeout(fixed_tick_timer, self.server_rcv.recv()) => {
                    if let Ok(Some(msg)) = msg {
                        self.handle_server_request_msg(msg).await?;
                    }
                }
                msg = timeout(fixed_tick_timer, self.local_rcv.recv()) => {
                    if let Ok(Some(msg)) = msg {
                        self.handle_local_request_msg(msg).await?;
                    }
                }
            }

            let elapsed = now.elapsed();
            now = Instant::now();
            if elapsed > tick_timer {
                tick_timer = fixed_tick_timer;
                self.raw_node.tick();
            } else {
                tick_timer -= elapsed;
            }

            self.on_ready().await?
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
            self.logger
                .info("Restoring state machine and snapshot metadata...");
            let snapshot = ready.snapshot();
            if !snapshot.get_data().is_empty() {
                self.fsm.restore(snapshot.get_data().to_vec()).await?;
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

        if !light_rd.messages().is_empty() {
            self.send_messages(light_rd.take_messages()).await;
        }

        self.handle_committed_entries(light_rd.take_committed_entries())
            .await?;

        self.raw_node.advance_apply();

        Ok(())
    }
}
