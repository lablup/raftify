from asyncio import Queue
from typing import Optional

import grpc

from raftify.logger import AbstractRaftifyLogger
from raftify.protos import raft_service_pb2_grpc
from raftify.raft_service import RaftService
from raftify.utils import SocketAddr


class RaftServer:
    def __init__(
        self,
        addr: SocketAddr,
        sender: Queue,
        logger: AbstractRaftifyLogger,
        *,
        credentials: Optional[grpc.ServerCredentials] = None,
    ):
        self.addr = addr
        self.sender = sender
        self.credentials = credentials
        self.logger = logger

    async def run(self) -> None:
        grpc_server = grpc.aio.server()

        if self.credentials:
            grpc_server.add_secure_port(str(self.addr), self.credentials)
        else:
            grpc_server.add_insecure_port(str(self.addr))

        self.logger.info(f'Start listening gRPC requests on "{self.addr}"...')

        raft_service_pb2_grpc.add_RaftServiceServicer_to_server(
            RaftService(self.sender, self.logger), grpc_server
        )

        await grpc_server.start()
        await grpc_server.wait_for_termination()

        self.logger.warning("gRPC server has been terminated.")
