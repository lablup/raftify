import asyncio
import math
import pickle
import time
from asyncio import Queue

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

from raftify.config import RaftifyConfig
from raftify.fsm import FSM
from raftify.logger import AbstractRaftifyLogger
from raftify.pb_adapter import ConfChangeV2Adapter, MessageAdapter
from raftify.peers import Peer, Peers, PeerState
from raftify.protos.raft_service_pb2 import RerouteMsgType
from raftify.raft_client import RaftClient
from raftify.raft_server import RaftServer
from raftify.request_message import (
    ApplyConfigChangeForcelyReqMessage,
    ClusterBootstrapReadyReqMessage,
    ConfigChangeReqMessage,
    MemberBootstrapReadyReqMessage,
    ProposeReqMessage,
    RaftReqMessage,
    ReportUnreachableReqMessage,
    RequestIdReqMessage,
    RerouteToLeaderReqMessage,
)
from raftify.response_message import (
    IdReservedRespMessage,
    JoinSuccessRespMessage,
    RaftOkRespMessage,
    RaftRespMessage,
    RaftResponse,
    WrongLeaderRespMessage,
)
from raftify.storage.lmdb import LMDBStorage
from raftify.utils import AtomicInteger, SocketAddr


class RaftNode:
    def __init__(
        self,
        *,
        raw_node: RawNode,
        raft_server: RaftServer,
        peers: Peers,
        message_queue: Queue,
        fsm: FSM,
        lmdb: LMDBStorage,
        storage: Storage,
        seq: AtomicInteger,
        last_snap_time: float,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        bootstrap_done: bool,
    ):
        self.raw_node = raw_node
        self.raft_server = raft_server
        self.peers = peers
        self.message_queue = message_queue
        self.fsm = fsm
        self.lmdb = lmdb
        self.storage = storage
        self.seq = seq
        self.last_snap_time = last_snap_time
        self.logger = logger
        self.should_exit = False
        self.raftify_cfg = raftify_cfg
        self.bootstrap_done = bootstrap_done

    @classmethod
    def bootstrap_leader(
        cls,
        *,
        message_queue: Queue,
        fsm: FSM,
        raft_server: RaftServer,
        peers: Peers,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        bootstrap_done: bool,
    ) -> "RaftNode":
        cfg = raftify_cfg.raft_config

        cfg.set_id(1)
        cfg.validate()

        lmdb = LMDBStorage.create(
            map_size=raftify_cfg.lmdb_map_size,
            path=raftify_cfg.log_dir,
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

        seq = AtomicInteger(0)
        last_snap_time = time.time()

        raw_node.get_raft().become_candidate()
        raw_node.get_raft().become_leader()

        return cls(
            raw_node=raw_node,
            raft_server=raft_server,
            peers=peers,
            message_queue=message_queue,
            fsm=fsm,
            lmdb=lmdb,
            storage=storage,
            seq=seq,
            last_snap_time=last_snap_time,
            logger=logger,
            raftify_cfg=raftify_cfg,
            bootstrap_done=bootstrap_done,
        )

    @classmethod
    def new_follower(
        cls,
        *,
        message_queue: Queue,
        id: int,
        fsm: FSM,
        raft_server: RaftServer,
        peers: Peers,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
        raftify_cfg: RaftifyConfig,
        bootstrap_done: bool,
    ) -> "RaftNode":
        cfg = raftify_cfg.raft_config

        cfg.set_id(id)
        cfg.validate()

        lmdb = LMDBStorage.create(
            map_size=raftify_cfg.lmdb_map_size,
            path=raftify_cfg.log_dir,
            node_id=id,
            logger=logger,
        )

        storage = Storage(lmdb)
        raw_node = RawNode(cfg, storage, slog)

        seq = AtomicInteger(0)
        last_snap_time = time.time()

        return cls(
            raw_node=raw_node,
            raft_server=raft_server,
            peers=peers,
            message_queue=message_queue,
            fsm=fsm,
            lmdb=lmdb,
            storage=storage,
            seq=seq,
            last_snap_time=last_snap_time,
            logger=logger,
            raftify_cfg=raftify_cfg,
            bootstrap_done=bootstrap_done,
        )

    def get_id(self) -> int:
        return self.raw_node.get_raft().get_id()

    def get_leader_id(self) -> int:
        return self.raw_node.get_raft().get_leader_id()

    def is_leader(self) -> bool:
        return self.get_id() == self.get_leader_id()

    def remove_node(self, node_id: int) -> None:
        conf_change = ConfChange.default()
        conf_change.set_node_id(node_id)
        conf_change.set_context(pickle.dumps([self.peers[node_id].addr]))
        conf_change.set_change_type(ConfChangeType.RemoveNode)

        conf_change_v2 = conf_change.as_v2()
        conf_change_v2.set_transition(ConfChangeTransition.Auto)

        leader_id = self.raw_node.get_raft().get_id()
        asyncio.create_task(
            self.peers[leader_id].client.apply_change_config_forcely(conf_change_v2)
        )

    async def report_unreachable(self, node_id: int) -> None:
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

        elapsed = time.time() - failed_client.first_failed_time

        return round(elapsed, 4)

    def handle_node_auto_removal(self, elapsed: float, node_id: int) -> None:
        assert self.is_leader()

        if node_id not in self.peers.data.keys():
            return

        if elapsed >= self.raftify_cfg.node_auto_remove_threshold:
            if self.raftify_cfg.auto_remove_node:
                peer = self.peers[node_id]

                if peer.state == PeerState.Connected:
                    self.peers.data[node_id].state = PeerState.Disconnecting
                    self.remove_node(node_id)

                    self.logger.error(
                        f"Removing node {node_id} from the cluster "
                        f"automatically "
                        f"because the requests to the connection kept failed."
                    )

    def should_retry(self, message: Message, current_retry_count: int) -> bool:
        """ """
        node_id = message.get_to()

        if node_id not in self.peers.data.keys():
            return False

        return current_retry_count < self.raftify_cfg.max_retry_cnt

    async def send_message(self, client: RaftClient, message: Message) -> None:
        """
        Attempt to send a message 'max_retry_cnt' times at 'message_timeout' interval.
        If 'auto_remove_node' is set to True, the send function will automatically remove the node from the cluster.
        """

        node_id = message.get_to()
        current_retry_count = 0

        while True:
            try:
                if self.bootstrap_done:
                    if (
                        self.peers[node_id].state == PeerState.Disconnecting
                        or self.peers[node_id].state == PeerState.Disconnected
                    ):
                        return

                    await client.send_message(
                        message, timeout=self.raftify_cfg.message_timeout
                    )

                    failed_client = self.peers[node_id].client
                    assert failed_client is not None
                    failed_client.first_failed_time = None
                return
            except grpc.aio.AioRpcError or asyncio.TimeoutError as err:
                if not isinstance(err, asyncio.TimeoutError):
                    if not err.code() == grpc.StatusCode.UNAVAILABLE:
                        raise

                await self.report_unreachable(node_id)
                elapsed = self.get_elapsed_time_from_first_connection_lost(node_id)
                self.logger.warning(
                    f"Failed to connect to node {node_id}, elapsed from failure: {elapsed}s"
                )

                if self.is_leader():
                    self.handle_node_auto_removal(elapsed, node_id)

                if not isinstance(err, asyncio.TimeoutError):
                    await asyncio.sleep(self.raftify_cfg.message_timeout)

                if self.should_retry(message, current_retry_count):
                    current_retry_count += 1
                    continue
                else:
                    return
            # Unknown error reraising
            except Exception:
                raise

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
        # TODO: Make this follower to new cluster's leader
        assert (
            self.get_leader_id() in self.peers.data
        ), "Leader node not found in peers!"

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
        response_queues: dict[AtomicInteger, Queue],
    ) -> None:
        # Mostly, you need to save the last apply index to resume applying
        # after restart. Here we just ignore this because we use a Memory storage.

        # _last_apply_index = 0

        for entry in committed_entries:
            match entry.get_entry_type():
                case EntryType.EntryNormal:
                    await self.handle_committed_normal_entry(entry, response_queues)
                case EntryType.EntryConfChangeV2:
                    await self.handle_committed_config_change_entry(
                        entry, response_queues
                    )
                case _:
                    raise NotImplementedError

    async def create_snapshot(self, index: int, term: int) -> None:
        self.last_snap_time = time.time()
        snapshot_data = await self.fsm.snapshot()

        last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
        self.lmdb.compact(last_applied)
        self.lmdb.create_snapshot(snapshot_data, index, term)
        self.logger.info("Snapshot created successfully.")

    async def handle_committed_normal_entry(
        self, entry: Entry | EntryRef, response_queues: dict[AtomicInteger, Queue]
    ) -> None:
        if not entry.get_data():
            return

        seq = AtomicInteger(pickle.loads(entry.get_context()))
        data = await self.fsm.apply(entry.get_data())

        if response_queue := response_queues.pop(seq, None):
            response_queue.put_nowait(RaftRespMessage(data))

        if (
            self.raftify_cfg.snapshot_interval > 0
            and time.time() > self.last_snap_time + self.raftify_cfg.snapshot_interval
        ):
            await self.create_snapshot(entry.get_index(), entry.get_term())

    async def handle_committed_config_change_entry(
        self, entry: Entry | EntryRef, response_queues: dict[AtomicInteger, Queue]
    ) -> None:
        # TODO: Write documents to clarify when to use entry with empty data.
        if not entry.get_data():
            zero = ConfChangeV2.default()
            assert zero.leave_joint()
            if cs := self.raw_node.apply_conf_change_v2(zero):
                self.lmdb.set_conf_state(cs)
                await self.create_snapshot(entry.get_index(), entry.get_term())
            return

        seq = AtomicInteger(pickle.loads(entry.get_context()))
        conf_change_v2 = ConfChangeV2.decode(entry.get_data())

        print(f"{conf_change_v2=}")

        conf_changes = conf_change_v2.get_changes()
        addrs = pickle.loads(conf_change_v2.get_context())

        for cc_idx, conf_change in enumerate(conf_changes):
            node_id = conf_change.get_node_id()
            change_type = conf_change.get_change_type()

            match change_type:
                case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                    addr = addrs[cc_idx]

                    self.logger.info(f"Node {node_id} ({addr}) joined the cluster.")
                    self.peers[node_id] = Peer(
                        addr=addr, client=RaftClient(addr), state=PeerState.Connected
                    )
                case ConfChangeType.RemoveNode:
                    if node_id == self.get_id():
                        self.should_exit = True
                        await self.raft_server.terminate()
                        self.logger.info(f"Node {node_id} quit the cluster.")
                    else:
                        self.peers.data[node_id].state = PeerState.Disconnected
                case _:
                    raise NotImplementedError

        if conf_state := self.raw_node.apply_conf_change_v2(conf_change_v2):
            self.lmdb.set_conf_state(conf_state)
            await self.create_snapshot(entry.get_index(), entry.get_term())

        if response_queue := response_queues.pop(seq, None):
            response: RaftResponse

            match change_type:
                case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                    response = JoinSuccessRespMessage(
                        assigned_id=node_id, peers=self.peers.encode()
                    )
                case ConfChangeType.RemoveNode:
                    response = RaftOkRespMessage()
                case _:
                    raise NotImplementedError

            try:
                response_queue.put_nowait(response)
            except Exception as e:
                self.logger.error(f"Error occurred while sending response. {e}")

    # TODO: Abstract and improve this event handling loop. especially, the part of handling response_queues.
    async def run(self) -> None:
        tick_timer = self.raftify_cfg.tick_interval
        response_queues: dict[AtomicInteger, Queue] = {}
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
                            conf_change=message.conf_change, chan=message.chan
                        )
                    case RerouteMsgType.Propose:
                        assert message.proposed_data is not None

                        message = ProposeReqMessage(
                            data=message.proposed_data, chan=message.chan
                        )

            if isinstance(message, ClusterBootstrapReadyReqMessage):
                self.logger.info(
                    "All nodes are ready to join the cluster. Start to bootstrap process..."
                )
                peers = Peers.decode(message.peers)
                self.peers = peers
                for node_id, peer in self.peers.data.items():
                    peers.connect(node_id, peer.addr)

                message.chan.put_nowait(RaftOkRespMessage())
                self.bootstrap_done = True

            elif isinstance(message, MemberBootstrapReadyReqMessage):
                assert self.is_leader(), "Only leader can handle this message!"
                follower_id = message.follower_id
                self.logger.info(f"Node {follower_id} request to join the cluster.")
                self.peers.connect(follower_id, self.peers[follower_id].addr)
                message.chan.put_nowait(RaftOkRespMessage())

            elif isinstance(message, ConfigChangeReqMessage):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.chan)
                else:
                    if self.raw_node.get_raft().has_pending_conf():
                        pending_conf_index = (
                            self.raw_node.get_raft().get_pending_conf_index()
                        )
                        self.logger.error(
                            f"Reject the conf change because pending conf change exist! ({pending_conf_index=}), try later..."
                        )
                    else:
                        conf_change_v2 = ConfChangeV2Adapter.from_pb(
                            message.conf_change
                        )
                        socket_addrs = pickle.loads(conf_change_v2.get_context())

                        for addr in socket_addrs:
                            self.peers.ready_peer(addr)

                        await self.seq.increase()
                        response_queues[self.seq] = message.chan
                        context = pickle.dumps(self.seq.value)

                        try:
                            self.raw_node.propose_conf_change_v2(
                                context, conf_change_v2
                            )

                            self.logger.debug(
                                f"Proposed new config change..., seq={self.seq.value}, {conf_change_v2=}"
                            )
                        except rraft.ProposalDroppedError:
                            # TODO: Study when it happens and handle it.
                            raise

            elif isinstance(message, ApplyConfigChangeForcelyReqMessage):
                # Ignore all pending conf changes and apply the given conf change forcely.
                # This won't persist the conf changes, but apply them successfully.

                await self.seq.increase()
                response_queues[self.seq] = message.chan
                context = pickle.dumps(self.seq.value)

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
                message.chan.put_nowait(RaftOkRespMessage())

                changes = conf_change_v2.get_changes()
                for change in changes:
                    self.peers.data[change.get_node_id()].state = PeerState.Disconnected

            elif isinstance(message, ProposeReqMessage):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.chan)
                else:
                    await self.seq.increase()
                    response_queues[self.seq] = message.chan
                    context = pickle.dumps(self.seq.value)
                    self.raw_node.propose(context, message.data)

            elif isinstance(message, RequestIdReqMessage):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.chan)
                else:
                    reserved_id = self.peers.reserve_peer(
                        self.raw_node, SocketAddr.from_str(message.addr)
                    )

                    message.chan.put_nowait(
                        IdReservedRespMessage(
                            leader_id=self.get_leader_id(),
                            reserved_id=reserved_id,
                            peers=self.peers.encode(),
                        )
                    )

            elif isinstance(message, RaftReqMessage):
                msg = MessageAdapter.from_pb(message.msg)
                self.logger.debug(
                    f"Node {msg.get_to()} received Raft message from the node {msg.get_from()}"
                )

                try:
                    self.raw_node.step(msg)
                except rraft.StepPeerNotFoundError:
                    self.logger.warning(
                        "StepPeerNotFoundError occurred. Ignore this message if RemoveNode happend."
                    )
                    continue
                except rraft.StepLocalMsgError:
                    # TODO: Study what is LocalMsg and when it happens and handle it.
                    raise

            elif isinstance(message, ReportUnreachableReqMessage):
                self.raw_node.report_unreachable(message.node_id)

            now = time.time()
            elapsed = now - before
            before = now

            if elapsed > tick_timer:
                tick_timer = self.raftify_cfg.tick_interval
                self.raw_node.tick()
            else:
                tick_timer -= elapsed

            await self.on_ready(response_queues)

    def handle_persisted_entries(self, entries: list[Entry] | list[EntryRef]):
        # TODO: Write documents to clarify when to use entry with empty data.
        for entry in entries:
            if (
                entry.get_entry_type() == EntryType.EntryConfChangeV2
                and not entry.get_data()
            ):
                asyncio.create_task(self.commit_zero(entry))

        self.lmdb.append(entries)

    async def on_ready(self, response_queues: dict[AtomicInteger, Queue]) -> None:
        if not self.raw_node.has_ready():
            return

        ready = self.raw_node.ready()

        if msgs := ready.take_messages():
            self.send_messages(msgs)

        snapshot_default = Snapshot.default()
        if ready.snapshot() != snapshot_default.make_ref():
            snapshot = ready.snapshot()
            self.logger.info("Restoring FSM from the snapshot...")
            await self.fsm.restore(snapshot.get_data())
            self.lmdb.apply_snapshot(snapshot.clone())

        await self.handle_committed_entries(
            ready.take_committed_entries(), response_queues
        )

        if entries := ready.entries():
            self.lmdb.append(entries)

        if hs := ready.hs():
            self.lmdb.set_hard_state(hs)

        if persisted_msgs := ready.take_persisted_messages():
            self.send_messages(persisted_msgs)

        light_rd = self.raw_node.advance(ready.make_ref())

        if commit := light_rd.commit_index():
            self.lmdb.set_hard_state_comit(commit)

        self.send_messages(light_rd.take_messages())

        await self.handle_committed_entries(
            light_rd.take_committed_entries(), response_queues
        )

        self.raw_node.advance_apply()
