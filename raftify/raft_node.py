import asyncio
import pickle
import time
from asyncio import Queue
from typing import Optional

from rraft import (
    ConfChange,
    ConfChangeType,
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

from raftify.config import RaftConfig
from raftify.fsm import FSM
from raftify.lmdb import LMDBStorage
from raftify.logger import AbstractRaftifyLogger
from raftify.message_sender import MessageSender
from raftify.pb_adapter import ConfChangeAdapter, MessageAdapter
from raftify.protos.raft_service_pb2 import RerouteMsgType
from raftify.raft_client import RaftClient
from raftify.request_message import (
    MessageConfigChange,
    MessagePropose,
    MessageRaft,
    MessageReportUnreachable,
    MessageRequestId,
    MessageRerouteToLeader,
)
from raftify.response_message import (
    RaftRespIdReserved,
    RaftRespJoinSuccess,
    RaftRespOk,
    RaftRespResponse,
    RaftRespWrongLeader,
)
from raftify.utils import AtomicInteger


class RaftNode:
    def __init__(
        self,
        raw_node: RawNode,
        # the peer client could be optional, because an id can be reserved and later populated
        peers: dict[int, Optional[RaftClient]],
        chan: Queue,
        fsm: FSM,
        lmdb: LMDBStorage,
        storage: Storage,
        seq: AtomicInteger,
        last_snap_time: float,
        logger: AbstractRaftifyLogger,
        cluster_cfg: RaftConfig,
    ):
        self.raw_node = raw_node
        self.peers = peers
        self.chan = chan
        self.fsm = fsm
        self.lmdb = lmdb
        self.storage = storage
        self.seq = seq
        self.last_snap_time = last_snap_time
        self.logger = logger
        self.should_exit = False
        self.cluster_cfg = cluster_cfg

    @classmethod
    def bootstrap_leader(
        cls,
        chan: Queue,
        fsm: FSM,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
        cluster_cfg: RaftConfig,
    ) -> "RaftNode":
        cfg = cluster_cfg.config

        cfg.set_id(1)
        cfg.validate()

        snapshot = Snapshot.default()
        snapshot.get_metadata().set_index(0)
        snapshot.get_metadata().set_term(0)
        snapshot.get_metadata().get_conf_state().set_voters([1])

        lmdb = LMDBStorage.create(cluster_cfg.log_dir, 1, logger)
        lmdb.apply_snapshot(snapshot)

        storage = Storage(lmdb)
        raw_node = RawNode(cfg, storage, slog)

        peers = {}
        seq = AtomicInteger(0)
        last_snap_time = time.time()

        raw_node.get_raft().become_candidate()
        raw_node.get_raft().become_leader()

        return cls(
            raw_node,
            peers,
            chan,
            fsm,
            lmdb,
            storage,
            seq,
            last_snap_time,
            logger,
            cluster_cfg,
        )

    @classmethod
    def new_follower(
        cls,
        chan: Queue,
        id: int,
        fsm: FSM,
        slog: Logger | LoggerRef,
        logger: AbstractRaftifyLogger,
        cluster_cfg: RaftConfig,
    ) -> "RaftNode":
        cfg = cluster_cfg.config

        cfg.set_id(id)
        cfg.validate()

        lmdb = LMDBStorage.create(cluster_cfg.log_dir, id, logger)
        storage = Storage(lmdb)
        raw_node = RawNode(cfg, storage, slog)

        peers = {}
        seq = AtomicInteger(0)
        last_snap_time = time.time()

        return cls(
            raw_node,
            peers,
            chan,
            fsm,
            lmdb,
            storage,
            seq,
            last_snap_time,
            logger,
            cluster_cfg,
        )

    def get_id(self) -> int:
        return self.raw_node.get_raft().get_id()

    def get_leader_id(self) -> int:
        return self.raw_node.get_raft().get_leader_id()

    def is_leader(self) -> bool:
        return self.get_id() == self.get_leader_id()

    def peer_addrs(self) -> dict[int, str]:
        return {k: str(v.addr) for k, v in self.peers.items() if v is not None}

    def reserve_next_peer_id(self, addr: str) -> int:
        """
        Reserve a slot to insert node on next node addition commit.
        """
        prev_conns = [
            id for id, peer in self.peers.items() if peer and addr == str(peer.addr)
        ]

        if len(prev_conns) > 0:
            next_id = prev_conns[0]
        else:
            next_id = max(self.peers.keys()) if any(self.peers) else 1
            next_id = max(next_id + 1, self.get_id())

            # if assigned id is ourself, return next one
            if next_id == self.get_id():
                next_id += 1

        self.logger.info(f"Reserved peer id {next_id}.")
        self.peers[next_id] = None
        return next_id

    def send_messages(self, msgs: list[Message]):
        for msg in msgs:
            if client := self.peers.get(msg.get_to()):
                asyncio.create_task(
                    MessageSender(
                        client_id=msg.get_to(),
                        client=client,
                        chan=self.chan,
                        message=msg,
                        timeout=0.1,
                        max_retries=5,
                        logger=self.logger,
                    ).send()
                )

    async def send_wrongleader_response(self, channel: Queue) -> None:
        # TODO: Make this follower to new cluster's leader
        assert self.get_leader_id() in self.peers, "Leader can't be an empty node!"

        try:
            # TODO: handle error here
            await channel.put(
                RaftRespWrongLeader(
                    leader_id=self.get_leader_id(),
                    leader_addr=str(self.peers[self.get_leader_id()].addr),
                )
            )
        except Exception:
            pass

    async def handle_committed_entries(
        self,
        committed_entries: list[Entry] | list[EntryRef],
        client_senders: dict[int, Queue],
    ) -> None:
        # Mostly, you need to save the last apply index to resume applying
        # after restart. Here we just ignore this because we use a Memory storage.

        # _last_apply_index = 0

        for entry in committed_entries:
            # Empty entry, when the peer becomes Leader it will send an empty entry.
            if not entry.get_data():
                continue

            match entry.get_entry_type():
                case EntryType.EntryNormal:
                    await self.handle_normal_entry(entry, client_senders)
                case EntryType.EntryConfChange:
                    await self.handle_config_change_entry(entry, client_senders)
                case _:
                    raise NotImplementedError

    async def handle_normal_entry(
        self, entry: Entry | EntryRef, senders: dict[int, Queue]
    ) -> None:
        seq = pickle.loads(entry.get_context())
        data = await self.fsm.apply(entry.get_data())

        if sender := senders.pop(seq, None):
            sender.put_nowait(RaftRespResponse(data))

        if time.time() > self.last_snap_time + 15:
            self.logger.info("Creating snapshot...")
            self.last_snap_time = time.time()
            snapshot = await self.fsm.snapshot()

            if self.cluster_cfg.use_log_compaction:
                last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
                self.lmdb.compact(last_applied)

            try:
                self.lmdb.create_snapshot(snapshot, entry.get_index(), entry.get_term())
                self.logger.info("Snapshot created successfully.")
            except Exception:
                pass

    async def handle_config_change_entry(
        self, entry: Entry | EntryRef, senders: dict[int, Queue]
    ) -> None:
        seq = pickle.loads(entry.get_context())
        change = ConfChange.decode(entry.get_data())
        node_id = change.get_node_id()

        change_type = change.get_change_type()

        match change_type:
            case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                addr = pickle.loads(change.get_context())
                self.logger.info(
                    f"Node '{addr} (node id: {node_id})' added to the cluster."
                )
                self.peers[node_id] = RaftClient(addr)
            case ConfChangeType.RemoveNode:
                if change.get_node_id() == self.get_id():
                    self.logger.info(f"{self.get_id()} quit the cluster.")
                    self.should_exit = True
                else:
                    self.peers.pop(change.get_node_id(), None)
            case _:
                raise NotImplementedError

        if cs := self.raw_node.apply_conf_change(change):
            snapshot = await self.fsm.snapshot()
            self.lmdb.set_conf_state(cs)

            if self.cluster_cfg.use_log_compaction:
                last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
                self.lmdb.compact(last_applied)

            try:
                self.lmdb.create_snapshot(snapshot, entry.get_index(), entry.get_term())
            except Exception:
                pass

        if sender := senders.pop(seq, None):
            match change_type:
                case ConfChangeType.AddNode | ConfChangeType.AddLearnerNode:
                    response = RaftRespJoinSuccess(
                        assigned_id=node_id, peer_addrs=self.peer_addrs()
                    )
                case ConfChangeType.RemoveNode:
                    response = RaftRespOk()
                case _:
                    raise NotImplementedError

            try:
                sender.put_nowait(response)
            except Exception:
                self.logger.error("Error sending response")

    async def run(self) -> None:
        heartbeat = 0.1

        # A map to contain sender to client responses
        client_senders: dict[int, Queue] = {}
        timer = time.time()

        while not self.should_exit:
            message = None

            try:
                message = await asyncio.wait_for(self.chan.get(), heartbeat)
            except asyncio.TimeoutError:
                pass
            except asyncio.CancelledError:
                self.logger.warning("Cancelled error occurred!")
                raise
            except Exception:
                raise

            if isinstance(message, MessageRerouteToLeader):
                match message.type:
                    case RerouteMsgType.ConfChange:
                        message = MessageConfigChange(
                            change=message.confchange, chan=message.chan
                        )
                    case RerouteMsgType.Propose:
                        message = MessagePropose(
                            data=message.proposed_data, chan=message.chan
                        )

            if isinstance(message, MessageConfigChange):
                change = ConfChangeAdapter.from_pb(message.change)

                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.chan)
                else:
                    # leader assign new id to peer
                    self.logger.debug(
                        f'Received request from the "node {change.get_node_id()}"'
                    )
                    self.seq.increase()
                    client_senders[self.seq.value] = message.chan
                    context = pickle.dumps(self.seq.value)
                    self.raw_node.propose_conf_change(context, change)

            elif isinstance(message, MessagePropose):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.chan)
                else:
                    self.seq.increase()
                    client_senders[self.seq.value] = message.chan
                    context = pickle.dumps(self.seq.value)
                    self.raw_node.propose(context, message.data)

            elif isinstance(message, MessageRequestId):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    await self.send_wrongleader_response(message.chan)
                else:
                    await message.chan.put(
                        RaftRespIdReserved(
                            leader_id=self.get_leader_id(),
                            reserved_id=self.reserve_next_peer_id(message.addr),
                            peer_addrs=self.peer_addrs(),
                        )
                    )

            elif isinstance(message, MessageRaft):
                msg = MessageAdapter.from_pb(message.msg)
                self.logger.debug(
                    f'Node {msg.get_to()} Received Raft message from the "node {msg.get_from()}"'
                )

                try:
                    self.raw_node.step(msg)
                except Exception:
                    pass

            elif isinstance(message, MessageReportUnreachable):
                self.raw_node.report_unreachable(message.node_id)

            now = time.time()
            elapsed = now - timer
            timer = now

            if elapsed > heartbeat:
                heartbeat = 0.1
                self.raw_node.tick()
            else:
                heartbeat -= elapsed

            await self.on_ready(client_senders)

    async def on_ready(self, client_senders: dict[int, Queue]) -> None:
        if not self.raw_node.has_ready():
            return

        ready = self.raw_node.ready()

        if msgs := ready.take_messages():
            self.send_messages(msgs)

        snapshot_default = Snapshot.default()
        if ready.snapshot() != snapshot_default.make_ref():
            snapshot = ready.snapshot()
            await self.fsm.restore(snapshot.get_data())
            self.lmdb.apply_snapshot(snapshot.clone())

        await self.handle_committed_entries(
            ready.take_committed_entries(), client_senders
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
            light_rd.take_committed_entries(), client_senders
        )

        self.raw_node.advance_apply()
