use crate::tests::bdd::fixtures::*;
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

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "first-turn-history-is-real-user"
)]
async fn test_c2826_first_turn_history(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "context-reload-trust-gated"
)]
async fn test_c2826_context_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "skills-reload-trust-gated-and-queryable"
)]
async fn test_c2826_skills_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "abort-persists-partial-with-aborted-reason"
)]
async fn test_c2826_abort_persist(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "tool-turn-two-iterations-one-settlement"
)]
async fn test_c2826_two_iterations(agent: AgentState, ws: Workspace) {}

// ── c2835 后继：agent-runtime 裸规则回填 ────────────────────────────
use crate::tests::bdd::steps_app_tui_bridge::{BridgeBdd, bridge_bdd};
use crate::tests::bdd::steps_app_tui_interaction::{TuiInteraction, tui_interaction};
use crate::tests::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use crate::tests::bdd::steps_bridge::{PromptBdd, prompt_bdd};
use crate::tests::bdd::steps_c2827::{
    T2CapsBdd, T2EstBdd, T6McpBdd, t2_caps_bdd, t2_est_bdd, t6_mcp_bdd,
};
use crate::tests::bdd::steps_protocol::{ProtocolBdd, protocol_bdd};
use crate::tests::bdd::steps_runtime_config::{RcSnap, rc_snap};

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "builder-ports-drive-one-round"
)]
async fn test_ar1_builder_ports(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "ports-yield-xy-events"
)]
async fn test_ar2_ports_yield_events(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "hot-applied-slots-rebuild-prompt"
)]
async fn test_ar3_hot_slots_next_turn(t2_caps_bdd: T2CapsBdd, agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "unknown-queue-type-degrades"
)]
async fn test_ar4_unknown_event_degrades(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "centralised-default-thinking-level"
)]
fn test_ar5_centralised_defaults(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "done-usage-anchor-persisted"
)]
async fn test_ar6_done_usage_persisted(t2_est_bdd: T2EstBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "next-turn-rereads-model-and-thinking"
)]
async fn test_ar7_next_turn_reresolve(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "turn-end-threshold-auto-compact"
)]
async fn test_ar8_turn_end_threshold(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "overflow-case1-compact-retry"
)]
async fn test_ar9_overflow_case1(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "code-first-context-policy-defaults"
)]
async fn test_ar10_context_policy_defaults(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "first-generate-freezes-tool-table"
)]
async fn test_ar11_mcp_freeze_gate(t6_mcp_bdd: T6McpBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "error-kind-survives-projection"
)]
async fn test_ar12_error_kind(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "persisted-node-timings-settle-thought"
)]
async fn test_ar13_stream_node_timings(
    transcript_bdd: TranscriptBdd,
    tui_interaction: TuiInteraction,
) {
}
