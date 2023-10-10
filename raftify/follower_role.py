from enum import Enum

from rraft import ConfChangeType


class FollowerRole(Enum):
    Voter = ConfChangeType.AddNode
    Learner = ConfChangeType.AddLearnerNode

    def to_confchange_type(self) -> ConfChangeType:
        match self.value:
            case ConfChangeType.AddNode:
                return ConfChangeType.AddNode
            case ConfChangeType.AddLearnerNode:
                return ConfChangeType.AddLearnerNode
            case _:
                assert False, "FollowerRole should be Voter or Learner."
