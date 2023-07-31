import logging
from asyncio import Queue
from typing import Coroutine, List, Optional

import grpc

from raftify.protos import raft_service_pb2_grpc
from raftify.raft_service import RaftService
from raftify.utils import SocketAddr


class RaftServer:
    def __init__(
        self,
        addr: SocketAddr,
        sender: Queue,
        *,
        credentials: Optional[grpc.ServerCredentials] = None,
        cleanup_coroutines: Optional[List[Coroutine]] = None,
    ):
        self.addr = addr
        self.sender = sender
        self.credentials = credentials
        self.cleanup_coroutines = cleanup_coroutines

    async def run(self) -> None:
        grpc_server = grpc.aio.server()

        if self.credentials:
            grpc_server.add_secure_port(str(self.addr), self.credentials)
        else:
            grpc_server.add_insecure_port(str(self.addr))

        logging.info(f'Start listening gRPC requests on "{self.addr}"...')

        raft_service_pb2_grpc.add_RaftServiceServicer_to_server(
            RaftService(self.sender), grpc_server
        )

        await grpc_server.start()
        await grpc_server.wait_for_termination()

        async def server_graceful_shutdown():
            await grpc_server.stop(5)

        if self.cleanup_coroutines is not None:
            self.cleanup_coroutines.insert(0, server_graceful_shutdown())

        logging.warning("gRPC server has been terminated.")
