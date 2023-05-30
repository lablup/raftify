import eraftpb_pb2 as _eraftpb_pb2
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor
Error: ResultCode
Ok: ResultCode
WrongLeader: ResultCode

class Empty(_message.Message):
    __slots__ = []
    def __init__(self) -> None: ...

class Entry(_message.Message):
    __slots__ = ["key", "value"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: int
    value: str
    def __init__(
        self, key: _Optional[int] = ..., value: _Optional[str] = ...
    ) -> None: ...

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

class Proposal(_message.Message):
    __slots__ = ["inner"]
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...

class RaftResponse(_message.Message):
    __slots__ = ["inner"]
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...

class ResultCode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
