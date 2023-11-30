import asyncio
import json
import math
import os
import time
from asyncio import Queue
from typing import Any, Callable, Optional

import grpc
import rraft
from rraft import (
    ConfChange,
    ConfChangeTransition,
    ConfChangeType,
    ConfChangeV2,
    Entry,
    EntryRef,
    EntryType,
    Logger,
    LoggerRef,
    Message,
    RawNode,
    Snapshot,
    Storage,
)

from .codec.abc import AbstractCodec
from .config import RaftifyConfig
from .error import UnknownError
from .logger import AbstractRaftifyLogger
from .pb_adapter import ConfChangeV2Adapter, MessageAdapter
from .peers import Peer, Peers, PeerState
from .protos.raft_service_pb2 import RerouteMsgType
from .raft_client import RaftClient
from .raft_server import RaftServer
from .request_message import (
    ApplyConfigChangeForcelyReqMessage,
    ClusterBootstrapReadyReqMessage,
    ConfigChangeReqMessage,
    DebugEntriesReqMessage,
    DebugNodeReqMessage,
    GetPeersReqMessage,
    MemberBootstrapReadyReqMessage,
    ProposeReqMessage,
    RaftReqMessage,
    ReportUnreachableReqMessage,
    RequestIdReqMessage,
    RerouteToLeaderReqMessage,
    VersionReqMessage,
)
from .response_message import (
    ClusterBootstrapReadyRespMessage,
    ConfChangeRejectMessage,
    IdReservedRespMessage,
    JoinSuccessRespMessage,
    MemberBootstrapReadyRespMessage,
    PeerRemovalSuccessRespMessage,
    RaftRespMessage,
    ResponseMessage,
    WrongLeaderRespMessage,
)
from .rraft_deserializer import entry_data_deserializer, pickle_deserialize
from .state_machine.abc import AbstractStateMachine
from .storage.lmdb import LMDBStorage
from .utils import AtomicInteger, SocketAddr


