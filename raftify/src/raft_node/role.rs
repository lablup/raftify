use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InitialRole {
    Leader,
    Voter,
    Learner,
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
        let s = s.to_lowercase();
        match s.as_str() {
            "leader" => Ok(InitialRole::Leader),
            "voter" => Ok(InitialRole::Voter),
            "learner" => Ok(InitialRole::Learner),
            _ => Err(()),
        }
    }
}
