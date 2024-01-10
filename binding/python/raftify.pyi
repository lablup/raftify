import abc
from typing import Any, Callable, Final, Optional

# TODO: Make these abstract types available in the Python side.
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
    def apply(self, message: bytes) -> bytes:
        raise NotImplementedError
    @abc.abstractmethod
    def snapshot(self) -> bytes:
        raise NotImplementedError
    @abc.abstractmethod
    def restore(self, snapshot: bytes) -> None:
        raise NotImplementedError
    @abc.abstractmethod
    def encode(self) -> bytes:
        raise NotImplementedError
    @classmethod
    def decode(cls, packed: bytes) -> "AbstractStateMachine":
        raise NotImplementedError

class ReadOnlyOption:
    """Determines the relative safety of and consistency of read only requests."""

    Safe: Final[Any]
    """
    Safe guarantees the linearizability of the read only request by
    communicating with the quorum. It is the default and suggested option.
    """

    LeaseBased: Final[Any]
    """
    LeaseBased ensures linearizability of the read only request by
    relying on the leader lease. It can be affected by clock drift.
    If the clock drift is unbounded, leader might keep the lease longer than it
    should (clock can move backward/pause without any bound). ReadIndex is not safe
    in that case.
    """

class Logger:
    """ """

    def __init__(self) -> None: ...
    @staticmethod
    def default() -> "Logger": ...
    @staticmethod
    def new_file_logger(
        log_path: str, chan_size: int, rotate_size: int, rotate_keep: int
    ): ...
    def info(self, s: str) -> None:
        """
        Log info level record

        See `slog_log` for documentation.
        """
    def debug(self, s: str) -> None:
        """
        Log debug level record

        See `slog_debug` for documentation.
        """
    def trace(self, s: str) -> None:
        """
        Log trace level record

        See `slog_trace` for documentation.
        """
    def crit(self, s: str) -> None:
        """
        Log crit level record

        See `slog_crit` for documentation.
        """
    def error(self, s: str) -> None:
        """
        Log error level record

        See `slog_error` for documentation.
        """

class Raft:
    def __init__(self) -> None:
        """ """
    @staticmethod
    def build(
        node_id: int,
        raft_addr: str,
        fsm: AbstractStateMachine,
        config: "Config",
        logger: "Logger",
        initial_peers: Optional["Peers"] = None,
    ) -> "Raft":
        """ """
    @staticmethod
    async def request_id(self, peer_addr: str) -> "ClusterJoinTicket":
        """"""
    def prepare_join(self, join_ticket: "ClusterJoinTicket") -> None:
        """ """
    async def join(self) -> None:
        """ """
    def prepare_member_bootstrap_ready(self, leader_addr: str, node_id: int) -> None:
        """ """
    async def member_bootstrap_ready(self) -> None:
        """ """
    async def run(self) -> None:
        """ """
    async def snapshot(self) -> None:
        """ """
    def is_finished(self) -> bool:
        """ """
    def get_raft_node(self) -> "RaftNode":
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
    def prepare_add_peer(self, id: int, addr: str) -> None:
        """ """
    async def add_peer(self) -> None:
        """ """
    async def inspect(self) -> str:
        """ """
    def prepare_proposal(self, message: bytes) -> None:
        """ """
    async def propose(self) -> None:
        """ """
    def prepare_change_config(self, conf_change: "ConfChangeV2") -> None:
        """ """
    async def change_config(self) -> None:
        """ """
    def prepare_message(self, message: "Message") -> None:
        """ """
    async def send_message(self) -> None:
        """ """
    async def leave(self) -> None:
        """ """
    async def quit(self) -> None:
        """ """
    async def get_cluster_size(self) -> int:
        """ """
    async def set_bootstrap_done(self) -> None:
        """ """
    async def store(self) -> AbstractStateMachine:
        """ """

