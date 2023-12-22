import abc
from dataclasses import dataclass
from typing import Any, Optional

class AbstractLogEntry(metaclass=abc.ABCMeta):
    @abc.abstractmethod
    def encode(self) -> bytes:
        raise NotImplementedError
    @classmethod
    def decode(cls, packed: bytes) -> "AbstractLogEntry":
        raise NotImplementedError

class AbstractStateMachine(metaclass=abc.ABCMeta):
    """
    A Finite State Machine (FSM) class.
    This class is designed to apply commands to a state, take snapshots of the state,
    and restore the state from a snapshot.
    """

    @abc.abstractmethod
    async def apply(self, message: bytes) -> bytes:
        raise NotImplementedError
    @abc.abstractmethod
    async def snapshot(self) -> bytes:
        raise NotImplementedError
    @abc.abstractmethod
    async def restore(self, snapshot: bytes) -> None:
        raise NotImplementedError

class Raft:
    @staticmethod
    def build(
        node_id: int,
        raft_addr: str,
        fsm: AbstractStateMachine,
        config: "Config",
        initial_peers: Optional["Peers"] = None,
    ) -> "Raft":
        """ """
    async def run(self) -> None:
        """ """
    @staticmethod
    async def request_id(peer_addr: str) -> "RequestIdResponse":
        """"""
    async def member_bootstrap_ready(self, leader_addr: str, node_id: int) -> None:
        """ """
    async def snapshot(self) -> None:
        """ """
    async def cluster_size(self) -> int:
        """ """
    async def join(ticket: "RequestIdResponse") -> None:
        """ """

class RaftNode:
    async def is_leader(self) -> bool:
        """ """
    async def get_id(self) -> int:
        """ """
    async def get_leader_id(self) -> int:
        """ """
    async def get_peers(self) -> "Peers":
        """ """
    async def add_peer(self) -> None:
        """ """
    async def inspect(self) -> str:
        """ """
    async def propose(self) -> None:
        """ """
    async def change_config(self) -> None:
        """ """
    async def leave(self) -> None:
        """ """
    async def quit(self) -> None:
        """ """

@dataclass
class RequestIdResponse:
    """ """

    reserved_id: int
    leader_id: int
    leader_addr: str
    peers: "Peers"

class Peers:
    """ """

    pass

class RaftConfig:
    """
    Config contains the parameters to start a raft.
    """

    def __init__(
        self,
        *,
        id: Optional[int] = None,
        election_tick: Optional[int] = None,
        heartbeat_tick: Optional[int] = None,
        applied: Optional[int] = None,
        max_size_per_msg: Optional[int] = None,
        max_inflight_msgs: Optional[int] = None,
        check_quorum: Optional[bool] = None,
        pre_vote: Optional[bool] = None,
        min_election_tick: Optional[int] = None,
        max_election_tick: Optional[int] = None,
        # read_only_option: Optional["ReadOnlyOption"] = None,
        skip_bcast_commit: Optional[bool] = None,
        batch_append: Optional[bool] = None,
        priority: Optional[int] = None,
        max_uncommitted_size: Optional[int] = None,
        max_committed_size_per_ready: Optional[int] = None,
    ) -> None:
        """
        :param id: The identity of the local raft. It cannot be 0, and must be unique in the group.

        :param election_tick: The number of node.tick invocations that must pass between
        elections. That is, if a follower does not receive any message from the
        leader of current term before ElectionTick has elapsed, it will become
        candidate and start an election. election_tick must be greater than
        HeartbeatTick. We suggest election_tick = 10 * HeartbeatTick to avoid
        unnecessary leader switching

        :param heartbeat_tick: HeartbeatTick is the number of node.tick invocations that must pass between
        heartbeats. That is, a leader sends heartbeat messages to maintain its
        leadership every heartbeat ticks.

        :param applied: Applied is the last applied index. It should only be set when restarting
        raft. raft will not return entries to the application smaller or equal to Applied.
        If Applied is unset when restarting, raft might return previous applied entries.
        This is a very application dependent configuration.

        :param max_size_per_msg: Limit the max size of each append message. Smaller value lowers
        the raft recovery cost(initial probing and message lost during normal operation).
        On the other side, it might affect the throughput during normal replication.
        Note: math.MaxUusize64 for unlimited, 0 for at most one entry per message.

        :param max_inflight_msgs: Limit the max number of in-flight append messages during optimistic
        replication phase. The application transportation layer usually has its own sending
        buffer over TCP/UDP. Set to avoid overflowing that sending buffer.

        :param check_quorum: Specify if the leader should check quorum activity. Leader steps down when
        quorum is not active for an electionTimeout.

        :param pre_vote: Enables the Pre-Vote algorithm described in raft thesis section
        9.6. This prevents disruption when a node that has been partitioned away
        rejoins the cluster.

        :param min_election_tick: The range of election timeout. In some cases, we hope some nodes has less possibility
        to become leader. This configuration ensures that the randomized election_timeout
        will always be suit in [min_election_tick, max_election_tick).
        If it is 0, then election_tick will be chosen.

        :param max_election_tick: If it is 0, then 2 * election_tick will be chosen.

        :param read_only_option: Choose the linearizability mode or the lease mode to read data. If you don’t care about the read consistency and want a higher read performance, you can use the lease mode.
        Setting this to `LeaseBased` requires `check_quorum = true`.

        :param skip_bcast_commit: Don't broadcast an empty raft entry to notify follower to commit an entry.
        This may make follower wait a longer time to apply an entry. This configuration
        May affect proposal forwarding and follower read.

        :param batch_append: Batches every append msg if any append msg already exists

        :param priority: The election priority of this node.

        :param max_uncommitted_size: Specify maximum of uncommitted entry size.
        When this limit is reached, all proposals to append new log will be dropped

        :param max_committed_size_per_ready: Max size for committed entries in a `Ready`.
        """

class Config:
    """ """

class RaftClient:
    """ """

    async def change_config(self) -> None:
        """ """
    async def send_message(self) -> None:
        """ """
