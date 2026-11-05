#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StageNamePolicy {
    #[default]
    FirstTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageNameContext {
    pub attempted: bool,
    pub has_name: bool,
    pub aborted: bool,
    pub completed_turns: u64,
}

impl StageNamePolicy {
    pub fn should_request(self, ctx: StageNameContext) -> bool {
        match self {
            Self::FirstTurn => !ctx.attempted && !ctx.has_name && !ctx.aborted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(attempted: bool, has_name: bool, aborted: bool) -> StageNameContext {
        StageNameContext {
            attempted,
            has_name,
            aborted,
            completed_turns: 1,
        }
    }

    #[test]
    fn first_turn_requests_only_once() {
        let policy = StageNamePolicy::FirstTurn;
        assert!(policy.should_request(ctx(false, false, false)));
        assert!(!policy.should_request(ctx(true, false, false)));
        assert!(!policy.should_request(ctx(false, true, false)));
        assert!(!policy.should_request(ctx(false, false, true)));
    }
}
