# type: ignore
# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: eraftpb.proto
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder

# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()


DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(
    b'\n\reraftpb.proto\x12\x07\x65raftpb"}\n\x05\x45ntry\x12&\n\nentry_type\x18\x01 \x01(\x0e\x32\x12.eraftpb.EntryType\x12\x0c\n\x04term\x18\x02 \x01(\x04\x12\r\n\x05index\x18\x03 \x01(\x04\x12\x0c\n\x04\x64\x61ta\x18\x04 \x01(\x0c\x12\x0f\n\x07\x63ontext\x18\x06 \x01(\x0c\x12\x10\n\x08sync_log\x18\x05 \x01(\x08"W\n\x10SnapshotMetadata\x12&\n\nconf_state\x18\x01 \x01(\x0b\x32\x12.eraftpb.ConfState\x12\r\n\x05index\x18\x02 \x01(\x04\x12\x0c\n\x04term\x18\x03 \x01(\x04"E\n\x08Snapshot\x12\x0c\n\x04\x64\x61ta\x18\x01 \x01(\x0c\x12+\n\x08metadata\x18\x02 \x01(\x0b\x32\x19.eraftpb.SnapshotMetadata"\xb2\x02\n\x07Message\x12&\n\x08msg_type\x18\x01 \x01(\x0e\x32\x14.eraftpb.MessageType\x12\n\n\x02to\x18\x02 \x01(\x04\x12\x0c\n\x04\x66rom\x18\x03 \x01(\x04\x12\x0c\n\x04term\x18\x04 \x01(\x04\x12\x10\n\x08log_term\x18\x05 \x01(\x04\x12\r\n\x05index\x18\x06 \x01(\x04\x12\x1f\n\x07\x65ntries\x18\x07 \x03(\x0b\x32\x0e.eraftpb.Entry\x12\x0e\n\x06\x63ommit\x18\x08 \x01(\x04\x12#\n\x08snapshot\x18\t \x01(\x0b\x32\x11.eraftpb.Snapshot\x12\x18\n\x10request_snapshot\x18\r \x01(\x04\x12\x0e\n\x06reject\x18\n \x01(\x08\x12\x13\n\x0breject_hint\x18\x0b \x01(\x04\x12\x0f\n\x07\x63ontext\x18\x0c \x01(\x0c\x12\x10\n\x08priority\x18\x0e \x01(\x04"7\n\tHardState\x12\x0c\n\x04term\x18\x01 \x01(\x04\x12\x0c\n\x04vote\x18\x02 \x01(\x04\x12\x0e\n\x06\x63ommit\x18\x03 \x01(\x04"q\n\tConfState\x12\x0e\n\x06voters\x18\x01 \x03(\x04\x12\x10\n\x08learners\x18\x02 \x03(\x04\x12\x17\n\x0fvoters_outgoing\x18\x03 \x03(\x04\x12\x15\n\rlearners_next\x18\x04 \x03(\x04\x12\x12\n\nauto_leave\x18\x05 \x01(\x08"h\n\nConfChange\x12,\n\x0b\x63hange_type\x18\x02 \x01(\x0e\x32\x17.eraftpb.ConfChangeType\x12\x0f\n\x07node_id\x18\x03 \x01(\x04\x12\x0f\n\x07\x63ontext\x18\x04 \x01(\x0c\x12\n\n\x02id\x18\x01 \x01(\x04"Q\n\x10\x43onfChangeSingle\x12,\n\x0b\x63hange_type\x18\x01 \x01(\x0e\x32\x17.eraftpb.ConfChangeType\x12\x0f\n\x07node_id\x18\x02 \x01(\x04"~\n\x0c\x43onfChangeV2\x12\x31\n\ntransition\x18\x01 \x01(\x0e\x32\x1d.eraftpb.ConfChangeTransition\x12*\n\x07\x63hanges\x18\x02 \x03(\x0b\x32\x19.eraftpb.ConfChangeSingle\x12\x0f\n\x07\x63ontext\x18\x03 \x01(\x0c*H\n\tEntryType\x12\x0f\n\x0b\x45ntryNormal\x10\x00\x12\x13\n\x0f\x45ntryConfChange\x10\x01\x12\x15\n\x11\x45ntryConfChangeV2\x10\x02*\x8c\x03\n\x0bMessageType\x12\n\n\x06MsgHup\x10\x00\x12\x0b\n\x07MsgBeat\x10\x01\x12\x0e\n\nMsgPropose\x10\x02\x12\r\n\tMsgAppend\x10\x03\x12\x15\n\x11MsgAppendResponse\x10\x04\x12\x12\n\x0eMsgRequestVote\x10\x05\x12\x1a\n\x16MsgRequestVoteResponse\x10\x06\x12\x0f\n\x0bMsgSnapshot\x10\x07\x12\x10\n\x0cMsgHeartbeat\x10\x08\x12\x18\n\x14MsgHeartbeatResponse\x10\t\x12\x12\n\x0eMsgUnreachable\x10\n\x12\x11\n\rMsgSnapStatus\x10\x0b\x12\x12\n\x0eMsgCheckQuorum\x10\x0c\x12\x15\n\x11MsgTransferLeader\x10\r\x12\x11\n\rMsgTimeoutNow\x10\x0e\x12\x10\n\x0cMsgReadIndex\x10\x0f\x12\x14\n\x10MsgReadIndexResp\x10\x10\x12\x15\n\x11MsgRequestPreVote\x10\x11\x12\x1d\n\x19MsgRequestPreVoteResponse\x10\x12*<\n\x14\x43onfChangeTransition\x12\x08\n\x04\x41uto\x10\x00\x12\x0c\n\x08Implicit\x10\x01\x12\x0c\n\x08\x45xplicit\x10\x02*A\n\x0e\x43onfChangeType\x12\x0b\n\x07\x41\x64\x64Node\x10\x00\x12\x0e\n\nRemoveNode\x10\x01\x12\x12\n\x0e\x41\x64\x64LearnerNode\x10\x02\x62\x06proto3'
)

_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, "eraftpb_pb2", globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    _ENTRYTYPE._serialized_start = 1111
    _ENTRYTYPE._serialized_end = 1183
    _MESSAGETYPE._serialized_start = 1186
    _MESSAGETYPE._serialized_end = 1582
    _CONFCHANGETRANSITION._serialized_start = 1584
    _CONFCHANGETRANSITION._serialized_end = 1644
    _CONFCHANGETYPE._serialized_start = 1646
    _CONFCHANGETYPE._serialized_end = 1711
    _ENTRY._serialized_start = 26
    _ENTRY._serialized_end = 151
    _SNAPSHOTMETADATA._serialized_start = 153
    _SNAPSHOTMETADATA._serialized_end = 240
    _SNAPSHOT._serialized_start = 242
    _SNAPSHOT._serialized_end = 311
    _MESSAGE._serialized_start = 314
    _MESSAGE._serialized_end = 620
    _HARDSTATE._serialized_start = 622
    _HARDSTATE._serialized_end = 677
    _CONFSTATE._serialized_start = 679
    _CONFSTATE._serialized_end = 792
    _CONFCHANGE._serialized_start = 794
    _CONFCHANGE._serialized_end = 898
    _CONFCHANGESINGLE._serialized_start = 900
    _CONFCHANGESINGLE._serialized_end = 981
    _CONFCHANGEV2._serialized_start = 983
    _CONFCHANGEV2._serialized_end = 1109
# @@protoc_insertion_point(module_scope)