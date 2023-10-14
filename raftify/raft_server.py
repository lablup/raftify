import asyncio
from asyncio import Queue, Task
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
        message_queue: Queue,
        logger: AbstractRaftifyLogger,
        *,
        credentials: Optional[grpc.ServerCredentials] = None,
    ):
        self.addr = addr
        self.message_queue = message_queue
        self.credentials = credentials
        self.logger = logger
        self.grpc_server = None
        self.server_task = None

    async def run(self) -> Task:
        self.grpc_server = grpc.aio.server()
        assert self.grpc_server is not None

        if self.credentials:
            self.grpc_server.add_secure_port(str(self.addr), self.credentials)
        else:
            self.grpc_server.add_insecure_port(str(self.addr))

        self.logger.debug(f'Start listening gRPC requests on "{self.addr}"...')

        raft_service_pb2_grpc.add_RaftServiceServicer_to_server(
            RaftService(self.message_queue, self.logger), self.grpc_server
        )

        await self.grpc_server.start()
        self.server_task = asyncio.create_task(self.grpc_server.wait_for_termination())
        return self.server_task

    async def terminate(self, grace: Optional[float] = None) -> None:
        assert self.grpc_server is not None, "gRPC server is not running."
        await self.grpc_server.stop(grace)
        self.logger.debug("gRPC server has been terminated.")
