import asyncio
import pickle

from rraft import ConfChange, ConfChangeType

from raftify.log_entry.set_command import SetCommand
from raftify.raft_client import RaftClient
from raftify.utils import SocketAddr


async def main() -> None:
    """
    A simple set of commands to test and show usage of RaftClient.
    Please bootstrap the Raft cluster before running this script.
    """

    print("---Message propose---")
    await RaftClient("127.0.0.1:60061").propose(SetCommand("1", "A").encode())

    print("---Message propose rerouting---")
    await RaftClient("127.0.0.1:60062").propose(SetCommand("2", "A").encode())

    print("---Debug node result---", await RaftClient("127.0.0.1:60061").debug_node())

    print(
        "---Debug peers---",
        pickle.loads((await RaftClient("127.0.0.1:60061").get_peers()).peers),
    )

    print("---Make Confchange manually---")
    addr = SocketAddr.from_str("127.0.0.1:60062")

    conf_change = ConfChange.default()
    conf_change.set_node_id(2)
    conf_change.set_context(pickle.dumps([addr]))
    conf_change.set_change_type(ConfChangeType.RemoveNode)

    await RaftClient(addr).change_config(conf_change)


if __name__ == "__main__":
    asyncio.run(main())
