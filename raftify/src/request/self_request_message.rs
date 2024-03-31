/// Request type sent from a RaftNode to itself (RaftNode).
/// Used for accessing the RaftNode from a future created by RaftNode asynchronous methods
#[derive(Debug)]
pub enum SelfMessage {
    ReportUnreachable { node_id: u64 },
}