class ClusterJoinTicket:
    """ """

    def __init__(
        self, reserved_id: int, leader_id: int, leader_addr: str, peers: "Peers"
    ) -> None: ...
    def get_reserved_id(self) -> int:
        """ """

class Peers:
    """ """

    def __init__(self, peers: dict) -> None: ...
    def to_dict(self) -> dict[int, str]: ...
    def is_empty(self) -> bool: ...
    def items(self) -> list[tuple[int, str]]: ...
    def get(self, node_id: int) -> str: ...
    def add_peer(self, node_id: int, addr: str) -> None: ...
    def remove(self, node_id: int) -> None: ...
    def get_node_id_by_addr(self, addr: str) -> int: ...

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
        read_only_option: Optional["ReadOnlyOption"] = None,
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
    def __init__(
        self,
        *,
        raft_config: Optional[RaftConfig] = None,
        log_dir: Optional[str] = None,
        save_compacted_logs: Optional[bool] = None,
        compacted_log_dir: Optional[str] = None,
        compacted_log_size_threshold: Optional[int] = None,
        snapshot_interval: Optional[float] = None,
        tick_interval: Optional[float] = None,
        lmdb_map_size: Optional[int] = None,
        cluster_id: Optional[int] = None,
        terminate_on_remove: Optional[bool] = None,
        conf_change_request_timeout: Optional[float] = None,
    ) -> None:
        """ """

class RaftServiceClient:
    """ """

    @staticmethod
    async def build(addr: str) -> "RaftServiceClient":
        """ """
    def prepare_change_config(self, conf_change: "ConfChangeV2") -> None:
        """ """
    async def change_config(self) -> None:
        """ """
    def prepare_message(self, message: "Message") -> None:
        """ """
    async def send_message(self) -> None:
        """ """
    def prepare_propose(self, proposal: bytes) -> None:
        """ """
    async def propose(self) -> None:
        """ """
    async def get_peers(self) -> str:
        """ """
    async def debug_node(self) -> str:
        """ """

def set_snapshot_data_deserializer(cb: Callable[[bytes], str | bytes | None]) -> None:
    """ """

def set_message_context_deserializer(cb: Callable[[bytes], str | bytes | None]) -> None:
    """ """

def set_confchange_context_deserializer(
    cb: Callable[[bytes], str | bytes | None]
) -> None:
    """ """

def set_confchangev2_context_deserializer(
    cb: Callable[[bytes], str | bytes | None]
) -> None:
    """ """

def set_entry_data_deserializer(cb: Callable[[bytes], str | bytes | None]) -> None:
    """ """

def set_entry_context_deserializer(cb: Callable[[bytes], str | bytes | None]) -> None:
    """ """

def set_fsm_deserializer(cb: Callable[[bytes], str | bytes | None]) -> None:
    """ """
    ...

def set_log_entry_deserializer(cb: Callable[[bytes], str | bytes | None]) -> None:
    """ """
    ...

class ConfChangeTransition:
    """ """

    Auto: Final[Any]
    """
    Automatically use the simple protocol if possible, otherwise fall back
    to ConfChangeType::Implicit. Most applications will want to use this.
    """

    Implicit: Final[Any]
    """
    Use joint consensus unconditionally, and transition out of them
    automatically (by proposing a zero configuration change).

    This option is suitable for applications that want to minimize the time
    spent in the joint configuration and do not store the joint configuration
    in the state machine (outside of InitialState).
    """

    Explicit: Final[Any]
    """
    Use joint consensus and remain in the joint configuration until the
    application proposes a no-op configuration change. This is suitable for
    applications that want to explicitly control the transitions, for example
    to use a custom payload (via the Context field).
    """

    @staticmethod
    def from_int(v: int) -> "ConfChangeTransition": ...
    def __int__(self) -> int: ...

