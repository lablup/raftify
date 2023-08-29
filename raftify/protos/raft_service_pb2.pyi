from typing import ClassVar as _ClassVar
from typing import Mapping as _Mapping
from typing import Optional as _Optional
from typing import Union as _Union

import eraftpb_pb2 as _eraftpb_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper

DESCRIPTOR: _descriptor.FileDescriptor

class ChangeConfigResult(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    ChangeConfig_Success: _ClassVar[ChangeConfigResult]
    ChangeConfig_WrongLeader: _ClassVar[ChangeConfigResult]
    ChangeConfig_TimeoutError: _ClassVar[ChangeConfigResult]
    ChangeConfig_GrpcError: _ClassVar[ChangeConfigResult]
    ChangeConfig_UnknownError: _ClassVar[ChangeConfigResult]

class IdRequestResult(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    IdRequest_Success: _ClassVar[IdRequestResult]
    IdRequest_Error: _ClassVar[IdRequestResult]
    IdRequest_WrongLeader: _ClassVar[IdRequestResult]

class RerouteMsgType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    ConfChange: _ClassVar[RerouteMsgType]
    Propose: _ClassVar[RerouteMsgType]

ChangeConfig_Success: ChangeConfigResult
ChangeConfig_WrongLeader: ChangeConfigResult
ChangeConfig_TimeoutError: ChangeConfigResult
ChangeConfig_GrpcError: ChangeConfigResult
ChangeConfig_UnknownError: ChangeConfigResult
IdRequest_Success: IdRequestResult
IdRequest_Error: IdRequestResult
IdRequest_WrongLeader: IdRequestResult
ConfChange: RerouteMsgType
Propose: RerouteMsgType

class Proposal(_message.Message):
    __slots__ = ["inner"]
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...

class Entry(_message.Message):
    __slots__ = ["key", "value"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: int
    value: str
    def __init__(
        self, key: _Optional[int] = ..., value: _Optional[str] = ...
    ) -> None: ...

class RaftMessageResponse(_message.Message):
    __slots__ = ["data"]
    DATA_FIELD_NUMBER: _ClassVar[int]
    data: bytes
    def __init__(self, data: _Optional[bytes] = ...) -> None: ...

class ChangeConfigResponse(_message.Message):
    __slots__ = ["result", "data"]
    RESULT_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    result: ChangeConfigResult
    data: bytes
    def __init__(
        self,
        result: _Optional[_Union[ChangeConfigResult, str]] = ...,
        data: _Optional[bytes] = ...,
    ) -> None: ...

class IdRequestResponse(_message.Message):
    __slots__ = ["result", "data"]
    RESULT_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    result: IdRequestResult
    data: bytes
    def __init__(
        self,
        result: _Optional[_Union[IdRequestResult, str]] = ...,
        data: _Optional[bytes] = ...,
    ) -> None: ...

class IdRequestArgs(_message.Message):
    __slots__ = ["addr"]
    ADDR_FIELD_NUMBER: _ClassVar[int]
    addr: str
    def __init__(self, addr: _Optional[str] = ...) -> None: ...

class RerouteMessageArgs(_message.Message):
    __slots__ = ["proposed_data", "conf_change", "type"]
    PROPOSED_DATA_FIELD_NUMBER: _ClassVar[int]
    CONF_CHANGE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    proposed_data: bytes
    conf_change: _eraftpb_pb2.ConfChange
    type: RerouteMsgType
    def __init__(
        self,
        proposed_data: _Optional[bytes] = ...,
        conf_change: _Optional[_Union[_eraftpb_pb2.ConfChange, _Mapping]] = ...,
        type: _Optional[_Union[RerouteMsgType, str]] = ...,
    ) -> None: ...
