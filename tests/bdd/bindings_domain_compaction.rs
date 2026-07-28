use crate::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "need-compact"
)]
fn test_compaction_need(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "no-compact"
)]
fn test_compaction_not_needed(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "disabled-no-compact"
)]
fn test_compaction_disabled(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "retain-recent"
)]
fn test_compaction_keep_recent(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "write-entry"
)]
fn test_compaction_write(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "branch-summary"
)]
fn test_compaction_branch(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "estimate-uses-priority"
)]
fn test_comp_est_priority(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "estimate-fallback-chain"
)]
fn test_comp_est_fallback(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "find-cut"
)]
fn test_comp_find_cut(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "provenance-available"
)]
fn test_comp_provenance(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "reserve-trigger-shares-footer-estimate"
)]
fn test_comp_reserve_trigger_shared(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "summarize"
)]
async fn test_comp_summarize_c3(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "generate-summary"
)]
async fn test_comp_generate_summary(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "agent"
)]
async fn test_comp_agent_c12(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "persist"
)]
async fn test_comp_persist(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "branch"
)]
fn test_comp_branch_c5(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "iterative"
)]
async fn test_comp_iterative(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "files"
)]
async fn test_comp_files(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "entry"
)]
async fn test_comp_entry(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "split-by-responsibility"
)]
async fn test_comp_split(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "auto-over-threshold"
)]
fn test_comp_auto_over(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "auto-under-threshold"
)]
fn test_comp_auto_under(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "auto-disabled-no-compact"
)]
fn test_comp_auto_disabled(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "manual-force-bypasses-reserve"
)]
fn test_comp_manual_force(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "stale-guard-after-compaction"
)]
fn test_comp_stale_guard(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "cut-assistant"
)]
fn test_comp_cut_assistant(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "never-tool-result"
)]
fn test_comp_never_tool(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "keep-budget"
)]
fn test_comp_keep_budget(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "split-dual-summary"
)]
async fn test_comp_split_dual(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "tokens-before"
)]
async fn test_comp_tokens_before(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
