#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StageNamePolicy {
    #[default]
    FirstTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageNamePhase {
    NotStarted,
    Requested,
    Named,
    Aborted,
}

impl StageNamePolicy {
    pub fn should_request(self, phase: StageNamePhase) -> bool {
        match self {
            Self::FirstTurn => phase == StageNamePhase::NotStarted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_turn_requests_only_once() {
        let policy = StageNamePolicy::FirstTurn;
        assert!(policy.should_request(StageNamePhase::NotStarted));
        assert!(!policy.should_request(StageNamePhase::Requested));
        assert!(!policy.should_request(StageNamePhase::Named));
        assert!(!policy.should_request(StageNamePhase::Aborted));
    }
}
