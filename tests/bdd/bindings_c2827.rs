//! Bindings for c2827 specs-compact wave 2.

use crate::tests::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::tests::bdd::steps_c2827::{C2827Bdd, c2827_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

// ── layer-architecture ───────────────────────────────────────────
// r1511 reload 双端（复用既有 host 步骤）

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "reload-dual-end-effect"
)]
fn test_la_reload_dual(host_pump_bdd: HostPumpBdd) {}

// r1512 print/嵌入同进程无需监听器
#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "embed-print-no-listener"
)]
fn test_la_embed_print(host_pump_bdd: HostPumpBdd) {}

// ── app-tui-host ─────────────────────────────────────────────────

// r1248 teardown
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "terminal-restore-on-exit"
)]
fn test_ath_exit_restore(host_pump_bdd: HostPumpBdd) {}

// r1270 min-size
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "min-size-friendly-hint"
)]
fn test_ath_min_size(host_pump_bdd: HostPumpBdd) {}

// r1279 bang 单扇入环（Esc 中止）
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "bang-esc-single-loop"
)]
fn test_ath_bang_esc(host_pump_bdd: HostPumpBdd) {}

// r1279 bang 中 agent 流不被饿死
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "bang-polls-agent-stream"
)]
fn test_ath_bang_polls(host_pump_bdd: HostPumpBdd) {}

// r1244 GetMessages 失败显错
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "get-messages-failure-surfaced"
)]
fn test_ath_get_messages_error(host_pump_bdd: HostPumpBdd) {}

// r1247 删除当前会话拒绝
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "resume-delete-current-refused"
)]
fn test_ath_delete_refused(host_pump_bdd: HostPumpBdd) {}

// r1247 删除须确认
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "resume-delete-requires-confirm"
)]
fn test_ath_delete_confirm(host_pump_bdd: HostPumpBdd) {}

// r1253 idle Tick 不重绘
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "idle-tick-no-full-repaint"
)]
fn test_ath_idle_tick(c2827_bdd: C2827Bdd) {}

// r1258 Mouse Moved 不重绘
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "mouse-moved-no-repaint"
)]
fn test_ath_mouse_moved(c2827_bdd: C2827Bdd) {}

// r1262 空输入 Enter
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "empty-enter-no-submit"
)]
fn test_ath_empty_enter(host_pump_bdd: HostPumpBdd) {}

// ── app-tui-session-tree ─────────────────────────────────────────

// r1326 树内 fork
#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "product-tree-fork-creates-child"
)]
fn test_st_tree_fork(host_pump_bdd: HostPumpBdd) {}

// r1327 Search/TreeHelp 行
#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "tree-slot-search-help-lines"
)]
fn test_st_search_help(host_pump_bdd: HostPumpBdd) {}

// r1328 标签写入
#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "tree-label-persist-via-driver"
)]
fn test_st_label(host_pump_bdd: HostPumpBdd) {}

// r1329 debug 树 fixture
#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "debug-tree-fixture-nonempty"
)]
fn test_st_debug_fixture(host_pump_bdd: HostPumpBdd) {}

// ── app-tui-input ────────────────────────────────────────────────

// r1298 /model 补全应用
#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "model-arg-popup-and-apply"
)]
fn test_ati_model_apply(host_pump_bdd: HostPumpBdd) {}

// r1298 Esc 不换模
#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "model-arg-esc-no-setmodel"
)]
fn test_ati_model_esc(host_pump_bdd: HostPumpBdd) {}

// r1309 @ 路径补全
#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "at-path-popup-and-apply"
)]
fn test_ati_at_popup(host_pump_bdd: HostPumpBdd) {}

// r1310 粘贴展开提交
#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "paste-marker-expanded-on-submit"
)]
fn test_ati_paste_expand(host_pump_bdd: HostPumpBdd) {}

// r1315 $ skill 补全
#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "dollar-skill-popup-apply"
)]
fn test_ati_dollar_popup(host_pump_bdd: HostPumpBdd) {}

