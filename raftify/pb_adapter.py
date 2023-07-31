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
from raftify.protos.eraftpb_pb2 import ConfChangeV2 as Pb_ConfChangeV2
from raftify.protos.eraftpb_pb2 import ConfState as Pb_ConfState
from raftify.protos.eraftpb_pb2 import Entry as Pb_Entry
from raftify.protos.eraftpb_pb2 import HardState as Pb_HardState
from raftify.protos.eraftpb_pb2 import Message as Pb_Message
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


class ConfChangeAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChange | ConfChangeRef) -> Pb_ConfChange:
        return Pb_ConfChange(
            id=v.get_id(),
            node_id=v.get_node_id(),
            context=v.get_context(),
            change_type=int(v.get_change_type()),
        )

    @staticmethod
    def from_pb(v: Pb_ConfChange) -> ConfChange:
        cc = ConfChange.default()
        cc.set_id(v.id)
        cc.set_node_id(v.node_id)
        cc.set_context(v.context)
        cc.set_change_type(ConfChangeType.from_int(v.change_type))
        return cc


class ConfChangeSingleAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChangeSingle | ConfChangeSingleRef) -> Pb_ConfChangeSingle:
        return Pb_ConfChangeSingle(
            node_id=v.get_node_id(), change_type=int(v.get_change_type())
        )

    @staticmethod
    def from_pb(v: Pb_ConfChangeSingle) -> ConfChangeSingle:
        cc = ConfChangeSingle.default()
        cc.set_node_id(v.node_id)
        cc.set_change_type(ConfChangeType.from_int(v.change_type))
        return cc


class ConfChangeV2Adapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: ConfChangeV2 | ConfChangeV2Ref) -> Pb_ConfChange:
        return Pb_ConfChangeV2(
            transition=int(v.get_transition()),
            changes=list(map(ConfChangeSingleAdapter.to_pb, v.get_changes())),
            context=v.get_context(),
        )

    @staticmethod
    def from_pb(v: Pb_ConfChangeV2) -> ConfChangeV2:
        cc = ConfChangeV2.default()
        cc.set_transition(ConfChangeTransition.from_int(v.transition))
        cc.set_changes(list(map(ConfChangeSingleAdapter.from_pb, v.changes)))
        cc.set_context(v.context)
        return cc


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
        cs = ConfState.default()
        cs.set_auto_leave(v.auto_leave)
        cs.set_learners(list(v.learners))
        cs.set_learners_next(list(v.learners_next))
        cs.set_voters(list(v.voters))
        cs.set_voters_outgoing(list(v.voters_outgoing))
        return cs


class EntryAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: Entry | EntryRef) -> Pb_Entry:
        return Pb_Entry(
            context=v.get_context(),
            data=v.get_data(),
            entry_type=int(v.get_entry_type()),
            index=v.get_index(),
            sync_log=v.get_sync_log(),
            term=v.get_term(),
        )

    @staticmethod
    def from_pb(v: Pb_Entry) -> Entry:
        e = Entry.default()
        e.set_context(v.context)
        e.set_data(v.data)
        e.set_entry_type(EntryType.from_int(v.entry_type))
        e.set_index(v.index)
        e.set_sync_log(v.sync_log)
        e.set_term(v.term)
        return e


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
        hs = HardState.default()
        hs.set_commit(v.commit)
        hs.set_term(v.term)
        hs.set_vote(v.vote)
        return hs


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
            msg_type=int(v.get_msg_type()),
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
        m = Message.default()
        m.set_commit(v.commit)
        m.set_commit_term(v.commit_term)
        m.set_context(v.context)
        m.set_deprecated_priority(v.deprecated_priority)
        m.set_entries(list(map(EntryAdapter.from_pb, v.entries)))
        m.set_from(v.from_)
        m.set_index(v.index)
        m.set_log_term(v.log_term)
        m.set_msg_type(MessageType.from_int(v.msg_type))
        m.set_priority(v.priority)
        m.set_reject(v.reject)
        m.set_reject_hint(v.reject_hint)
        m.set_request_snapshot(v.request_snapshot)
        m.set_snapshot(SnapshotAdapter.from_pb(v.snapshot))
        m.set_term(v.term)
        m.set_to(v.to)
        return m


class SnapshotAdapter(ProtobufAdapter):
    @staticmethod
    def to_pb(v: Snapshot | SnapshotRef) -> Pb_Snapshot:
        return Pb_Snapshot(
            data=v.get_data(),
            metadata=SnapshotMetadataAdapter.to_pb(v.get_metadata()),
        )

    @staticmethod
    def from_pb(v: Pb_Snapshot) -> Snapshot:
        s = Snapshot.default()
        s.set_data(v.data)
        s.set_metadata(SnapshotMetadataAdapter.from_pb(v.metadata))
        return s


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
        sm = SnapshotMetadata.default()
        sm.set_conf_state(ConfStateAdapter.from_pb(v.conf_state))
        sm.set_index(v.index)
        sm.set_term(v.term)
        return sm
