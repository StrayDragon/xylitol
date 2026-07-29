//! Optional parent [`SpanContext`] for nesting llm.request / token.estimate /
//! agent.compaction (c1495 / c1700).
//!
//! Process-local like [`super::obs_session`]: agent installs turn/iteration/
//! compaction contexts; bridge adapters read them without changing `XyModel` /
//! adapter signatures.
//!
//! **Performance**: callers MUST only write when `provider_trace_active()`; readers
//! in `ProviderRequestTrace::start` / token estimate already gate on that flag first
//! so the mutex is never taken on the inactive hot path.

use std::sync::{Mutex, OnceLock};

use fastrace::prelude::SpanContext;

#[derive(Clone, Copy, Debug, Default)]
struct ObsSpanParents {
    turn: Option<SpanContext>,
    iteration: Option<SpanContext>,
    /// Active `agent.compaction` — preferred over iteration for summarization LLM.
    compaction: Option<SpanContext>,
}

fn slot() -> &'static Mutex<ObsSpanParents> {
    static SLOT: OnceLock<Mutex<ObsSpanParents>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(ObsSpanParents::default()))
}

/// Install or clear the `agent.turn` parent context.
pub fn set_obs_turn_parent(ctx: Option<SpanContext>) {
    let mut g = slot().lock().expect("obs span parent mutex");
    g.turn = ctx;
    if ctx.is_none() {
        g.iteration = None;
        g.compaction = None;
    }
}

/// Install or clear the `agent.iteration` parent (child of turn when both set).
pub fn set_obs_iteration_parent(ctx: Option<SpanContext>) {
    slot().lock().expect("obs span parent mutex").iteration = ctx;
}

/// Install or clear the `agent.compaction` parent (child of turn when both set).
pub fn set_obs_compaction_parent(ctx: Option<SpanContext>) {
    slot().lock().expect("obs span parent mutex").compaction = ctx;
}

pub fn clear_obs_span_parents() {
    *slot().lock().expect("obs span parent mutex") = ObsSpanParents::default();
}

/// Parent for `llm.request` / tools: prefer compaction, else iteration, else turn.
#[inline]
pub fn obs_llm_parent() -> Option<SpanContext> {
    let g = slot().lock().expect("obs span parent mutex");
    g.compaction.or(g.iteration).or(g.turn)
}

/// Parent for `token.estimate` / `agent.compaction`: turn only (never invent under a stale iteration).
#[inline]
pub fn obs_turn_parent() -> Option<SpanContext> {
    slot().lock().expect("obs span parent mutex").turn
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastrace::prelude::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn llm_parent_prefers_iteration() {
        let _g = TEST_LOCK.lock().unwrap();
        clear_obs_span_parents();
        let turn = Span::root("t", SpanContext::random());
        let turn_ctx = SpanContext::from_span(&turn).unwrap();
        let iter = Span::enter_with_parent("i", &turn);
        let iter_ctx = SpanContext::from_span(&iter).unwrap();
        set_obs_turn_parent(Some(turn_ctx));
        set_obs_iteration_parent(Some(iter_ctx));
        assert_eq!(obs_llm_parent(), Some(iter_ctx));
        assert_eq!(obs_turn_parent(), Some(turn_ctx));
        let compact = Span::enter_with_parent("c", &turn);
        let compact_ctx = SpanContext::from_span(&compact).unwrap();
        set_obs_compaction_parent(Some(compact_ctx));
        assert_eq!(obs_llm_parent(), Some(compact_ctx));
        set_obs_compaction_parent(None);
        assert_eq!(obs_llm_parent(), Some(iter_ctx));
        set_obs_iteration_parent(None);
        assert_eq!(obs_llm_parent(), Some(turn_ctx));
        clear_obs_span_parents();
    }
}
