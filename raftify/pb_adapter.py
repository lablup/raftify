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

from raftify.protos.eraftpb_pb2 import ConfChange as Pb_ConfChange
from raftify.protos.eraftpb_pb2 import ConfChangeSingle as Pb_ConfChangeSingle
from raftify.protos.eraftpb_pb2 import ConfChangeTransition as Pb_ConfChangeTransition
from raftify.protos.eraftpb_pb2 import ConfChangeType as Pb_ConfChangeType
from raftify.protos.eraftpb_pb2 import ConfChangeV2 as Pb_ConfChangeV2
from raftify.protos.eraftpb_pb2 import ConfState as Pb_ConfState
from raftify.protos.eraftpb_pb2 import Entry as Pb_Entry
from raftify.protos.eraftpb_pb2 import EntryType as Pb_EntryType
from raftify.protos.eraftpb_pb2 import HardState as Pb_HardState
from raftify.protos.eraftpb_pb2 import Message as Pb_Message
from raftify.protos.eraftpb_pb2 import MessageType as Pb_MessageType
from raftify.protos.eraftpb_pb2 import Snapshot as Pb_Snapshot
from raftify.protos.eraftpb_pb2 import SnapshotMetadata as Pb_SnapshotMetadata


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
    @staticmethod
    def to_pb(v: ConfChangeTransition) -> Pb_ConfChangeTransition:
        match v:
            case ConfChangeTransition.Auto:
                return Pb_ConfChangeTransition.Auto
            case ConfChangeTransition.Implicit:
                return Pb_ConfChangeTransition.Implicit
            case ConfChangeTransition.Explicit:
                return Pb_ConfChangeTransition.Explicit
        assert False

    @staticmethod
    def from_pb(v: Pb_ConfChangeTransition) -> ConfChangeTransition:
        return ConfChangeTransition.from_int(v)


class ConfChangeTypeAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChangeType) -> Pb_ConfChangeType:
        match v:
            case ConfChangeType.AddNode:
                return Pb_ConfChangeType.AddNode
            case ConfChangeType.RemoveNode:
                return Pb_ConfChangeType.RemoveNode
            case ConfChangeType.AddLearnerNode:
                return Pb_ConfChangeType.AddLearnerNode
        assert False

    @staticmethod
    def from_pb(v: Pb_ConfChangeType) -> ConfChangeType:
        return ConfChangeType.from_int(v)


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
    @staticmethod
    def to_pb(v: EntryType) -> Pb_EntryType:
        match v:
            case EntryType.EntryNormal:
                return Pb_EntryType.EntryNormal
            case EntryType.EntryConfChange:
                return Pb_EntryType.EntryConfChange
            case EntryType.EntryConfChangeV2:
                return Pb_EntryType.EntryConfChangeV2
        assert False

    @staticmethod
    def from_pb(v: Pb_EntryType) -> EntryType:
        return EntryType.from_int(v)


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
    @staticmethod
    def to_pb(v: MessageType) -> Pb_MessageType:
        match v:
            case MessageType.MsgHup:
                return Pb_MessageType.MsgHup
            case MessageType.MsgBeat:
                return Pb_MessageType.MsgBeat
            case MessageType.MsgPropose:
                return Pb_MessageType.MsgPropose
            case MessageType.MsgAppend:
                return Pb_MessageType.MsgAppend
            case MessageType.MsgAppendResponse:
                return Pb_MessageType.MsgAppendResponse
            case MessageType.MsgRequestVote:
                return Pb_MessageType.MsgRequestVote
            case MessageType.MsgRequestVoteResponse:
                return Pb_MessageType.MsgRequestVoteResponse
            case MessageType.MsgSnapshot:
                return Pb_MessageType.MsgSnapshot
            case MessageType.MsgHeartbeat:
                return Pb_MessageType.MsgHeartbeat
            case MessageType.MsgHeartbeatResponse:
                return Pb_MessageType.MsgHeartbeatResponse
            case MessageType.MsgUnreachable:
                return Pb_MessageType.MsgUnreachable
            case MessageType.MsgSnapStatus:
                return Pb_MessageType.MsgSnapStatus
            case MessageType.MsgCheckQuorum:
                return Pb_MessageType.MsgCheckQuorum
            case MessageType.MsgTransferLeader:
                return Pb_MessageType.MsgTransferLeader
            case MessageType.MsgTimeoutNow:
                return Pb_MessageType.MsgTimeoutNow
            case MessageType.MsgReadIndex:
                return Pb_MessageType.MsgReadIndex
            case MessageType.MsgReadIndexResp:
                return Pb_MessageType.MsgReadIndexResp
            case MessageType.MsgRequestPreVote:
                return Pb_MessageType.MsgRequestPreVote
            case MessageType.MsgRequestPreVoteResponse:
                return Pb_MessageType.MsgRequestPreVoteResponse
        assert False

    @staticmethod
    def from_pb(v: int) -> MessageType:
        return MessageType.from_int(v)


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
