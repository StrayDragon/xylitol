use crate::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "react-terminates"
)]
async fn test_ar_react_terminates(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "stream-is-xyevent"
)]
async fn test_ar_stream_is_xyevent(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "continues-after-tools"
)]
async fn test_ar_continues_after_tools(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "intent-before-execution"
)]
async fn test_ar_intent_before_execution(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-drops-sse"
)]
async fn test_ar_abort_drops_sse(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "steer-before-model"
)]
async fn test_ar_steer_before_model(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "followup-extends"
)]
async fn test_ar_followup_extends(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "queue-update"
)]
async fn test_ar_queue_update(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-clears-steer"
)]
async fn test_ar_abort_clears_steer(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "second-run-after-abort"
)]
async fn test_ar_second_run_after_abort(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-cancels-bang"
)]
async fn test_ar_abort_cancels_bang(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "before-denies"
)]
async fn test_ar_before_denies(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "should-stop-emits-agent-end"
)]
async fn test_ar_should_stop_emits_agent_end(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "should-stop-skips-followup"
)]
async fn test_ar_should_stop_skips_followup(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "no-hook-open-end"
)]
async fn test_ar_no_hook_open_end(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "max-turns-stops-run"
)]
async fn test_ar_max_turns_stops_run(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-default-barrier-parallel"
)]
async fn test_ar_batch_default_barrier_parallel(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-barrier-parallel-overlap"
)]
async fn test_ar_batch_barrier_parallel_overlap(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-barrier-preserves-source-windows"
)]
async fn test_ar_batch_barrier_preserves_windows(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-mcp-never-parallel"
)]
async fn test_ar_batch_mcp_never_parallel(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "batch-history-source-order"
)]
async fn test_ar_batch_history_source_order(agent: AgentState, ws: Workspace) {}
