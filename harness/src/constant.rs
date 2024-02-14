use const_format::concatcp;

pub const CLUSTER_INFO_PATH: &str = "./fixtures";

pub const LOOPBACK: &str = "127.0.0.1";

pub const RAFT_PORTS: [u16; 5] = [60061, 60062, 60063, 60064, 60065];

pub const RAFT_ADDRS: [&str; 5] = [
    concatcp!(LOOPBACK, ":", RAFT_PORTS[0]),
    concatcp!(LOOPBACK, ":", RAFT_PORTS[1]),
    concatcp!(LOOPBACK, ":", RAFT_PORTS[2]),
    concatcp!(LOOPBACK, ":", RAFT_PORTS[3]),
    concatcp!(LOOPBACK, ":", RAFT_PORTS[4]),
];

pub const ZERO_NODE_EXAMPLE: &str = "0-node-example.toml";
pub const ONE_NODE_EXAMPLE: &str = "1-node-example.toml";
pub const THREE_NODE_EXAMPLE: &str = "3-node-example.toml";
pub const FIVE_NODE_EXAMPLE: &str = "5-node-example.toml";