class MessageType:
    """ """

    MsgHup: Final[Any]
    MsgBeat: Final[Any]
    MsgPropose: Final[Any]
    MsgAppend: Final[Any]
    MsgAppendResponse: Final[Any]
    MsgRequestVote: Final[Any]
    MsgRequestVoteResponse: Final[Any]
    MsgSnapshot: Final[Any]
    MsgHeartbeat: Final[Any]
    MsgHeartbeatResponse: Final[Any]
    MsgUnreachable: Final[Any]
    MsgSnapStatus: Final[Any]
    MsgCheckQuorum: Final[Any]
    MsgTransferLeader: Final[Any]
    MsgTimeoutNow: Final[Any]
    MsgReadIndex: Final[Any]
    MsgReadIndexResp: Final[Any]
    MsgRequestPreVote: Final[Any]
    MsgRequestPreVoteResponse: Final[Any]
    @staticmethod
    def from_int(v: int) -> "MessageType": ...
    def __int__(self) -> int: ...

class ConfChangeType:
    """ """

    AddNode: Final[Any]
    AddLearnerNode: Final[Any]
    RemoveNode: Final[Any]
    @staticmethod
    def from_int(v: int) -> "ConfChangeType": ...
    def __int__(self) -> int: ...

class EntryType:
    """ """

    EntryConfChange: Final[Any]
    EntryConfChangeV2: Final[Any]
    EntryNormal: Final[Any]
    @staticmethod
    def from_int(v: int) -> "EntryType": ...
    def __int__(self) -> int: ...

class Entry:
    def get_context(self) -> bytes:
        """ """
    def set_context(self, context: bytes) -> None:
        """ """
    def get_data(self) -> bytes:
        """ """
    def set_data(self, data: bytes) -> None:
        """ """
    def get_entry_type(self) -> "EntryType":
        """ """
    def set_entry_type(self, typ: "EntryType") -> None:
        """ """
    def get_term(self) -> int:
        """ """
    def set_term(self, term: int) -> None:
        """ """
    def get_index(self) -> int:
        """ """
    def set_index(self, index: int) -> None:
        """ """
    def get_sync_log(self) -> bool:
        """
        Deprecated! It is kept for backward compatibility.
        TODO: remove it in the next major release.
        """
    def set_sync_log(self, sync_log: bool) -> None:
        """
        Deprecated! It is kept for backward compatibility.
        TODO: remove it in the next major release.
        """

class ConfState:
    """ """

    def __init__(
        self, voters: Optional[list[int]], learners: Optional[list[int]]
    ) -> None: ...
    @staticmethod
    def default() -> "ConfState": ...
    def get_auto_leave(self) -> bool:
        """ """
    def set_auto_leave(self, auto_leave: bool) -> None:
        """ """
    def get_learners(self) -> list[int]:
        """ """
    def set_learners(self, learners: list[int]) -> None:
        """ """
    def get_learners_next(self) -> list[int]:
        """ """
    def set_learners_next(self, learners_next: list[int]) -> None:
        """ """
    def get_voters(self) -> list[int]:
        """ """
    def set_voters(self, voters: list[int]) -> None:
        """ """
    def get_voters_outgoing(self) -> list[int]:
        """ """
    def set_voters_outgoing(self, voters_outgoing: list[int]) -> None:
        """ """

class Snapshot:
    """ """

    def __init__(self) -> None: ...
    def get_data(self) -> bytes:
        """ """
    def set_data(self, data: bytes) -> None:
        """ """
    def get_metadata(self) -> "SnapshotMetadata":
        """ """
    def set_metadata(
        self, meta_data: "SnapshotMetadata"
    ) -> None:
        """ """
    def has_metadata(self) -> bool:
        """ """

