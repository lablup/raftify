import asyncio
import logging
import time
from asyncio import Queue
from typing import Dict, Optional

from rraft import (
    ConfChange,
    ConfChangeType,
    Config,
    Entry_Ref,
    EntryType,
    Logger_Ref,
    RawNode,
    Snapshot,
    Storage,
)

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
    RaftRespWrongLeader,
)
from riteraft.message_sender import MessageSender
from riteraft.raft_client import RaftClient
from riteraft.store import AbstractStore
from riteraft.utils import AtomicInteger, decode_u64, encode_u64


class RaftNode:
    def __init__(
        self,
        raw_node: RawNode,
        # the peer client could be optional, because an id can be reserved and later populated
        peers: Dict[int, Optional[RaftClient]],
        chan: Queue,
        store: AbstractStore,
        lmdb: LMDBStorage,
        should_quit: bool,
        seq: AtomicInteger,
        last_snap_time: float,
    ):
        self.raw_node = raw_node
        self.peers = peers
        self.chan = chan
        self.store = store
        self.lmdb = lmdb
        self.should_quit = should_quit
        self.seq = seq
        self.last_snap_time = last_snap_time

    @staticmethod
    def new_leader(chan: Queue, store: AbstractStore, logger: Logger_Ref) -> "RaftNode":
        config = Config.default()
        config.set_id(1)
        config.set_election_tick(10)
        # Heartbeat tick is for how long the leader needs to send
        # a heartbeat to keep alive.
        config.set_heartbeat_tick(3)
        config.validate()

        snapshot = Snapshot.default()
        # Because we don't use the same configuration to initialize every node, so we use
        # a non-zero index to force new followers catch up logs by snapshot first, which will
        # bring all nodes to the same initial state.
        snapshot.get_metadata().set_index(1)
        snapshot.get_metadata().set_term(1)
        snapshot.get_metadata().get_conf_state().set_voters([1])

        lmdb = LMDBStorage.create(".", 1)
        lmdb.apply_snapshot(snapshot)

        lmdb_wrapper = Storage(lmdb)
        raw_node = RawNode(config, lmdb_wrapper, logger)
        peers = {}
        seq = AtomicInteger(0)
        last_snap_time = time.time()

        raw_node.get_raft().become_candidate()
        raw_node.get_raft().become_leader()

        return RaftNode(
            raw_node,
            peers,
            chan,
            store,
            lmdb,
            False,
            seq,
            last_snap_time,
        )

    @staticmethod
    def new_follower(
        chan: Queue,
        id: int,
        store: AbstractStore,
        logger: Logger_Ref,
    ) -> "RaftNode":
        config = Config.default()
        config.set_id(id)
        config.set_election_tick(10)
        # Heartbeat tick is for how long the leader needs to send
        # a heartbeat to keep alive.
        config.set_heartbeat_tick(3)
        config.validate()

        lmdb = LMDBStorage.create(".", id)
        lmdb_wrapper = Storage(lmdb)
        raw_node = RawNode(config, lmdb_wrapper, logger)
        peers = {}
        seq = AtomicInteger(0)
        last_snap_time = 1000.0

        return RaftNode(
            raw_node,
            peers,
            chan,
            store,
            lmdb,
            False,
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
        return {k: str(v.addr) for k, v in self.peers.items()}

    def reserve_next_peer_id(self) -> int:
        """
        Reserve a slot to insert node on next node addition commit
        """
        next_id = max(self.peers.keys()) if any(self.peers) else 1
        # if assigned id is ourself, return next one
        next_id = max(next_id + 1, self.id())
        self.peers[next_id] = None

        logging.info(f"Reserved peer id {next_id}")
        return next_id

    async def send_wrong_leader(self, channel: Queue) -> None:
        leader_id = self.leader()
        # leader can't be an empty node
        leader_addr = self.peers[leader_id].addr
        raft_response = RaftRespWrongLeader(
            leader_id=leader_id,
            leader_addr=str(leader_addr),
        )
        # TODO handle error here
        await channel.put(raft_response)

    async def run(self) -> None:
        heartbeat = 0.1

        # A map to contain sender to client responses
        client_senders: Dict[int, Queue] = {}

        while True:
            if self.should_quit:
                logging.warning("Quitting raft")
                return

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
                # whenever a change id is 0, it's a message to self.
                if message.change.get_node_id() == 0:
                    message.change.set_node_id(self.id())

                if not self.is_leader():
                    # wrong leader send client cluster data
                    # TODO: retry strategy in case of failure
                    await self.send_wrong_leader(channel=message.chan)
                else:
                    # leader assign new id to peer
                    logging.debug(
                        f"Received request from: {message.change.get_node_id()}"
                    )
                    self.seq.increase()
                    client_senders[self.seq] = message.chan
                    context = encode_u64(self.seq.value)
                    self.raw_node.propose_conf_change(context, message.change)

            elif isinstance(message, MessageRaft):
                logging.debug(
                    f"Raft message: to={self.get_raft().get_id()} from={message.msg.get_from()}"
                )
                self.raw_node.step(message.msg)

            elif isinstance(message, MessagePropose):
                if not self.is_leader():
                    # wrong leader send client cluster data
                    leader_id = self.leader()
                    # leader can't be an empty node
                    leader_addr = self.peers[leader_id].addr

                    await message.chan.put(
                        RaftRespWrongLeader(
                            leader_id=leader_id,
                            leader_addr=str(leader_addr),
                        )
                    )
                else:
                    self.seq.increase()
                    client_senders[self.seq] = message.chan
                    context = encode_u64(self.seq.value)
                    self.raw_node.propose(context, message.proposal)

            elif isinstance(message, MessageRequestId):
                if not self.is_leader():
                    # TODO: retry strategy in case of failure
                    logging.info("Requested Id, but not leader")
                    await self.send_wrong_leader(message.chan)
                else:
                    await message.chan.put(
                        RaftRespIdReserved(id=self.reserve_next_peer_id())
                    )

            elif isinstance(message, MessageReportUnreachable):
                self.raw_node.report_unreachable(message.node_id)

            self.raw_node.tick()
            await self.on_ready(client_senders)

    async def on_ready(self, client_senders: Dict[int, Queue]) -> None:
        if not self.raw_node.has_ready():
            return

        ready = self.raw_node.ready()

        if entries := ready.entries():
            self.lmdb.append(entries)

        if hs := ready.hs():
            # Raft HardState changed, and we need to persist it.
            self.lmdb.set_hard_state(hs)

        for message in ready.take_messages():
            logging.debug(f"Message from {message.get_from()} to {message.get_to()}")

            if client := self.peers.get(message.get_to()):
                asyncio.create_task(
                    MessageSender(
                        client_id=message.get_to(),
                        client=client,
                        chan=self.chan,
                        message=message,
                        timeout=0.1,
                        max_retries=5,
                    ).send()
                )

        if snapshot := ready.snapshot():
            await self.store.restore(snapshot.get_data())
            self.lmdb.apply_snapshot(snapshot)

        if hs := ready.hs():
            # Raft HardState changed, and we need to persist it.
            self.lmdb.set_hard_state(hs)

        if committed_entries := ready.committed_entries():
            # Mostly, you need to save the last apply index to resume applying
            # after restart. Here we just ignore this because we use a Memory storage.

            # _last_apply_index = 0

            for entry in committed_entries:
                if not any(entry.get_data()):
                    # Empty entry, when the peer becomes Leader it will send an empty entry.
                    continue

                if entry.get_entry_type() == EntryType.EntryNormal:
                    await self.handle_normal(entry, client_senders)
                elif entry.get_entry_type() == EntryType.EntryConfChange:
                    await self.handle_config_change(entry, client_senders)
                elif entry.get_entry_type() == EntryType.EntryConfChangeV2:
                    raise NotImplementedError

        self.raw_node.advance(ready.make_ref())

    async def handle_config_change(
        self, entry: Entry_Ref, senders: Dict[int, Queue]
    ) -> None:
        seq = decode_u64(entry.get_context())
        change = ConfChange.decode(entry.get_data())
        id = change.get_node_id()

        change_type = change.get_change_type()

        if change_type == ConfChangeType.AddNode:
            addr = decode_u64(change.get_context())
            logging.info(f"Adding {addr} ({id}) to peers")
            self.peers[id] = RaftClient(addr)
        elif change_type == ConfChangeType.RemoveNode:
            if change.get_node_id() == self.id():
                self.should_quit = True
                logging.warning("Quitting the cluster")
            else:
                self.peers.pop(change.get_node_id())
        else:
            raise NotImplementedError

        if cs := self.raw_node.apply_conf_change(change):
            last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
            snapshot = await self.store.snapshot()

            self.lmdb.set_conf_state(cs)
            self.lmdb.compact(last_applied)
            self.lmdb.create_snapshot(snapshot)

        if sender := senders.pop(seq):
            if change_type == ConfChangeType.AddNode:
                response = RaftRespJoinSuccess(
                    assigned_id=id, peer_addrs=self.peer_addrs()
                )
            elif change_type == ConfChangeType.RemoveNode:
                response = RaftRespOk()
            else:
                raise NotImplementedError

            try:
                await sender.put(response)
            except Exception:
                logging.error("Error sending response")

    async def handle_normal(self, entry: Entry_Ref, senders: Dict[int, Queue]) -> None:
        seq = decode_u64(entry.get_context())
        print("entry", entry)
        print("seqseq", entry.get_context())
        print("senders", senders)
        data = await self.store.apply(entry.get_data())

        if sender := senders.pop(seq):
            await sender.put(data)

        if time.time() > self.last_snap_time + 15:
            logging.info("Creating snapshot...")
            self.last_snap_time = time.time()
            last_applied = self.raw_node.get_raft().get_raft_log().get_applied()
            snapshot = await self.store.snapshot()
            self.lmdb.compact(last_applied)
            self.lmdb.create_snapshot(snapshot)
