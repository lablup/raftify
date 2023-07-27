from typing import ClassVar as _ClassVar
from typing import Optional as _Optional
from typing import Union as _Union

import eraftpb_pb2 as _eraftpb_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper

DESCRIPTOR: _descriptor.FileDescriptor

class ResultCode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    Ok: _ClassVar[ResultCode]
    Error: _ClassVar[ResultCode]
    WrongLeader: _ClassVar[ResultCode]

Ok: ResultCode
Error: ResultCode
WrongLeader: ResultCode

class Proposal(_message.Message):
    __slots__ = ["inner"]
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...

class IdRequestResponse(_message.Message):
    __slots__ = ["code", "data"]
    CODE_FIELD_NUMBER: _ClassVar[int]
    DATA_FIELD_NUMBER: _ClassVar[int]
    code: ResultCode
    data: bytes
    def __init__(
        self,
        code: _Optional[_Union[ResultCode, str]] = ...,
        data: _Optional[bytes] = ...,
    ) -> None: ...

class RequestIdArgs(_message.Message):
    __slots__ = ["addr"]
    ADDR_FIELD_NUMBER: _ClassVar[int]
    addr: str
    def __init__(self, addr: _Optional[str] = ...) -> None: ...

class Entry(_message.Message):
    __slots__ = ["key", "value"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: int
    value: str
    def __init__(
        self, key: _Optional[int] = ..., value: _Optional[str] = ...
    ) -> None: ...

class RaftResponse(_message.Message):
    __slots__ = ["inner"]
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...

class RerouteMessageArgs(_message.Message):
    __slots__ = ["inner"]
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...
