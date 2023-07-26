import asyncio
import logging
import pickle
import sys
import time
from asyncio import Queue
from typing import Dict, List, Optional

from rraft import (
    ConfChange,
    ConfChangeType,
    Config,
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

from riteraft.fsm import FSM
from riteraft.lmdb import LMDBStorage
from riteraft.message import (
    MessageConfigChange,
    MessagePropose,
    MessageRaft,
    MessageReportUnreachable,
    MessageRequestId,
    RaftRespIdReserved,
    RaftRespJoinSuccess,
    RaftRespOk,
    RaftRespResponse,
    RaftRespWrongLeader,
)
from riteraft.message_sender import MessageSender
from riteraft.pb_adapter import ConfChangeAdapter, MessageAdapter
from riteraft.raft_client import RaftClient
from riteraft.utils import AtomicInteger


class RaftNode:
    def __init__(
        self,
        raw_node: RawNode,
        # the peer client could be optional, because an id can be reserved and later populated
        peers: Dict[int, Optional[RaftClient]],
        chan: Queue,
        fsm: FSM,
        lmdb: LMDBStorage,
        storage: Storage,
        seq: AtomicInteger,
        last_snap_time: float,
    ):
        self.raw_node = raw_node
        self.peers = peers
        self.chan = chan
        self.fsm = fsm
        self.lmdb = lmdb
        self.storage = storage
        self.seq = seq
        self.last_snap_time = last_snap_time

    @staticmethod
    def bootstrap_leader(
        chan: Queue, fsm: FSM, logger: Logger | LoggerRef
    ) -> "RaftNode":
        config = Config.default()
        config.set_id(1)
        config.set_election_tick(10)
        config.set_heartbeat_tick(3)
        config.validate()

        snapshot = Snapshot.default()
        snapshot.get_metadata().set_index(0)
        snapshot.get_metadata().set_term(0)
        snapshot.get_metadata().get_conf_state().set_voters([1])

        lmdb = LMDBStorage.create(".", 1)
        lmdb.apply_snapshot(snapshot)

        storage = Storage(lmdb)
        raw_node = RawNode(config, storage, logger)

        peers = {}
        seq = AtomicInteger(0)
        last_snap_time = time.time()

        raw_node.get_raft().become_candidate()
        raw_node.get_raft().become_leader()

        return RaftNode(
            raw_node,
            peers,
            chan,
            fsm,
            lmdb,
            storage,
            seq,
            last_snap_time,
        )

    @staticmethod
    def new_follower(
        chan: Queue,
        id: int,
        fsm: FSM,
        logger: Logger | LoggerRef,
    ) -> "RaftNode":
        config = Config.default()

        config.set_id(id)
        config.set_election_tick(10)
        config.set_heartbeat_tick(3)
        config.validate()

        # TODO: Create mdb files in temp dir path. Now it create them in current dir for easy test and debugging.
        lmdb = LMDBStorage.create(".", id)
        storage = Storage(lmdb)
        raw_node = RawNode(config, storage, logger)

        peers = {}
        seq = AtomicInteger(0)
        last_snap_time = time.time()

        return RaftNode(
            raw_node,
            peers,
            chan,
            fsm,
            lmdb,
            storage,
            seq,
            last_snap_time,
        )

    def id(self) -> int:
        return self.raw_node.get_raft().get_id()

    def leader(self) -> int:
        return self.raw_node.get_raft().get_leader_id()

    def is_leader(self) -> bool:
        return self.id() == self.leader()

    def peer_addrs(self) -> Dict[int, str]:
        return {k: str(v.addr) for k, v in self.peers.items() if v is not None}

    def reserve_next_peer_id(self, addr: str) -> int:
        """
        Reserve a slot to insert node on next node addition commit.
        """
        prev_conns = [id for id, peer in self.peers.items() if addr == peer.addr]

        if len(prev_conns) > 0:
            next_id = prev_conns[0]
        else:
            next_id = max(self.peers.keys()) if any(self.peers) else 1
            next_id = max(next_id + 1, self.id())

            # if assigned id is ourself, return next one
            if next_id == self.id():
                next_id += 1

        logging.info(f"Reserved peer id {next_id}.")
        self.peers[next_id] = None
        return next_id

    def send_messages(self, msgs: List[Message]):
        for msg in msgs:
            logging.debug(
                f"light ready message from {msg.get_from()} to {msg.get_to()}."
            )

            if client := self.peers.get(msg.get_to()):
                asyncio.create_task(
                    MessageSender(
                        client_id=msg.get_to(),
                        client=client,
                        chan=self.chan,
                        message=msg,
                        timeout=0.1,
                        max_retries=5,
                    ).send()
                )

    async def send_wrong_leader(self, channel: Queue) -> None:
        print("self.peers", self.peers)
        assert self.leader() in self.peers, "Leader can't be an empty node!"

        try:
            # TODO handle error here
            await channel.put(
                RaftRespWrongLeader(
                    leader_id=self.leader(),
                    leader_addr=str(self.peers[self.leader()].addr),
                )
            )
        except Exception:
            pass

    async def handle_committed_entries(
        self,
        committed_entries: List[Entry] | List[EntryRef],
        client_senders: Dict[int, Queue],
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
        self, entry: Entry | EntryRef, senders: Dict[int, Queue]
    ) -> None:
        seq = pickle.loads(entry.get_context())
        data = await self.fsm.apply(entry.get_data())

        if sender := senders.pop(seq, None):
            sender.put_nowait(RaftRespResponse(data))

        if time.time() > self.last_snap_time + 15:
            logging.info("Creating snapshot...")
            self.last_snap_time = time.time()
            last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
            snapshot = await self.fsm.snapshot()
            self.lmdb.compact(last_applied)

            try:
                self.lmdb.create_snapshot(snapshot, entry.get_index(), entry.get_term())
            except Exception:
                pass

    async def handle_config_change_entry(
        self, entry: Entry | EntryRef, senders: Dict[int, Queue]
    ) -> None:
        seq = pickle.loads(entry.get_context())
        change = ConfChange.decode(entry.get_data())
        node_id = change.get_node_id()

        change_type = change.get_change_type()

        match change_type:
            case ConfChangeType.AddNode:
                addr = pickle.loads(change.get_context())
                logging.info(
                    f"Node '{addr} (node id: {node_id})' added to the cluster."
                )
                self.peers[node_id] = RaftClient(addr)
            case ConfChangeType.RemoveNode:
                if change.get_node_id() == self.id():
                    logging.warning("Quitting the cluster.")
                    sys.exit(0)
                else:
                    self.peers.pop(change.get_node_id(), None)
            case _:
                raise NotImplementedError

        if cs := self.raw_node.apply_conf_change(change):
            last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
            snapshot = await self.fsm.snapshot()

            self.lmdb.set_conf_state(cs)
            self.lmdb.compact(last_applied)

            try:
                self.lmdb.create_snapshot(snapshot, entry.get_index(), entry.get_term())
            except Exception:
                pass

        if sender := senders.pop(seq, None):
            match change_type:
                case ConfChangeType.AddNode:
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
                logging.error("Error sending response")

    async def run(self) -> None:
        heartbeat = 0.1

        # A map to contain sender to client responses
        client_senders: Dict[int, Queue] = {}
        timer = time.time()

        while True:
            message = None

            try:
                message = await asyncio.wait_for(self.chan.get(), heartbeat)
            except asyncio.TimeoutError:
                pass
            except asyncio.CancelledError:
                logging.warning("Cancelled error occurred!")
                raise
            except Exception:
                raise

            if isinstance(message, MessageConfigChange):
                change = ConfChangeAdapter.from_pb(message.change)

                # whenever a change id is 0, it's a message to self.
                if change.get_node_id() == 0:
                    change.set_node_id(self.id())

                if not self.is_leader():
                    # wrong leader send client cluster data
                    # TODO: retry strategy in case of failure
                    await self.send_wrong_leader(message.chan)
                else:
                    # leader assign new id to peer
                    logging.debug(f"Received request from: {change.get_node_id()}")
                    self.seq.increase()
                    client_senders[self.seq.value] = message.chan
                    context = pickle.dumps(self.seq.value)
                    self.raw_node.propose_conf_change(context, change)

            elif isinstance(message, MessagePropose):
                if not self.is_leader():
                    await self.send_wrong_leader(message.chan)
                else:
                    self.seq.increase()
                    client_senders[self.seq.value] = message.chan
                    context = pickle.dumps(self.seq.value)
                    self.raw_node.propose(context, message.proposal)

            elif isinstance(message, MessageRequestId):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    logging.info("Requested Id, but not leader")
                    await self.send_wrong_leader(message.chan)
                else:
                    await message.chan.put(
                        RaftRespIdReserved(
                            leader_id=self.leader(),
                            reserved_id=self.reserve_next_peer_id(message.addr),
                            peer_addrs=self.peer_addrs(),
                        )
                    )

            elif isinstance(message, MessageRaft):
                msg = MessageAdapter.from_pb(message.msg)
                logging.debug(f"Received Raft message from {msg.get_from()}")

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

    async def on_ready(self, client_senders: Dict[int, Queue]) -> None:
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
