from typing import ClassVar as _ClassVar
from typing import Iterable as _Iterable
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper

AddLearnerNode: ConfChangeType
AddNode: ConfChangeType
Auto: ConfChangeTransition
DESCRIPTOR: _descriptor.FileDescriptor
EntryConfChange: EntryType
EntryConfChangeV2: EntryType
EntryNormal: EntryType
Explicit: ConfChangeTransition
Implicit: ConfChangeTransition
MsgAppend: MessageType
MsgAppendResponse: MessageType
MsgBeat: MessageType
MsgCheckQuorum: MessageType
MsgHeartbeat: MessageType
MsgHeartbeatResponse: MessageType
MsgHup: MessageType
MsgPropose: MessageType
MsgReadIndex: MessageType
MsgReadIndexResp: MessageType
MsgRequestPreVote: MessageType
MsgRequestPreVoteResponse: MessageType
MsgRequestVote: MessageType
MsgRequestVoteResponse: MessageType
MsgSnapStatus: MessageType
MsgSnapshot: MessageType
MsgTimeoutNow: MessageType
MsgTransferLeader: MessageType
MsgUnreachable: MessageType
RemoveNode: ConfChangeType

class ConfChange(_message.Message):
    __slots__ = ["change_type", "context", "id", "node_id"]
    CHANGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    change_type: ConfChangeType
    context: bytes
    id: int
    node_id: int
    def __init__(
        self,
        change_type: _Optional[_Union[ConfChangeType, str]] = ...,
        node_id: _Optional[int] = ...,
        context: _Optional[bytes] = ...,
        id: _Optional[int] = ...,
    ) -> None: ...

class ConfChangeSingle(_message.Message):
    __slots__ = ["change_type", "node_id"]
    CHANGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    change_type: ConfChangeType
    node_id: int
    def __init__(
        self,
        change_type: _Optional[_Union[ConfChangeType, str]] = ...,
        node_id: _Optional[int] = ...,
    ) -> None: ...

class ConfChangeV2(_message.Message):
    __slots__ = ["changes", "context", "transition"]
    CHANGES_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_FIELD_NUMBER: _ClassVar[int]
    changes: _containers.RepeatedCompositeFieldContainer[ConfChangeSingle]
    context: bytes
    transition: ConfChangeTransition
    def __init__(
        self,
        transition: _Optional[_Union[ConfChangeTransition, str]] = ...,
        changes: _Optional[_Iterable[_Union[ConfChangeSingle, _Mapping]]] = ...,
        context: _Optional[bytes] = ...,
    ) -> None: ...

class ConfState(_message.Message):
    __slots__ = ["auto_leave", "learners", "learners_next", "voters", "voters_outgoing"]
    AUTO_LEAVE_FIELD_NUMBER: _ClassVar[int]
    LEARNERS_FIELD_NUMBER: _ClassVar[int]
    LEARNERS_NEXT_FIELD_NUMBER: _ClassVar[int]
    VOTERS_FIELD_NUMBER: _ClassVar[int]
    VOTERS_OUTGOING_FIELD_NUMBER: _ClassVar[int]
    auto_leave: bool
    learners: _containers.RepeatedScalarFieldContainer[int]
    learners_next: _containers.RepeatedScalarFieldContainer[int]
    voters: _containers.RepeatedScalarFieldContainer[int]
    voters_outgoing: _containers.RepeatedScalarFieldContainer[int]
    def __init__(
        self,
        voters: _Optional[_Iterable[int]] = ...,
        learners: _Optional[_Iterable[int]] = ...,
        voters_outgoing: _Optional[_Iterable[int]] = ...,
        learners_next: _Optional[_Iterable[int]] = ...,
        auto_leave: bool = ...,
    ) -> None: ...