// ── app-tui-transcript ───────────────────────────────────────────

// r1347 超大 diff 截断
#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "oversize-diff-truncated-with-omitted"
)]
fn test_at_diff_cap(host_pump_bdd: HostPumpBdd) {}

// ═══════════════════════════════════════════════════════════════════
// c2827 T2：agent 域
// ═══════════════════════════════════════════════════════════════════

use crate::tests::bdd::fixtures::XySessionStore;
use crate::tests::bdd::fixtures::{AgentState, Workspace, agent, sess, ws};
use crate::tests::bdd::steps_bridge::{PromptBdd, prompt_bdd};
use crate::tests::bdd::steps_c2827::{
    T2CapsBdd, T2EstBdd, T2SchemaBdd, T2SkillBdd, t2_caps_bdd, t2_est_bdd, t2_schema_bdd,
    t2_skill_bdd,
};

// agent-prompt r1017
#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "available-tools-omits-mcp-names"
)]
fn test_ap_mcp_omitted(prompt_bdd: PromptBdd) {}

// agent-prompt r1023
#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "apply-prompt-resources-next-run"
)]
fn test_ap_apply_resources(agent: AgentState, t2_caps_bdd: T2CapsBdd) {}

// agent-prompt r1024
#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "apply-skills-rebuilds-catalog"
)]
fn test_ap_apply_skills(agent: AgentState, t2_caps_bdd: T2CapsBdd) {}

// agent-prompt r1025
#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "dollar-skill-expands-body"
)]
fn test_ap_dollar_expand(t2_skill_bdd: T2SkillBdd) {}

// agent-tools r1149
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "search-tools-expose-optional-timeout"
)]
fn test_at_search_timeout(ws: Workspace, t2_schema_bdd: T2SchemaBdd) {}

// agent-tools r1153
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "fs-tools-hide-per-call-timeout"
)]
fn test_at_fs_no_timeout(ws: Workspace, t2_schema_bdd: T2SchemaBdd) {}

// accounting r1561
#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "estimate-priority-api-first"
)]
fn test_acc_api_first(t2_est_bdd: T2EstBdd) {}

// accounting r1564
#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "aborted-usage-not-anchor"
)]
fn test_acc_aborted_anchor(t2_est_bdd: T2EstBdd) {}

use crate::tests::bdd::steps_c2827::{T2TodoBdd, t2_todo_bdd};
use crate::tests::bdd::steps_otel_obs::{OtelBdd, otel_bdd};

// agent-todo r1125
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-tools-write-and-reject"
)]
async fn test_td_write_reject(t2_todo_bdd: T2TodoBdd) {}

// agent-todo r1126
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "multiple-in-progress-accepted"
)]
async fn test_td_multi_in_progress(t2_todo_bdd: T2TodoBdd) {}

// agent-todo r1842
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-content-over-80-rejected"
)]
async fn test_td_overlong(t2_todo_bdd: T2TodoBdd) {}

// agent-todo r1123
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-snapshot-latest-wins"
)]
async fn test_td_latest_wins(t2_todo_bdd: T2TodoBdd) {}

// agent-todo r1119
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "compaction-reappends-todo-snapshot"
)]
async fn test_td_compact_ensure(t2_todo_bdd: T2TodoBdd) {}

// agent-todo r1121
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "export-includes-agent-todo"
)]
async fn test_td_export(t2_todo_bdd: T2TodoBdd) {}

use crate::tests::bdd::steps_c2827::{T2ResBdd, t2_res_bdd};

// runtime-resource-discovery r1759/r1760/r1762/r1761/r1763/r1764/r1765
#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "resource-list-skills-themes-no-prompts"
)]
fn test_rd_list(agent: AgentState, ws: Workspace, prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "context-hot-reload-trust-gated"
)]
async fn test_rd_context_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "skills-hot-reload-trust-gated"
)]
async fn test_rd_skills_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "themes-listed-by-stem"
)]
fn test_rd_themes(t2_res_bdd: T2ResBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "resource-info-missing-nonzero"
)]
fn test_rd_info_missing(t2_res_bdd: T2ResBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "doctor-fails-on-diagnostics"
)]
fn test_rd_doctor_fail(t2_res_bdd: T2ResBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "resource-commands-readonly"
)]
fn test_rd_readonly(t2_res_bdd: T2ResBdd) {}

