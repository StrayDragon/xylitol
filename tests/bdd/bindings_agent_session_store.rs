use crate::tests::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "create-load"
)]
async fn test_session_create_load(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "list-sessions"
)]
async fn test_session_list(ws: Workspace, sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "fork"
)]
async fn test_session_fork(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "tree-nav"
)]
async fn test_session_tree_nav(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "model-change"
)]
async fn test_session_model_change(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "thinking-change"
)]
async fn test_session_thinking_change(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "jsonl-format"
)]
async fn test_session_jsonl_format(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "label-set"
)]
async fn test_session_label_set(sess: XySessionStore) {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "label-clear"
)]
async fn test_session_label_clear(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "delayed-header-flush-on-first-append"
)]
async fn test_s12_delayed_flush(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "fork-copies-cutoff-and-summary"
)]
async fn test_s9_fork_copy(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "branch-summary-content"
)]
async fn test_s10_summary_content(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "fork-session-returns-child-id"
)]
async fn test_s11_fork_child_id(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "list-resilient-to-corrupt-file"
)]
async fn test_s21_list_resilient(ws: Workspace, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "list-skips-legacy-only-session"
)]
async fn test_s26_list_skips_legacy(ws: Workspace, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "timestamps-u64-ms"
)]
async fn test_s22_timestamps(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "export-jsonl"
)]
async fn test_ex2_export_jsonl(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "import-jsonl-roundtrip"
)]
async fn test_ex3_import_roundtrip(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "export-html"
)]
async fn test_ex1_export_html(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "html-readable-blocks"
)]
async fn test_ex4_html_blocks(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "resume-fallback-cwd-rescues"
)]
async fn test_sc1_fallback_rescues(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "stored-cwd-accessible-loads"
)]
async fn test_s16_stored_cwd_loads(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "cwd-error-carries-both-paths"
)]
async fn test_sc2_both_paths(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "tool-result-persists-tool-call-id-key"
)]
async fn test_s19_tool_call_id_key(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "branch-summary-empty-input-empty-output"
)]
async fn test_s6_empty_boundary(sess: XySessionStore) {}

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "v7-shell-camelcase-and-bash-message"
)]
async fn test_c2826_v7_shell(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "load-skips-unparseable-lines"
)]
async fn test_c2826_load_skips(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "legacy-untagged-content-rejected"
)]
async fn test_c2826_legacy_untagged(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "orphan-temp-segment-excluded"
)]
async fn test_c2826_orphan_excluded(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "unreferenced-segment-does-not-affect-list"
)]
async fn test_c2826_unref_segment_list(ws: Workspace, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "fork-header-carries-cut-entry"
)]
async fn test_c2826_fork_header(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "cold-segment-restore-leaf-correct"
)]
async fn test_c2826_cold_restore(agent: AgentState, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "sidecar-corrupt-falls-back-to-scan"
)]
async fn test_c2826_sidecar_corrupt(agent: AgentState, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "resume-projects-llm-api-verbatim"
)]
async fn test_c2826_resume_projection(sess: XySessionStore) {}
