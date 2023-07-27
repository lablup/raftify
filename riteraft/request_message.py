from dataclasses import dataclass
from multiprocessing import Queue

from riteraft.protos.eraftpb_pb2 import ConfChange, Message
from riteraft.utils import PickleSerializer


@dataclass
class MessagePropose(PickleSerializer):
    data: bytes
    chan: Queue


@dataclass
class MessageRerouteToLeader(PickleSerializer):
    data: bytes
    chan: Queue


@dataclass
class MessageConfigChange(PickleSerializer):
    change: ConfChange
    chan: Queue


@dataclass
class MessageRequestId(PickleSerializer):
    addr: str
    chan: Queue


@dataclass
class MessageReportUnreachable(PickleSerializer):
    node_id: int


@dataclass
class MessageRaft(PickleSerializer):
    msg: Message