class RaftNode:
    response_queues: dict[AtomicInteger, Queue]
    """
    Queues that are stored for each message to handle asynchronous requests
    """

    prev_leader_id: Optional[int] = None
    prev_state_role: Optional[rraft.StateRole] = None

    on_leader_id_change: Optional[Callable[[int], None]] = None
    on_state_role_change: Optional[Callable[[rraft.StateRole], None]] = None
    """
    Callback functions that is called when the soft state of the node changes.
    """

    def __init__(
        self,
        *,
        raw_node: RawNode,
        raft_server: RaftServer,
        peers: Peers,
        message_queue: Queue,
        fsm: AbstractStateMachine,
        lmdb: LMDBStorage,
        storage: Storage,
        response_seq: AtomicInteger,
        last_snapshot_created: float,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        bootstrap_done: bool,
        codec: AbstractCodec,
    ):
        self.raw_node = raw_node
        self.raft_server = raft_server
        self.peers = peers
        self.message_queue = message_queue
        self.fsm = fsm
        self.lmdb = lmdb
        self.storage = storage
        self.response_seq = response_seq
        self.last_snapshot_created = last_snapshot_created
        self.logger = logger
        self.should_exit = False
        self.raftify_cfg = raftify_cfg
        self.bootstrap_done = bootstrap_done
        self.codec = codec
        self.response_queues = {}

    @classmethod
    def bootstrap_cluster(
        cls,
        *,
        message_queue: Queue,
        fsm: AbstractStateMachine,
        raft_server: RaftServer,
        initial_peers: Peers,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        bootstrap_done: bool,
        codec: AbstractCodec,
    ) -> "RaftNode":
        """
        Create new RaftCluster and make this RaftNode as a leader.
        """
        # TODO: Support the below case.
        assert (
            len(initial_peers) != 1
        ), "The number of initial peers should be more than 2 or equals to 0."

        cfg = raftify_cfg.raft_config

        cfg.set_id(1)
        cfg.validate()

        lmdb = LMDBStorage.create(
            map_size=raftify_cfg.lmdb_map_size,
            log_dir_path=raftify_cfg.log_dir,
            compacted_log_dir_path=raftify_cfg.compacted_log_dir,
            compacted_logs_size_threshold=raftify_cfg.compacted_logs_size_threshold,
            cluster_id=raftify_cfg.cluster_id,
            node_id=1,
            logger=logger,
        )

        snapshot = Snapshot.default()
        snapshot.get_metadata().set_index(0)
        snapshot.get_metadata().set_term(0)
        snapshot.get_metadata().get_conf_state().set_voters([1])

        lmdb.apply_snapshot(snapshot)

        storage = Storage(lmdb)
        raw_node = RawNode(cfg, storage, slog)

        response_seq = AtomicInteger(0)
        last_snapshot_created = time.time()

        raw_node.get_raft().become_candidate()
        raw_node.get_raft().become_leader()

        return cls(
            raw_node=raw_node,
            raft_server=raft_server,
            peers=initial_peers,
            message_queue=message_queue,
            fsm=fsm,
            lmdb=lmdb,
            storage=storage,
            response_seq=response_seq,
            last_snapshot_created=last_snapshot_created,
            logger=logger,
            raftify_cfg=raftify_cfg,
            bootstrap_done=bootstrap_done,
            codec=codec,
        )

    @classmethod
    def new_follower(
        cls,
        *,
        message_queue: Queue,
        id: int,
        fsm: AbstractStateMachine,
        raft_server: RaftServer,
        peers: Peers,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        bootstrap_done: bool,
        codec: AbstractCodec,
    ) -> "RaftNode":
        """
        Add new RaftNode to the existing RaftCluster and bootstrap this RaftNode as a follower.
        """
        cfg = raftify_cfg.raft_config

        cfg.set_id(id)
        cfg.validate()

        lmdb = LMDBStorage.create(
            map_size=raftify_cfg.lmdb_map_size,
            log_dir_path=raftify_cfg.log_dir,
            compacted_log_dir_path=raftify_cfg.compacted_log_dir,
            compacted_logs_size_threshold=raftify_cfg.compacted_logs_size_threshold,
            cluster_id=raftify_cfg.cluster_id,
            node_id=id,
            logger=logger,
        )

        storage = Storage(lmdb)
        raw_node = RawNode(cfg, storage, slog)

        response_seq = AtomicInteger(0)
        last_snapshot_created = time.time()

        return cls(
            raw_node=raw_node,
            raft_server=raft_server,
            peers=peers,
            message_queue=message_queue,
            fsm=fsm,
            lmdb=lmdb,
            storage=storage,
            response_seq=response_seq,
            last_snapshot_created=last_snapshot_created,
            logger=logger,
            raftify_cfg=raftify_cfg,
            bootstrap_done=bootstrap_done,
            codec=codec,
        )

    def get_id(self) -> int:
        return self.raw_node.get_raft().get_id()

    def get_leader_id(self) -> int:
        return self.raw_node.get_raft().get_leader_id()

    def is_leader(self) -> bool:
        return self.get_id() == self.get_leader_id()

    def transfer_leader(
        self,
        node_id: int,
    ) -> bool:
        """
        Handle Leader transfer.
        """
        if not self.is_leader():
            self.logger.warning("LeaderTransfer requested but not leader!")
            return False

        self.raw_node.transfer_leader(node_id)
        return True

    def remove_node(self, node_id: int) -> None:
        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_context(self.codec.encode([self.peers[node_id].addr]))
        conf_change.set_change_type(ConfChangeType.RemoveNode)

        conf_change_v2 = conf_change.as_v2()
        conf_change_v2.set_transition(ConfChangeTransition.Auto)

        leader_id = self.raw_node.get_raft().get_id()
        asyncio.create_task(
            self.peers[leader_id].client.apply_change_config_forcely(conf_change_v2)
        )

    def inspect(self) -> dict[str, Any]:
        """
        Collect and return lots of useful information for debugging
        """

        progress_trackers = self.raw_node.get_raft().prs().collect()
        hs = self.lmdb.hard_state()
        cs = self.lmdb.conf_state()
        snapshot = self.lmdb.snapshot(0, 0)
        snapshot_dict = snapshot.to_dict()
        snapshot_dict["data"] = pickle_deserialize(snapshot.get_data())

        return {
            "node_id": self.raw_node.get_raft().get_id(),
            "current_leader_id": self.raw_node.get_raft().get_leader_id(),
            "storage": {
                "hard_state": hs.to_dict(),
                "conf_state": cs.to_dict(),
                "snapshot": snapshot_dict,
                "last_index": self.lmdb.last_index(),
            },
            "progress": [
                pr_tracker.progress().to_dict() for pr_tracker in progress_trackers
            ],
            "raft": {
                "term": self.raw_node.get_raft().get_term(),
            },
            "raft_log": {
                "applied": self.raw_node.get_raft().get_raft_log().get_applied(),
                "committed": self.raw_node.get_raft().get_raft_log().get_committed(),
                "persisted": self.raw_node.get_raft().get_raft_log().get_persisted(),
            },
            "failure": {
                "pending_conf_index": self.raw_node.get_raft().get_pending_conf_index(),
                "has_pending_conf": self.raw_node.get_raft().has_pending_conf(),
            },
            "peer_states": {
                node_id: peer.to_dict() for node_id, peer in self.peers.items()
            },
        }

    async def report_node_unreachable(self, node_id: int) -> None:
        try:
            await self.message_queue.put(ReportUnreachableReqMessage(node_id))
        except Exception as e:
            self.logger.error(str(e))
            pass

    def get_elapsed_time_from_first_connection_lost(self, node_id: int) -> float:
        peer = self.peers.get(node_id)

        if not peer:
            return math.nan

        failed_client = peer.client
        assert failed_client is not None

        if failed_client.first_failed_time is None:
            failed_client.first_failed_time = time.time()

        assert failed_client.first_failed_time is not None
        elapsed = time.time() - failed_client.first_failed_time

        return round(elapsed, 4)

    def handle_node_auto_removal(self, elapsed: float, node_id: int) -> None:
        assert self.is_leader()

        if node_id not in self.peers:
            return

        if elapsed >= self.raftify_cfg.node_auto_remove_threshold:
            if self.raftify_cfg.auto_remove_node:
                peer = self.peers[node_id]

                if peer.state == PeerState.Connected:
                    self.remove_node(node_id)

                    self.logger.error(
                        f"Removing node {node_id} from the cluster "
                        f"automatically "
                        f"because the requests to the connection kept failed."
                    )

    def should_retry(self, node_id: int, current_retry_count: int) -> bool:
        """ """
        if node_id not in self.peers:
            return False

        return current_retry_count < self.raftify_cfg.max_retry_cnt

    async def leave_joint(self):
        """
        Force Empty ConfChange entry to be committed for leaving joint consensus state.
        """
        # TODO: Execute commit on the more appropriate timing.
        # If possible, it would be great to switch to use "Auto" confchange transition.
        await asyncio.sleep(1)
        zero = ConfChangeV2.default()
        assert zero.leave_joint(), "Zero ConfChangeV2 must be empty"
        self.raw_node.propose_conf_change_v2(b"", zero)

    def get_all_entry_logs(self) -> dict[str, Any]:
        """
        Collect and return all entries in the raft log
        """
        persisted_entries = self.lmdb.all_entries()
        persisted_entries_dicts = []

        # TODO: Improve below logic to avoid code duplication
        for entry in persisted_entries:
            entry_dict = entry.to_dict()
            entry_dict["data"] = entry_data_deserializer(entry.get_data())
            entry_dict["context"] = pickle_deserialize(entry.get_context())
            persisted_entries_dicts.append(entry_dict)

        compacted_logs_path = os.path.join(
            self.lmdb.compacted_log_dir_path, "compacted-logs.json"
        )

        if os.path.exists(compacted_logs_path):
            with open(compacted_logs_path, "r", encoding="utf-8") as file:
                compacted_all_entries = json.load(file)
        else:
            compacted_all_entries = []

        return {
            "persisted_entries": persisted_entries_dicts,
            "compacted_all_entries": compacted_all_entries,
        }

    async def send_message(self, client: RaftClient, message: Message) -> None:
        """
        Attempt to send a message through the given client.
        """

        node_id = message.get_to()
        current_retry_count = 0

        while True:
            try:
                if self.bootstrap_done:
                    await client.send_message(
                        message, timeout=self.raftify_cfg.message_timeout
                    )

                    self.peers[node_id].client.first_failed_time = None
                return
            except (grpc.aio.AioRpcError, asyncio.TimeoutError) as err:
                if not isinstance(err, asyncio.TimeoutError):
                    if err.code() != grpc.StatusCode.UNAVAILABLE:
                        raise

                await self.report_node_unreachable(node_id)
                elapsed = self.get_elapsed_time_from_first_connection_lost(node_id)

                if elapsed < self.raftify_cfg.node_auto_remove_threshold:
                    self.logger.warning(
                        f"Failed to connect to node {node_id} "
                        f"elapsed from first failure: {format(elapsed, '.4f')}s. Err message: {str(err)}"
                    )

                if self.is_leader():
                    self.handle_node_auto_removal(elapsed, node_id)

                if not isinstance(err, asyncio.TimeoutError):
                    await asyncio.sleep(self.raftify_cfg.message_timeout)

                if self.should_retry(message.get_to(), current_retry_count):
                    current_retry_count += 1
                    continue
                else:
                    return
            # Unknown error reraising
            except Exception:
                raise

    def get_version(self):
        current_dir = os.path.dirname(os.path.abspath(__file__))

        version_file_path = os.path.join(current_dir, "VERSION")

        try:
            with open(version_file_path, "r") as file:
                version = file.read().strip()
                return version
        except FileNotFoundError:
            return "unknown"

    def send_messages(self, messages: list[Message]):
        for message in messages:
            if peer := self.peers.get(message.get_to()):
                if client := peer.client:
                    asyncio.create_task(
                        self.send_message(
                            client,
                            message,
                        )
                    )

    async def send_wrongleader_response(self, channel: Queue) -> None:
        if self.get_leader_id() not in self.peers:
            self.logger.warning(
                "Request failed due to leader node not found in peers!"
                f"leader: {self.get_leader_id()}, peers: {self.peers.keys()}"
            )
            return

        # # TODO: Make this follower to new cluster's leader
        # assert (
        #     self.get_leader_id() in self.peers
        # ), f"Leader node not found in peers!, leader: {self.get_leader_id()}, peers: {self.peers}"

        try:
            # TODO: handle error here
            await channel.put(
                WrongLeaderRespMessage(
                    leader_id=self.get_leader_id(),
                    leader_addr=str(self.peers[self.get_leader_id()].addr),
                )
            )
        except Exception:
            pass

    async def handle_committed_entries(
        self,
        committed_entries: list[Entry] | list[EntryRef],
    ) -> None:
        # TODO: persist last_applied_index and implement log entries commit resuming logic
        # _last_apply_index = 0

        for entry in committed_entries:
            match entry.get_entry_type():
                case EntryType.EntryNormal:
                    await self.handle_committed_normal_entry(entry)
                case EntryType.EntryConfChange | EntryType.EntryConfChangeV2:
                    await self.handle_committed_config_change_entry(entry)
                case _:
                    raise UnknownError()

    async def create_snapshot(self, index: int, term: int) -> None:
        self.last_snapshot_created = time.time()
        snapshot_data = await self.fsm.snapshot()

        last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
        self.lmdb.compact(last_applied)
        self.lmdb.create_snapshot(snapshot_data, index, term)
        self.logger.info("Snapshot created successfully.")

    async def handle_committed_normal_entry(self, entry: Entry | EntryRef) -> None:
        if not entry.get_data():
            return

        response_seq = AtomicInteger(self.codec.decode(entry.get_context()))
        data = await self.fsm.apply(entry.get_data())

        if response_queue := self.response_queues.pop(response_seq, None):
            response_queue.put_nowait(RaftRespMessage(data))

        if (
            self.raftify_cfg.snapshot_interval > 0
            and time.time()
            > self.last_snapshot_created + self.raftify_cfg.snapshot_interval
        ):
            await self.create_snapshot(entry.get_index(), entry.get_term())

    async def handle_committed_config_change_entry(
        self, entry: Entry | EntryRef
    ) -> None:
        # TODO: Write documents to clarify when to use entry with empty data.
        if not entry.get_data():
            zero = ConfChangeV2.default()
            assert zero.leave_joint()
            if cs := self.raw_node.apply_conf_change_v2(zero):
                self.lmdb.set_conf_state(cs)
                await self.create_snapshot(entry.get_index(), entry.get_term())
            return

        # Block already applied entries handling
        if not entry.get_context():
            return

        response_seq = AtomicInteger(self.codec.decode(entry.get_context()))

        try:
            conf_change_v2 = ConfChangeV2.decode(entry.get_data())
        except Exception:
            conf_change_v2 = ConfChange.decode(entry.get_data()).as_v2()

        conf_changes = conf_change_v2.get_changes()
        addrs: list[SocketAddr] = self.codec.decode(conf_change_v2.get_context())

        for cc_idx, conf_change in enumerate(conf_changes):
            node_id = conf_change.get_node_id()
            change_type = conf_change.get_change_type()

            match change_type:
                case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                    addr = addrs[cc_idx]

                    self.logger.info(f"Node {node_id} ({addr}) joined the cluster.")
                    self.peers[node_id] = Peer(addr, PeerState.Connected)
                case ConfChangeType.RemoveNode:
                    if node_id == self.get_id():
                        self.should_exit = True
                        await self.raft_server.terminate()
                        self.logger.info(f"Node {node_id} quit the cluster.")
                    else:
                        self.logger.info(f"Node {node_id} removed from the cluster.")
                        self.peers[node_id].state = PeerState.Disconnected
                case _:
                    raise NotImplementedError

        if conf_state := self.raw_node.apply_conf_change_v2(conf_change_v2):
            self.lmdb.set_conf_state(conf_state)
            await self.create_snapshot(entry.get_index(), entry.get_term())

        if response_queue := self.response_queues.pop(response_seq, None):
            response: ResponseMessage

            match change_type:
                case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                    response = JoinSuccessRespMessage(
                        assigned_id=node_id,
                        raw_peers=self.codec.encode(self.peers.to_encodeable()),
                    )
                case ConfChangeType.RemoveNode:
                    response = PeerRemovalSuccessRespMessage()
                case _:
                    raise NotImplementedError

            try:
                response_queue.put_nowait(response)
            except Exception as e:
                self.logger.error(f"Error occurred while sending response. {e}")

    # TODO: Abstract and improve this event handling loop. especially, the part of handling response_queues.
    async def run(self) -> None:
        tick_timer = self.raftify_cfg.tick_interval
        before = time.time()

        while not self.should_exit:
            message = None

            try:
                message = await asyncio.wait_for(self.message_queue.get(), tick_timer)
            except asyncio.TimeoutError:
                pass
            except asyncio.CancelledError:
                self.logger.warning(
                    "Task cancelled error occurred in Raft node! preparing to terminate..."
                )
                break
            except Exception:
                raise

            if isinstance(message, RerouteToLeaderReqMessage):
                match message.type:
                    case RerouteMsgType.ConfChange:
                        assert message.conf_change is not None

                        message = ConfigChangeReqMessage(
                            conf_change=message.conf_change,
                            response_chan=message.response_chan,
                        )
                    case RerouteMsgType.Propose:
                        assert message.proposed_data is not None

                        message = ProposeReqMessage(
                            data=message.proposed_data,
                            response_chan=message.response_chan,
                        )

            if isinstance(message, ClusterBootstrapReadyReqMessage):
                self.logger.info(
                    "All nodes are ready to join the cluster. Start to bootstrap process..."
                )
                peers: Peers = self.codec.decode(message.peers)
                self.peers = peers
                for node_id, peer in self.peers.items():
                    peers.connect(node_id, peer.addr)

                message.response_chan.put_nowait(ClusterBootstrapReadyRespMessage())
                self.bootstrap_done = True

            elif isinstance(message, MemberBootstrapReadyReqMessage):
                assert self.is_leader(), "Only leader can handle this message!"
                follower_id = message.follower_id
                self.logger.info(f"Node {follower_id} request to join the cluster.")
                self.peers.connect(follower_id, self.peers[follower_id].addr)
                message.response_chan.put_nowait(MemberBootstrapReadyRespMessage())

            elif isinstance(message, ConfigChangeReqMessage):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.response_chan)
                else:
                    if self.raw_node.get_raft().has_pending_conf():
                        pending_conf_index = (
                            self.raw_node.get_raft().get_pending_conf_index()
                        )
                        self.logger.error(
                            f"Reject the conf change because pending conf change exist! ({pending_conf_index=}), try later..."
                        )
                        message.response_chan.put_nowait(ConfChangeRejectMessage())
                        return
                    else:
                        conf_change_v2 = ConfChangeV2Adapter.from_pb(
                            message.conf_change
                        )
                        socket_addrs: list[SocketAddr] = self.codec.decode(
                            conf_change_v2.get_context()
                        )

                        # TODO: Handle RemoveNode case here.
                        # Actually, RemoveNode case is handled in handle_committed_config_change_entry method.
                        # So, it is not necessary to handle it here.
                        # But, it would be better to handle it here to make the code more readable.
                        for addr in socket_addrs:
                            self.peers.ready_peer(addr)

                        await self.response_seq.increase()
                        self.response_queues[self.response_seq] = message.response_chan
                        context = self.codec.encode(self.response_seq.value)

                        try:
                            self.raw_node.propose_conf_change_v2(
                                context, conf_change_v2
                            )

                            self.logger.debug(
                                f"Proposed new config change..., seq={self.response_seq.value}, {conf_change_v2=}"
                            )
                        except rraft.ProposalDroppedError:
                            # TODO: Study when it happens and handle it.
                            raise

            elif isinstance(message, ApplyConfigChangeForcelyReqMessage):
                # Ignore all pending conf changes and apply the given conf change forcely.
                # This won't persist the conf changes, but apply them successfully.

                await self.response_seq.increase()
                self.response_queues[self.response_seq] = message.response_chan
                context = self.codec.encode(self.response_seq.value)

                conf_change_v2 = ConfChangeV2Adapter.from_pb(message.conf_change)

                try:
                    self.raw_node.propose_conf_change_v2(
                        context,
                        conf_change_v2,
                    )

                except rraft.ProposalDroppedError:
                    # TODO: Study when it happens and handle it.
                    raise

                self.raw_node.apply_conf_change_v2(conf_change_v2)
                message.response_chan.put_nowait(PeerRemovalSuccessRespMessage())

                changes = conf_change_v2.get_changes()
                for change in changes:
                    match change.get_change_type():
                        case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                            self.peers[change.get_node_id()].state = PeerState.Connected
                        case ConfChangeType.RemoveNode:
                            self.peers[
                                change.get_node_id()
                            ].state = PeerState.Disconnected

            elif isinstance(message, ProposeReqMessage):
                if not self.bootstrap_done:
                    self.logger.warning(
                        "Proposal rejected because the cluster bootstrap is not done yet."
                    )
                    message.response_chan.put_nowait(
                        RaftRespMessage(
                            data=b"Cluster bootstrap is not done yet.",
                            rejected=True,
                        )
                    )
                    continue

                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.response_chan)
                else:
                    await self.response_seq.increase()
                    self.response_queues[self.response_seq] = message.response_chan
                    context = self.codec.encode(self.response_seq.value)
                    self.raw_node.propose(context, message.data)

            elif isinstance(message, RequestIdReqMessage):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.response_chan)
                else:
                    reserved_id = self.peers.reserve_peer(
                        self.raw_node, SocketAddr.from_str(message.addr)
                    )

                    message.response_chan.put_nowait(
                        IdReservedRespMessage(
                            leader_id=self.get_leader_id(),
                            reserved_id=reserved_id,
                            raw_peers=self.codec.encode(self.peers.to_encodeable()),
                        )
                    )

            elif isinstance(message, RaftReqMessage):
                msg = MessageAdapter.from_pb(message.msg)
                self.logger.debug(
                    f"Node {msg.get_to()} received Raft message from the node {msg.get_from()}, Message: {msg}"
                )

                try:
                    self.raw_node.step(msg)
                except rraft.StepPeerNotFoundError:
                    self.logger.warning(
                        f"StepPeerNotFoundError occurred. Ignore this message if RemoveNode happened, Message: {msg}"
                    )
                    continue
                except rraft.StepLocalMsgError:
                    # TODO: Study what is LocalMsg and when it happens and handle it.
                    raise

            elif isinstance(message, GetPeersReqMessage):
                message.response_chan.put_nowait(
                    self.codec.encode(self.peers.to_encodeable())
                )

            elif isinstance(message, ReportUnreachableReqMessage):
                self.raw_node.report_unreachable(message.node_id)

            elif isinstance(message, DebugNodeReqMessage):
                message.response_chan.put_nowait(self.inspect())

            elif isinstance(message, DebugEntriesReqMessage):
                message.response_chan.put_nowait(self.get_all_entry_logs())

            elif isinstance(message, VersionReqMessage):
                message.response_chan.put_nowait(self.get_version())

            now = time.time()
            elapsed = now - before
            before = now

            if elapsed > tick_timer:
                tick_timer = self.raftify_cfg.tick_interval
                self.raw_node.tick()
            else:
                tick_timer -= elapsed

            await self.on_ready()

    async def on_ready(self) -> None:
        if not self.raw_node.has_ready():
            return

        ready = self.raw_node.ready()

        if soft_state := ready.ss():
            if self.on_leader_id_change:
                if soft_state.get_leader_id() != self.prev_leader_id:
                    self.on_leader_id_change(soft_state.get_leader_id())

            if self.on_state_role_change:
                if soft_state.get_raft_state() != self.prev_state_role:
                    self.on_state_role_change(soft_state.get_raft_state())

        if msgs := ready.take_messages():
            self.send_messages(msgs)

        snapshot_default = Snapshot.default()
        if ready.snapshot() != snapshot_default.make_ref():
            snapshot = ready.snapshot()
            self.logger.info("Restoring a state machine from the snapshot...")
            await self.fsm.restore(snapshot.get_data())
            self.lmdb.apply_snapshot(snapshot.clone())

        await self.handle_committed_entries(ready.take_committed_entries())

        if entries := ready.entries():
            self.lmdb.append(entries)

        if hard_state := ready.hs():
            self.lmdb.set_hard_state(hard_state)

        if persisted_msgs := ready.take_persisted_messages():
            self.send_messages(persisted_msgs)

        light_rd = self.raw_node.advance(ready.make_ref())

        if commit := light_rd.commit_index():
            self.lmdb.set_hard_state_comit(commit)

        self.send_messages(light_rd.take_messages())

        await self.handle_committed_entries(light_rd.take_committed_entries())

        self.raw_node.advance_apply()