class Entry(_message.Message):
    __slots__ = ["context", "data", "entry_type", "index", "sync_log", "term"]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    ENTRY_TYPE_FIELD_NUMBER: _ClassVar[int]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    SYNC_LOG_FIELD_NUMBER: _ClassVar[int]
    TERM_FIELD_NUMBER: _ClassVar[int]
    context: bytes
    data: bytes
    entry_type: EntryType
    index: int
    sync_log: bool
    term: int
    def __init__(
        self,
        entry_type: _Optional[_Union[EntryType, str]] = ...,
        term: _Optional[int] = ...,
        index: _Optional[int] = ...,
        data: _Optional[bytes] = ...,
        context: _Optional[bytes] = ...,
        sync_log: bool = ...,
    ) -> None: ...

class HardState(_message.Message):
    __slots__ = ["commit", "term", "vote"]
    COMMIT_FIELD_NUMBER: _ClassVar[int]
    TERM_FIELD_NUMBER: _ClassVar[int]
    VOTE_FIELD_NUMBER: _ClassVar[int]
    commit: int
    term: int
    vote: int
    def __init__(
        self,
        term: _Optional[int] = ...,
        vote: _Optional[int] = ...,
        commit: _Optional[int] = ...,
    ) -> None: ...

class Message(_message.Message):
    __slots__ = [
        "commit",
        "commit_term",
        "context",
        "entries",
        "from_",
        "index",
        "log_term",
        "msg_type",
        "priority",
        "reject",
        "reject_hint",
        "request_snapshot",
        "snapshot",
        "term",
        "to",
    ]
    COMMIT_FIELD_NUMBER: _ClassVar[int]
    COMMIT_TERM_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    ENTRIES_FIELD_NUMBER: _ClassVar[int]
    FROM__FIELD_NUMBER: _ClassVar[int]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    LOG_TERM_FIELD_NUMBER: _ClassVar[int]
    MSG_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIORITY_FIELD_NUMBER: _ClassVar[int]
    REJECT_FIELD_NUMBER: _ClassVar[int]
    REJECT_HINT_FIELD_NUMBER: _ClassVar[int]
    REQUEST_SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    TERM_FIELD_NUMBER: _ClassVar[int]
    TO_FIELD_NUMBER: _ClassVar[int]
    commit: int
    commit_term: int
    context: bytes
    entries: _containers.RepeatedCompositeFieldContainer[Entry]
    from_: int
    index: int
    log_term: int
    msg_type: MessageType
    priority: int
    reject: bool
    reject_hint: int
    request_snapshot: int
    snapshot: Snapshot
    term: int
    to: int
    def __init__(
        self,
        msg_type: _Optional[_Union[MessageType, str]] = ...,
        to: _Optional[int] = ...,
        from_: _Optional[int] = ...,
        term: _Optional[int] = ...,
        log_term: _Optional[int] = ...,
        index: _Optional[int] = ...,
        entries: _Optional[_Iterable[_Union[Entry, _Mapping]]] = ...,
        commit: _Optional[int] = ...,
        snapshot: _Optional[_Union[Snapshot, _Mapping]] = ...,
        request_snapshot: _Optional[int] = ...,
        reject: bool = ...,
        reject_hint: _Optional[int] = ...,
        context: _Optional[bytes] = ...,
        priority: _Optional[int] = ...,
        commit_term: _Optional[int] = ...,
    ) -> None: ...

class Snapshot(_message.Message):
    __slots__ = ["data", "metadata"]
    DATA_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes
    metadata: SnapshotMetadata
    def __init__(
        self,
        data: _Optional[bytes] = ...,
        metadata: _Optional[_Union[SnapshotMetadata, _Mapping]] = ...,
    ) -> None: ...

class SnapshotMetadata(_message.Message):
    __slots__ = ["conf_state", "index", "term"]
    CONF_STATE_FIELD_NUMBER: _ClassVar[int]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    TERM_FIELD_NUMBER: _ClassVar[int]
    conf_state: ConfState
    index: int
    term: int
    def __init__(
        self,
        conf_state: _Optional[_Union[ConfState, _Mapping]] = ...,
        index: _Optional[int] = ...,
        term: _Optional[int] = ...,
    ) -> None: ...

class EntryType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []

class MessageType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []

class ConfChangeTransition(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []

class ConfChangeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
