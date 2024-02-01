use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::raft::eraftpb::ConfChangeType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InitialRole {
    Leader,
    Voter,
    Learner,
}

impl InitialRole {
    pub fn to_confchange_type(&self) -> ConfChangeType {
        match self {
            InitialRole::Voter => ConfChangeType::AddNode,
            InitialRole::Learner => ConfChangeType::AddLearnerNode,
            _ => panic!("Invalid follower role"),
        }
    }
}

impl fmt::Display for InitialRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InitialRole::Leader => write!(f, "Leader"),
            InitialRole::Voter => write!(f, "Voter"),
            InitialRole::Learner => write!(f, "Learner"),
        }
    }
}

impl FromStr for InitialRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Leader" => Ok(InitialRole::Leader),
            "Voter" => Ok(InitialRole::Voter),
            "Learner" => Ok(InitialRole::Learner),
            _ => Err(()),
        }
    }
}