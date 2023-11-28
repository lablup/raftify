from typing import ClassVar as _ClassVar
from typing import Iterable as _Iterable
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper

DESCRIPTOR: _descriptor.FileDescriptor

class EntryType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EntryNormal: _ClassVar[EntryType]
    EntryConfChange: _ClassVar[EntryType]
    EntryConfChangeV2: _ClassVar[EntryType]

class MessageType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MsgHup: _ClassVar[MessageType]
    MsgBeat: _ClassVar[MessageType]
    MsgPropose: _ClassVar[MessageType]
    MsgAppend: _ClassVar[MessageType]
    MsgAppendResponse: _ClassVar[MessageType]
    MsgRequestVote: _ClassVar[MessageType]
    MsgRequestVoteResponse: _ClassVar[MessageType]
    MsgSnapshot: _ClassVar[MessageType]
    MsgHeartbeat: _ClassVar[MessageType]
    MsgHeartbeatResponse: _ClassVar[MessageType]
    MsgUnreachable: _ClassVar[MessageType]
    MsgSnapStatus: _ClassVar[MessageType]
    MsgCheckQuorum: _ClassVar[MessageType]
    MsgTransferLeader: _ClassVar[MessageType]
    MsgTimeoutNow: _ClassVar[MessageType]
    MsgReadIndex: _ClassVar[MessageType]
    MsgReadIndexResp: _ClassVar[MessageType]
    MsgRequestPreVote: _ClassVar[MessageType]
    MsgRequestPreVoteResponse: _ClassVar[MessageType]

class ConfChangeTransition(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    Auto: _ClassVar[ConfChangeTransition]
    Implicit: _ClassVar[ConfChangeTransition]
    Explicit: _ClassVar[ConfChangeTransition]

class ConfChangeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    AddNode: _ClassVar[ConfChangeType]
    AddLearnerNode: _ClassVar[ConfChangeType]
    RemoveNode: _ClassVar[ConfChangeType]

EntryNormal: EntryType
EntryConfChange: EntryType
EntryConfChangeV2: EntryType
MsgHup: MessageType
MsgBeat: MessageType
MsgPropose: MessageType
MsgAppend: MessageType
MsgAppendResponse: MessageType
MsgRequestVote: MessageType
MsgRequestVoteResponse: MessageType
MsgSnapshot: MessageType
MsgHeartbeat: MessageType
MsgHeartbeatResponse: MessageType
MsgUnreachable: MessageType
MsgSnapStatus: MessageType
MsgCheckQuorum: MessageType
MsgTransferLeader: MessageType
MsgTimeoutNow: MessageType
MsgReadIndex: MessageType
MsgReadIndexResp: MessageType
MsgRequestPreVote: MessageType
MsgRequestPreVoteResponse: MessageType
Auto: ConfChangeTransition
Implicit: ConfChangeTransition
Explicit: ConfChangeTransition
AddNode: ConfChangeType
AddLearnerNode: ConfChangeType
RemoveNode: ConfChangeType

class Entry(_message.Message):
    __slots__ = ("entry_type", "term", "index", "data", "context", "sync_log")
    ENTRY_TYPE_FIELD_NUMBER: _ClassVar[int]
    TERM_FIELD_NUMBER: _ClassVar[int]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    SYNC_LOG_FIELD_NUMBER: _ClassVar[int]
    entry_type: EntryType
    term: int
    index: int
    data: bytes
    context: bytes
    sync_log: bool
    def __init__(
        self,
        entry_type: _Optional[_Union[EntryType, str]] = ...,
        term: _Optional[int] = ...,
        index: _Optional[int] = ...,
        data: _Optional[bytes] = ...,
        context: _Optional[bytes] = ...,
        sync_log: bool = ...,
    ) -> None: ...

class SnapshotMetadata(_message.Message):
    __slots__ = ("conf_state", "index", "term")
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

class Snapshot(_message.Message):
    __slots__ = ("data", "metadata")
    DATA_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes
    metadata: SnapshotMetadata
    def __init__(
        self,
        data: _Optional[bytes] = ...,
        metadata: _Optional[_Union[SnapshotMetadata, _Mapping]] = ...,
    ) -> None: ...

class Message(_message.Message):
    __slots__ = (
        "msg_type",
        "to",
        "from_",
        "term",
        "log_term",
        "index",
        "entries",
        "commit",
        "commit_term",
        "snapshot",
        "request_snapshot",
        "reject",
        "reject_hint",
        "context",
        "deprecated_priority",
        "priority",
    )
    MSG_TYPE_FIELD_NUMBER: _ClassVar[int]
    TO_FIELD_NUMBER: _ClassVar[int]
    FROM__FIELD_NUMBER: _ClassVar[int]
    TERM_FIELD_NUMBER: _ClassVar[int]
    LOG_TERM_FIELD_NUMBER: _ClassVar[int]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    ENTRIES_FIELD_NUMBER: _ClassVar[int]
    COMMIT_FIELD_NUMBER: _ClassVar[int]
    COMMIT_TERM_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    REQUEST_SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    REJECT_FIELD_NUMBER: _ClassVar[int]
    REJECT_HINT_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    DEPRECATED_PRIORITY_FIELD_NUMBER: _ClassVar[int]
    PRIORITY_FIELD_NUMBER: _ClassVar[int]
    msg_type: MessageType
    to: int
    from_: int
    term: int
    log_term: int
    index: int
    entries: _containers.RepeatedCompositeFieldContainer[Entry]
    commit: int
    commit_term: int
    snapshot: Snapshot
    request_snapshot: int
    reject: bool
    reject_hint: int
    context: bytes
    deprecated_priority: int
    priority: int
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
        commit_term: _Optional[int] = ...,
        snapshot: _Optional[_Union[Snapshot, _Mapping]] = ...,
        request_snapshot: _Optional[int] = ...,
        reject: bool = ...,
        reject_hint: _Optional[int] = ...,
        context: _Optional[bytes] = ...,
        deprecated_priority: _Optional[int] = ...,
        priority: _Optional[int] = ...,
    ) -> None: ...