// infra-bash r1430/r1432/r1433/r1427/r1434
#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "bang-abort-cancels-executor"
)]
async fn test_ib_abort(agent: AgentState, ws: Workspace, host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "bang-execution-recorded-as-message"
)]
async fn test_ib_record(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "idle-bang-routes-to-executor"
)]
async fn test_ib_route(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "bash-before-hook-rejects"
)]
async fn test_ib_hook_reject(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "excluded-bash-not-in-context"
)]
fn test_ib_exclude() {}

// infra-provider r1504
#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "input-items-carry-type"
)]
fn test_ip_items_typed(t2_schema_bdd: T2SchemaBdd) {}

// infra-observability r1463
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "trace-gate-off-emits-nothing"
)]
#[serial]
async fn test_io_gate_off(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

// infra-mcp r1445
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "no-mcp-config-no-tools"
)]
fn test_mcp_none() {}

// infra-otel r1470
#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "idle-settlement-independent-root"
)]
#[serial]
fn test_ot_idle_root(otel_bdd: OtelBdd) {}

use crate::tests::bdd::steps_c2827::{T4PolicyBdd, T4PrintBdd, t4_policy_bdd, t4_print_bdd};

// domain-compaction r1411
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "force-instructions-append-focus"
)]
fn test_dc_focus(ws: Workspace, agent: AgentState) {}

// domain-compaction r1416
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "policy-snapshot-recorded"
)]
async fn test_dc_policy(
    ws: Workspace,
    agent: AgentState,
    sess: XySessionStore,
    t4_policy_bdd: T4PolicyBdd,
) {
}

// protocol-app r1693
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "command-roundtrip-fidelity"
)]
fn test_pa_command_roundtrip() {}

// cli-print r34/r43/r52/r55
#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print-streams-delta-only"
)]
async fn test_cp_delta(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print-shows-tool-name-and-summary"
)]
async fn test_cp_tool(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print-no-accumulated-payload"
)]
async fn test_cp_dedup(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print-error-nonzero-tool-fail-zero"
)]
async fn test_cp_error(t4_print_bdd: T4PrintBdd) {}

// user-experience r1838-r1841
#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "login-guidance-references-docs"
)]
fn test_ux_login() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "no-models-guidance"
)]
fn test_ux_no_models() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "unset-model-display"
)]
fn test_ux_unset() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "no-api-key-names-provider"
)]
fn test_ux_no_key() {}

// package-tui-interaction-modes r1628/r1631
#[scenario(
    path = "llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "drag-select-copies-transcript"
)]
fn test_pt_drag_copy() {}

#[scenario(
    path = "llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "dock-row-excluded-from-selection"
)]
fn test_pt_dock_exclude() {}

// ═══════════════════════════════════════════════════════════════════
// c2829 批 2
// ═══════════════════════════════════════════════════════════════════

use crate::tests::bdd::steps_c2827::{T6McpBdd, t6_mcp_bdd};

// agent-todo r1122
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-update-emits-typed-event"
)]
async fn test_td_typed_event(agent: AgentState) {}

// infra-mcp r1447
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "fixture-server-assembles-xytools"
)]
async fn test_mcp_fixture(t6_mcp_bdd: T6McpBdd) {}

// infra-mcp r1448
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "toolset-follows-config"
)]
async fn test_mcp_config_driven(t6_mcp_bdd: T6McpBdd) {}

// infra-mcp r1449
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "invalid-entry-diagnosed-not-fatal"
)]
async fn test_mcp_invalid(t6_mcp_bdd: T6McpBdd) {}
