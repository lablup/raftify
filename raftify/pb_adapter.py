from abc import ABCMeta
from typing import Any

from rraft import ConfChange as ConfChange
from rraft import (
    ConfChangeRef,
    ConfChangeSingle,
    ConfChangeSingleRef,
    ConfChangeTransition,
    ConfChangeType,
    ConfChangeV2,
    ConfChangeV2Ref,
    ConfState,
    ConfStateRef,
    Entry,
    EntryRef,
    EntryType,
    HardState,
    HardStateRef,
    Message,
    MessageRef,
    MessageType,
    Snapshot,
    SnapshotMetadata,
    SnapshotMetadataRef,
    SnapshotRef,
)

from .protos.eraftpb_pb2 import ConfChange as Pb_ConfChange
from .protos.eraftpb_pb2 import ConfChangeSingle as Pb_ConfChangeSingle
from .protos.eraftpb_pb2 import ConfChangeTransition as Pb_ConfChangeTransition
from .protos.eraftpb_pb2 import ConfChangeType as Pb_ConfChangeType
from .protos.eraftpb_pb2 import ConfChangeV2 as Pb_ConfChangeV2
from .protos.eraftpb_pb2 import ConfState as Pb_ConfState
from .protos.eraftpb_pb2 import Entry as Pb_Entry
from .protos.eraftpb_pb2 import EntryType as Pb_EntryType
from .protos.eraftpb_pb2 import HardState as Pb_HardState
from .protos.eraftpb_pb2 import Message as Pb_Message
from .protos.eraftpb_pb2 import MessageType as Pb_MessageType
from .protos.eraftpb_pb2 import Snapshot as Pb_Snapshot
from .protos.eraftpb_pb2 import SnapshotMetadata as Pb_SnapshotMetadata


class ProtobufAdapter(metaclass=ABCMeta):
    """
    The types used in protobuf have the same names as the types in rraft-py,
    but they should be handled as different kinds of serializable pure Python types.
    Therefore, an adapter is needed to bridge the Python types used in protobuf and the types in rraft-py.
    This class defines the interface for such an adapter.
    """

    @staticmethod
    def to_pb(v: Any) -> Any:
        raise NotImplementedError

    @staticmethod
    def from_pb(v: Any) -> Any:
        raise NotImplementedError


class ConfChangeTransitionAdapter(ProtobufAdapter):
    to_pb_adapter = {
        ConfChangeTransition.Auto: Pb_ConfChangeTransition.Auto,
        ConfChangeTransition.Implicit: Pb_ConfChangeTransition.Implicit,
        ConfChangeTransition.Explicit: Pb_ConfChangeTransition.Explicit,
    }

    from_pb_adapter = {value: key for key, value in to_pb_adapter.items()}

    @staticmethod
    def to_pb(v: ConfChangeTransition) -> Pb_ConfChangeTransition:
        return ConfChangeTransitionAdapter.to_pb_adapter[v]

    @staticmethod
    def from_pb(v: Pb_ConfChangeTransition) -> ConfChangeTransition:
        return ConfChangeTransitionAdapter.from_pb_adapter[v]


class ConfChangeTypeAdapter(ProtobufAdapter):
    to_pb_adapter: dict[ConfChangeType, Pb_ConfChangeType] = {
        ConfChangeType.AddNode: Pb_ConfChangeType.AddNode,
        ConfChangeType.RemoveNode: Pb_ConfChangeType.RemoveNode,
        ConfChangeType.AddLearnerNode: Pb_ConfChangeType.AddLearnerNode,
    }

    from_pb_adapter: dict[Pb_ConfChangeType, ConfChangeType] = {
        value: key for key, value in to_pb_adapter.items()
    }

    @staticmethod
    def to_pb(v: ConfChangeType) -> Pb_ConfChangeType:
        return ConfChangeTypeAdapter.to_pb_adapter[v]

    @staticmethod
    def from_pb(v: Pb_ConfChangeType) -> ConfChangeType:
        return ConfChangeTypeAdapter.from_pb_adapter[v]


class ConfChangeAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChange | ConfChangeRef) -> Pb_ConfChange:
        return Pb_ConfChange(
            id=v.get_id(),
            node_id=v.get_node_id(),
            context=v.get_context(),
            change_type=ConfChangeTypeAdapter.to_pb(v.get_change_type()),
        )

    @staticmethod
    def from_pb(v: Pb_ConfChange) -> ConfChange:
        conf_change = ConfChange.default()
        conf_change.set_id(v.id)
        conf_change.set_node_id(v.node_id)
        conf_change.set_context(v.context)
        conf_change.set_change_type(ConfChangeTypeAdapter.from_pb(v.change_type))
        return conf_change


class ConfChangeSingleAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChangeSingle | ConfChangeSingleRef) -> Pb_ConfChangeSingle:
        return Pb_ConfChangeSingle(
            node_id=v.get_node_id(),
            change_type=ConfChangeTypeAdapter.to_pb(v.get_change_type()),
        )

    @staticmethod
    def from_pb(v: Pb_ConfChangeSingle) -> ConfChangeSingle:
        conf_change = ConfChangeSingle.default()
        conf_change.set_node_id(v.node_id)
        conf_change.set_change_type(ConfChangeTypeAdapter.from_pb(v.change_type))
        return conf_change


class ConfChangeV2Adapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChangeV2 | ConfChangeV2Ref) -> Pb_ConfChangeV2:
        return Pb_ConfChangeV2(
            transition=ConfChangeTransitionAdapter.to_pb(v.get_transition()),
            changes=list(map(ConfChangeSingleAdapter.to_pb, v.get_changes())),
            context=v.get_context(),
        )

    @staticmethod
    def from_pb(v: Pb_ConfChangeV2) -> ConfChangeV2:
        conf_change = ConfChangeV2.default()
        conf_change.set_transition(ConfChangeTransition.from_int(v.transition))
        conf_change.set_changes(list(map(ConfChangeSingleAdapter.from_pb, v.changes)))
        conf_change.set_context(v.context)
        return conf_change


class ConfStateAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfState | ConfStateRef) -> Pb_ConfState:
        return Pb_ConfState(
            auto_leave=v.get_auto_leave(),
            learners=v.get_learners(),
            learners_next=v.get_learners_next(),
            voters=v.get_voters(),
            voters_outgoing=v.get_voters_outgoing(),
        )

    @staticmethod
    def from_pb(v: Pb_ConfState) -> ConfState:
        conf_state = ConfState.default()
        conf_state.set_auto_leave(v.auto_leave)
        conf_state.set_learners(list(v.learners))
        conf_state.set_learners_next(list(v.learners_next))
        conf_state.set_voters(list(v.voters))
        conf_state.set_voters_outgoing(list(v.voters_outgoing))
        return conf_state


class EntryTypeAdapter(ProtobufAdapter):
    to_pb_adapter = {
        EntryType.EntryNormal: Pb_EntryType.EntryNormal,
        EntryType.EntryConfChange: Pb_EntryType.EntryConfChange,
        EntryType.EntryConfChangeV2: Pb_EntryType.EntryConfChangeV2,
    }

    from_pb_adapter = {value: key for key, value in to_pb_adapter.items()}

    @staticmethod
    def to_pb(v: EntryType) -> Pb_EntryType:
        return EntryTypeAdapter.to_pb_adapter[v]

    @staticmethod
    def from_pb(v: Pb_EntryType) -> EntryType:
        return EntryTypeAdapter.from_pb_adapter[v]


class EntryAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: Entry | EntryRef) -> Pb_Entry:
        return Pb_Entry(
            context=v.get_context(),
            data=v.get_data(),
            entry_type=EntryTypeAdapter.to_pb(v.get_entry_type()),
            index=v.get_index(),
            sync_log=v.get_sync_log(),
            term=v.get_term(),
        )

    @staticmethod
    def from_pb(v: Pb_Entry) -> Entry:
        entry = Entry.default()
        entry.set_context(v.context)
        entry.set_data(v.data)
        entry.set_entry_type(EntryTypeAdapter.from_pb(v.entry_type))
        entry.set_index(v.index)
        entry.set_sync_log(v.sync_log)
        entry.set_term(v.term)
        return entry


class HardStateAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: HardState | HardStateRef) -> Pb_HardState:
        return Pb_HardState(
            commit=v.get_commit(),
            term=v.get_term(),
            vote=v.get_vote(),
        )

    @staticmethod
    def from_pb(v: Pb_HardState) -> HardState:
        hard_state = HardState.default()
        hard_state.set_commit(v.commit)
        hard_state.set_term(v.term)
        hard_state.set_vote(v.vote)
        return hard_state


