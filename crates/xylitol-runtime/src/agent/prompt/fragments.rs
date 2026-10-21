use xylitol_plugin_sdk::XyBatchMode;

pub const FRAGMENT_TOOL_BATCH_BARRIER_PARALLEL: &str = "tool_batch.barrier_parallel";

const BODY_TOOL_BATCH_BARRIER_PARALLEL: &str = "\
Tool calling policy:
- When several independent read-only tools are needed (read/grep/find/ls), emit ALL of them in the SAME assistant message as multiple tool calls.
- Do NOT narrate parallel tool use while emitting only one tool call per turn.
- If you can only emit one tool call this turn, say so in one short sentence.
- Dependent writes/bash come after prior-turn read results.";

pub fn fragments_for_batch_mode(mode: XyBatchMode) -> Vec<&'static str> {
    match mode {
        XyBatchMode::BarrierParallel => vec![BODY_TOOL_BATCH_BARRIER_PARALLEL],
        XyBatchMode::Sequential => Vec::new(),
    }
}

pub fn fragment_ids_for_batch_mode(mode: XyBatchMode) -> Vec<&'static str> {
    match mode {
        XyBatchMode::BarrierParallel => vec![FRAGMENT_TOOL_BATCH_BARRIER_PARALLEL],
        XyBatchMode::Sequential => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrier_parallel_injects_one_fragment() {
        let bodies = fragments_for_batch_mode(XyBatchMode::BarrierParallel);
        let ids = fragment_ids_for_batch_mode(XyBatchMode::BarrierParallel);
        assert_eq!(ids, vec![FRAGMENT_TOOL_BATCH_BARRIER_PARALLEL]);
        assert_eq!(bodies.len(), 1);
        assert!(bodies[0].contains("SAME assistant message"));
    }

    #[test]
    fn sequential_injects_none() {
        assert!(fragments_for_batch_mode(XyBatchMode::Sequential).is_empty());
        assert!(fragment_ids_for_batch_mode(XyBatchMode::Sequential).is_empty());
    }
}
