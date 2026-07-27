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
    name = "threshold-shares-footer-estimate"
)]
fn test_comp_threshold(agent: AgentState, ws: Workspace) {}

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
