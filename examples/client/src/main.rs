#[macro_use]
extern crate slog;

use raftify::{create_client, raft_service};

// A simple set of commands to test and show usage of RaftClient.
// Please bootstrap the Raft cluster before running this script.
#[actix_rt::main]
async fn main() {
    println!("---Message propose---");
    let mut leader_client = create_client(&"127.0.0.1:60061").await.unwrap();
    leader_client.propose(raft_service::ProposeArgs { msg: "test".to_owned().into() }).await.unwrap();

    // println!("---Debug node result---");
    // let result = leader_client.debug_node(raft_service::Empty {}).await.unwrap().into_inner().result;
    // println!("Debug node result: {:?}", result);
}