class MessageTypeAdapter(ProtobufAdapter):
    to_pb_adapter = {
        MessageType.MsgHup: Pb_MessageType.MsgHup,
        MessageType.MsgBeat: Pb_MessageType.MsgBeat,
        MessageType.MsgPropose: Pb_MessageType.MsgPropose,
        MessageType.MsgAppend: Pb_MessageType.MsgAppend,
        MessageType.MsgAppendResponse: Pb_MessageType.MsgAppendResponse,
        MessageType.MsgRequestVote: Pb_MessageType.MsgRequestVote,
        MessageType.MsgRequestVoteResponse: Pb_MessageType.MsgRequestVoteResponse,
        MessageType.MsgSnapshot: Pb_MessageType.MsgSnapshot,
        MessageType.MsgHeartbeat: Pb_MessageType.MsgHeartbeat,
        MessageType.MsgHeartbeatResponse: Pb_MessageType.MsgHeartbeatResponse,
        MessageType.MsgUnreachable: Pb_MessageType.MsgUnreachable,
        MessageType.MsgSnapStatus: Pb_MessageType.MsgSnapStatus,
        MessageType.MsgCheckQuorum: Pb_MessageType.MsgCheckQuorum,
        MessageType.MsgTransferLeader: Pb_MessageType.MsgTransferLeader,
        MessageType.MsgTimeoutNow: Pb_MessageType.MsgTimeoutNow,
        MessageType.MsgReadIndex: Pb_MessageType.MsgReadIndex,
        MessageType.MsgReadIndexResp: Pb_MessageType.MsgReadIndexResp,
        MessageType.MsgRequestPreVote: Pb_MessageType.MsgRequestPreVote,
        MessageType.MsgRequestPreVoteResponse: Pb_MessageType.MsgRequestPreVoteResponse,
    }

    from_pb_adapter = {value: key for key, value in to_pb_adapter.items()}

    @staticmethod
    def to_pb(v: MessageType) -> Pb_MessageType:
        return MessageTypeAdapter.to_pb_adapter[v]

    @staticmethod
    def from_pb(v: Pb_MessageType) -> MessageType:
        return MessageTypeAdapter.from_pb_adapter[v]


class MessageAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: Message | MessageRef) -> Pb_Message:
        return Pb_Message(
            commit=v.get_commit(),
            commit_term=v.get_commit_term(),
            context=v.get_context(),
            deprecated_priority=v.get_deprecated_priority(),
            entries=list(map(EntryAdapter.to_pb, v.get_entries())),
            from_=v.get_from(),
            index=v.get_index(),
            log_term=v.get_log_term(),
            msg_type=MessageTypeAdapter.to_pb(v.get_msg_type()),
            priority=v.get_priority(),
            reject=v.get_reject(),
            reject_hint=v.get_reject_hint(),
            request_snapshot=v.get_request_snapshot(),
            snapshot=SnapshotAdapter.to_pb(v.get_snapshot()),
            term=v.get_term(),
            to=v.get_to(),
        )

    @staticmethod
    def from_pb(v: Pb_Message) -> Message:
        message = Message.default()
        message.set_commit(v.commit)
        message.set_commit_term(v.commit_term)
        message.set_context(v.context)
        message.set_deprecated_priority(v.deprecated_priority)
        message.set_entries(list(map(EntryAdapter.from_pb, v.entries)))
        message.set_from(v.from_)
        message.set_index(v.index)
        message.set_log_term(v.log_term)
        message.set_msg_type(MessageTypeAdapter.from_pb(v.msg_type))
        message.set_priority(v.priority)
        message.set_reject(v.reject)
        message.set_reject_hint(v.reject_hint)
        message.set_request_snapshot(v.request_snapshot)
        message.set_snapshot(SnapshotAdapter.from_pb(v.snapshot))
        message.set_term(v.term)
        message.set_to(v.to)
        return message


class SnapshotAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: Snapshot | SnapshotRef) -> Pb_Snapshot:
        return Pb_Snapshot(
            data=v.get_data(),
            metadata=SnapshotMetadataAdapter.to_pb(v.get_metadata()),
        )

    @staticmethod
    def from_pb(v: Pb_Snapshot) -> Snapshot:
        snapshot = Snapshot.default()
        snapshot.set_data(v.data)
        snapshot.set_metadata(SnapshotMetadataAdapter.from_pb(v.metadata))
        return snapshot


class SnapshotMetadataAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: SnapshotMetadata | SnapshotMetadataRef) -> Pb_SnapshotMetadata:
        return Pb_SnapshotMetadata(
            conf_state=ConfStateAdapter.to_pb(v.get_conf_state()),
            index=v.get_index(),
            term=v.get_term(),
        )

    @staticmethod
    def from_pb(v: Pb_SnapshotMetadata) -> SnapshotMetadata:
        snapshot_metadata = SnapshotMetadata.default()
        snapshot_metadata.set_conf_state(ConfStateAdapter.from_pb(v.conf_state))
        snapshot_metadata.set_index(v.index)
        snapshot_metadata.set_term(v.term)
        return snapshot_metadata
