use raft::eraftpb::ConfChangeType;

pub enum FollowerRole {
    Voter,
    Learner,
}

impl FollowerRole {
    pub fn to_confchange_type(&self) -> ConfChangeType {
        match self {
            FollowerRole::Voter => ConfChangeType::AddNode,
            FollowerRole::Learner => ConfChangeType::AddLearnerNode,
        }
    }
}
