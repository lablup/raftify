# type: ignore
# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: raft_service.proto
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
from google.protobuf.internal import builder as _builder

# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()


from . import eraftpb_pb2 as eraftpb__pb2

DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(
    b'\n\x12raft_service.proto\x12\x0braftservice\x1a\reraftpb.proto"/\n\x18MemberBootstrapReadyArgs\x12\x13\n\x0b\x66ollower_id\x18\x01 \x01(\x04"*\n\x19\x43lusterBootstrapReadyArgs\x12\r\n\x05peers\x18\x01 \x01(\x0c"#\n\x13RaftMessageResponse\x12\x0c\n\x04\x64\x61ta\x18\x01 \x01(\x0c"U\n\x14\x43hangeConfigResponse\x12/\n\x06result\x18\x01 \x01(\x0e\x32\x1f.raftservice.ChangeConfigResult\x12\x0c\n\x04\x64\x61ta\x18\x02 \x01(\x0c"\x1d\n\rIdRequestArgs\x12\x0c\n\x04\x61\x64\x64r\x18\x01 \x01(\t"O\n\x11IdRequestResponse\x12,\n\x06result\x18\x01 \x01(\x0e\x32\x1c.raftservice.IdRequestResult\x12\x0c\n\x04\x64\x61ta\x18\x02 \x01(\x0c"\x82\x01\n\x12RerouteMessageArgs\x12\x15\n\rproposed_data\x18\x01 \x01(\x0c\x12*\n\x0b\x63onf_change\x18\x02 \x01(\x0b\x32\x15.eraftpb.ConfChangeV2\x12)\n\x04type\x18\x03 \x01(\x0e\x32\x1b.raftservice.RerouteMsgType*\x8a\x01\n\x12\x43hangeConfigResult\x12\x18\n\x14\x43hangeConfig_Success\x10\x00\x12\x1c\n\x18\x43hangeConfig_WrongLeader\x10\x01\x12\x1d\n\x19\x43hangeConfig_TimeoutError\x10\x02\x12\x1d\n\x19\x43hangeConfig_UnknownError\x10\x03*X\n\x0fIdRequestResult\x12\x15\n\x11IdRequest_Success\x10\x00\x12\x13\n\x0fIdRequest_Error\x10\x01\x12\x19\n\x15IdRequest_WrongLeader\x10\x02*-\n\x0eRerouteMsgType\x12\x0e\n\nConfChange\x10\x00\x12\x0b\n\x07Propose\x10\x01\x32\x88\x04\n\x0bRaftService\x12\x61\n\x14MemberBootstrapReady\x12%.raftservice.MemberBootstrapReadyArgs\x1a .raftservice.RaftMessageResponse"\x00\x12\x63\n\x15\x43lusterBootstrapReady\x12&.raftservice.ClusterBootstrapReadyArgs\x1a .raftservice.RaftMessageResponse"\x00\x12I\n\tRequestId\x12\x1a.raftservice.IdRequestArgs\x1a\x1e.raftservice.IdRequestResponse"\x00\x12J\n\x0c\x43hangeConfig\x12\x15.eraftpb.ConfChangeV2\x1a!.raftservice.ChangeConfigResponse"\x00\x12\x43\n\x0bSendMessage\x12\x10.eraftpb.Message\x1a .raftservice.RaftMessageResponse"\x00\x12U\n\x0eRerouteMessage\x12\x1f.raftservice.RerouteMessageArgs\x1a .raftservice.RaftMessageResponse"\x00\x62\x06proto3'
)

_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, "raft_service_pb2", globals())
if _descriptor._USE_C_DESCRIPTORS == False:
    DESCRIPTOR._options = None
    _CHANGECONFIGRESULT._serialized_start = 513
    _CHANGECONFIGRESULT._serialized_end = 651
    _IDREQUESTRESULT._serialized_start = 653
    _IDREQUESTRESULT._serialized_end = 741
    _REROUTEMSGTYPE._serialized_start = 743
    _REROUTEMSGTYPE._serialized_end = 788
    _MEMBERBOOTSTRAPREADYARGS._serialized_start = 50
    _MEMBERBOOTSTRAPREADYARGS._serialized_end = 97
    _CLUSTERBOOTSTRAPREADYARGS._serialized_start = 99
    _CLUSTERBOOTSTRAPREADYARGS._serialized_end = 141
    _RAFTMESSAGERESPONSE._serialized_start = 143
    _RAFTMESSAGERESPONSE._serialized_end = 178
    _CHANGECONFIGRESPONSE._serialized_start = 180
    _CHANGECONFIGRESPONSE._serialized_end = 265
    _IDREQUESTARGS._serialized_start = 267
    _IDREQUESTARGS._serialized_end = 296
    _IDREQUESTRESPONSE._serialized_start = 298
    _IDREQUESTRESPONSE._serialized_end = 377
    _REROUTEMESSAGEARGS._serialized_start = 380
    _REROUTEMESSAGEARGS._serialized_end = 510
    _RAFTSERVICE._serialized_start = 791
    _RAFTSERVICE._serialized_end = 1311
# @@protoc_insertion_point(module_scope)