class HardState(_message.Message):
    __slots__ = ("term", "vote", "commit")
    TERM_FIELD_NUMBER: _ClassVar[int]
    VOTE_FIELD_NUMBER: _ClassVar[int]
    COMMIT_FIELD_NUMBER: _ClassVar[int]
    term: int
    vote: int
    commit: int
    def __init__(
        self,
        term: _Optional[int] = ...,
        vote: _Optional[int] = ...,
        commit: _Optional[int] = ...,
    ) -> None: ...

class ConfState(_message.Message):
    __slots__ = ("voters", "learners", "voters_outgoing", "learners_next", "auto_leave")
    VOTERS_FIELD_NUMBER: _ClassVar[int]
    LEARNERS_FIELD_NUMBER: _ClassVar[int]
    VOTERS_OUTGOING_FIELD_NUMBER: _ClassVar[int]
    LEARNERS_NEXT_FIELD_NUMBER: _ClassVar[int]
    AUTO_LEAVE_FIELD_NUMBER: _ClassVar[int]
    voters: _containers.RepeatedScalarFieldContainer[int]
    learners: _containers.RepeatedScalarFieldContainer[int]
    voters_outgoing: _containers.RepeatedScalarFieldContainer[int]
    learners_next: _containers.RepeatedScalarFieldContainer[int]
    auto_leave: bool
    def __init__(
        self,
        voters: _Optional[_Iterable[int]] = ...,
        learners: _Optional[_Iterable[int]] = ...,
        voters_outgoing: _Optional[_Iterable[int]] = ...,
        learners_next: _Optional[_Iterable[int]] = ...,
        auto_leave: bool = ...,
    ) -> None: ...

class ConfChange(_message.Message):
    __slots__ = ("change_type", "node_id", "context", "id")
    CHANGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    change_type: ConfChangeType
    node_id: int
    context: bytes
    id: int
    def __init__(
        self,
        change_type: _Optional[_Union[ConfChangeType, str]] = ...,
        node_id: _Optional[int] = ...,
        context: _Optional[bytes] = ...,
        id: _Optional[int] = ...,
    ) -> None: ...

class ConfChangeSingle(_message.Message):
    __slots__ = ("change_type", "node_id")
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
    __slots__ = ("transition", "changes", "context")
    TRANSITION_FIELD_NUMBER: _ClassVar[int]
    CHANGES_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    transition: ConfChangeTransition
    changes: _containers.RepeatedCompositeFieldContainer[ConfChangeSingle]
    context: bytes
    def __init__(
        self,
        transition: _Optional[_Union[ConfChangeTransition, str]] = ...,
        changes: _Optional[_Iterable[_Union[ConfChangeSingle, _Mapping]]] = ...,
        context: _Optional[bytes] = ...,
    ) -> None: ...