class SnapshotMetadata:
    def __init__(self) -> None: ...
    def get_index(self) -> int:
        """
        `index`: The applied index.
        """
    def set_index(self, index: int) -> None:
        """
        `index`: The applied index.
        """
    def get_term(self) -> int:
        """
        `term`: The term of the applied index.
        """
    def set_term(self, term: int) -> None:
        """
        `term`: The term of the applied index.
        """
    def get_conf_state(self) -> "ConfState":
        """
        `conf_state`: The current `ConfState`.
        """
    def set_conf_state(self, conf_state: "ConfState") -> None:
        """
        `conf_state`: The current `ConfState`.
        """
    def has_conf_state(self) -> bool:
        """
        `conf_state`: The current `ConfState`.
        """

class ConfChangeSingle:
    def get_node_id(self) -> int:
        """ """
    def set_node_id(self, node_id: int):
        """ """
    def get_change_type(self) -> "ConfChangeType":
        """ """
    def set_change_type(self, typ: "ConfChangeType") -> None:
        """ """

class ConfChangeV2:
    def get_changes(self) -> list["ConfChangeSingle"]:
        """ """
    def set_changes(
        self, changes: list["ConfChangeSingle"] | list["ConfChangeSingle"]
    ) -> None:
        """ """
    def get_context(self) -> bytes:
        """ """
    def set_context(self, context: bytes) -> None:
        """ """
    def get_transition(self) -> "ConfChangeTransition":
        """ """
    def set_transition(self, transition: "ConfChangeTransition") -> None:
        """ """
    # def enter_joint(self) -> Optional[bool]:
    #     """
    #     Checks if uses Joint Consensus.

    #     It will return Some if and only if this config change will use Joint Consensus,
    #     which is the case if it contains more than one change or if the use of Joint
    #     Consensus was requested explicitly. The bool indicates whether the Joint State
    #     will be left automatically.
    #     """

    # def leave_joint(self) -> bool:
    #     """
    #     Checks if the configuration change leaves a joint configuration.

    #     This is the case if the ConfChangeV2 is zero, with the possible exception of
    #     the Context field.
    #     """

class Message:
    def get_commit(self) -> int:
        """ """
    def set_commit(self, commit: int) -> None:
        """ """
    def get_commit_term(self) -> int:
        """ """
    def set_commit_term(self, commit_term: int) -> None:
        """ """
    def get_from(self) -> int:
        """ """
    def set_from(self, from_: int) -> None:
        """ """
    def get_index(self) -> int:
        """ """
    def set_index(self, index: int) -> None:
        """ """
    def get_term(self) -> int:
        """ """
    def set_term(self, term: int) -> None:
        """ """
    def get_log_term(self) -> int:
        """ """
    def set_log_term(self, log_index: int) -> None:
        """ """
    def get_priority(self) -> int:
        """ """
    def set_priority(self, priority: int) -> None:
        """ """
    def get_context(self) -> bytes:
        """ """
    def set_context(self, context: bytes) -> None:
        """ """
    def get_reject_hint(self) -> int:
        """ """
    def set_reject_hint(self, reject_hint: int) -> None:
        """ """
    def get_entries(self) -> list["Entry"]:
        """ """
    def set_entries(self, ents: list["Entry"]) -> None:
        """ """
    def get_msg_type(self) -> "MessageType":
        """ """
    def set_msg_type(self, typ: "MessageType") -> None:
        """ """
    def get_reject(self) -> bool:
        """ """
    def set_reject(self, reject: bool) -> None:
        """ """
    def get_snapshot(self) -> "Snapshot":
        """ """
    def set_snapshot(self, snapshot: "Snapshot") -> None:
        """ """
    def get_to(self) -> int:
        """ """
    def set_to(self, to: int) -> None:
        """ """
    def get_request_snapshot(self) -> int:
        """ """
    def set_request_snapshot(self, request_snapshot: int) -> None:
        """ """
    def has_snapshot(self) -> bool:
        """ """
    def get_deprecated_priority(self) -> int:
        """ """
    def set_deprecated_priority(self, v: int) -> None:
        """ """

async def cli_main(argv: list[str]) -> None:
    """ """
    ...
