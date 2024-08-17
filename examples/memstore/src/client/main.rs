use raftify::{create_client, create_client_with_ssl, raft_service, AbstractLogEntry};

use memstore_example_harness::state_machine::LogEntry;

// A simple set of commands to test and show usage of RaftServiceClient.
// Please bootstrap the Raft cluster before call this function.
#[actix_rt::main]
async fn main() {
    println!("---Message propose---");
    let mut leader_client = create_client(&"127.0.0.1:60061").await.unwrap();
    let mut leader_client2 = create_client_with_ssl(&"127.0.0.1:60061").await.unwrap();

    leader_client
        .propose(raft_service::ProposeArgs {
            msg: LogEntry::Insert {
                key: 1,
                value: "test".to_string(),
            }
            .encode()
            .unwrap(),
        })
        .await
        .unwrap();

    println!("---Get peers---");
    let peers = leader_client
        .get_peers(raft_service::Empty {})
        .await
        .unwrap();

    println!("Peers: {:?}", peers.into_inner().peers_json);

    println!("---Debug node result---");
    let result = leader_client
        .debug_node(raft_service::Empty {})
        .await
        .unwrap()
        .into_inner()
        .result_json;

    let result2 = leader_client2
        .debug_node(raft_service::Empty {})
        .await
        .unwrap()
        .into_inner()
        .result_json;

    assert_eq!(result, result2);
    println!("Debug node result: {:?}", result);

    // println!("---Change config example---");
    // leader_client.change_config(raft_service::ChangeConfigArgs {
    //     addrs: vec![
    //         "127.0.0.1:60061".to_owned(),
    //         "127.0.0.1:60062".to_owned(),
    //         "127.0.0.1:60063".to_owned(),
    //         "127.0.0.1:60064".to_owned(),
    //         "127.0.0.1:60065".to_owned(),
    //     ],
    //     changes: vec![
    //         ConfChangeSingle {
    //             node_id: 1,
    //             change_type: ConfChangeType::AddNode as i32,
    //         },
    //         ConfChangeSingle {
    //             node_id: 2,
    //             change_type: ConfChangeType::AddNode as i32,
    //         },
    //         ConfChangeSingle {
    //             node_id: 3,
    //             change_type: ConfChangeType::AddNode as i32,
    //         },
    //         ConfChangeSingle {
    //             node_id: 4,
    //             change_type: ConfChangeType::AddNode as i32,
    //         },
    //         ConfChangeSingle {
    //             node_id: 5,
    //             change_type: ConfChangeType::AddNode as i32,
    //         },
    //     ],
    // }).await.expect("Change config failed!");
}
