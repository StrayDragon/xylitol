//! Bindings for c2827 specs-compact wave 2.

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use crate::bdd::steps_c2827::{C2827Bdd, c2827_bdd};
use crate::bdd::steps_package_tui_tree_selector::{TreeSelBdd, tree_sel_bdd};
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

use crate::bdd::fixtures::XySessionStore;
use crate::bdd::fixtures::{AgentState, Workspace, agent, sess, ws};
use crate::bdd::steps_bridge::{PromptBdd, prompt_bdd};
use crate::bdd::steps_c2827::{
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
    path = "packages/xylitol-ai-bridge/llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "estimate-priority-api-first"
)]
fn test_acc_api_first(t2_est_bdd: T2EstBdd) {}

// accounting r1564
#[scenario(
    path = "packages/xylitol-ai-bridge/llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "aborted-usage-not-anchor"
)]
fn test_acc_aborted_anchor(t2_est_bdd: T2EstBdd) {}

use crate::bdd::steps_c2827::{T2TodoBdd, t2_todo_bdd};
use crate::bdd::steps_otel_obs::{OtelBdd, otel_bdd};

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

use crate::bdd::steps_c2827::{T2ResBdd, t2_res_bdd};

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

use crate::bdd::steps_c2827::{T4PolicyBdd, T4PrintBdd, t4_policy_bdd, t4_print_bdd};

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
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "drag-select-copies-transcript"
)]
fn test_pt_drag_copy() {}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "dock-row-excluded-from-selection"
)]
fn test_pt_dock_exclude() {}

// ═══════════════════════════════════════════════════════════════════
// c2829 批 2
// ═══════════════════════════════════════════════════════════════════

use crate::bdd::steps_c2827::{T6McpBdd, t6_mcp_bdd};

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

// ── c2835 后继：infra-provider 裸规则回填 ──────────────────────────
use crate::bdd::steps_bridge::{AiBridgeBdd, ai_bridge_bdd};

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "single-layer-adapter-wrap"
)]
fn test_ip_r1497_single_wrap() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "responses-adapter-request-body"
)]
fn test_ip_r1498_responses_body(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "anthropic-messages-thinking-branch"
)]
fn test_ip_r1502_anthropic_branch(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "adapter-selection-by-kind-and-api"
)]
fn test_ip_r1503_selection() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "single-xy-model-assembly-path"
)]
fn test_ip_r1505_single_path() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "vendor-types-stay-in-bridge"
)]
fn test_ip_r1506_vendor_boundary() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "model-port-takes-bridge-dto"
)]
fn test_ip_r1499_bridge_dto() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "named-compat-profile-injection"
)]
fn test_ip_r1500_named_compat(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "api-literals-full-names"
)]
fn test_ip_r1501_full_names() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "token-sync-single-script"
)]
fn test_pg_r1206_token_sync() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "markdown-body-styled-no-literal-markers"
)]
fn test_pg_r1207_markdown_body(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "tui-agents-reading-order"
)]
fn test_pg_r1208_reading_order() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "kind-prefix-themed-not-baked-role"
)]
fn test_pg_r1209_kind_prefix(tree_sel_bdd: TreeSelBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "live-tree-from-command"
)]
async fn test_pg_r1210_live_tree(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "static-slot-samples-covered"
)]
fn test_pg_r1213_static_slots() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "designing-lint-in-qa"
)]
fn test_pg_r1211_lint_gate() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "tui-agents-name-lint-commands"
)]
fn test_pg_r1212_lint_docs() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "drag-select-copies-assistant-body"
)]
fn test_cb_r13_drag_copy() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "default-mode-inline"
)]
fn test_cb_r14_default_mode() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "copy-notice-observable"
)]
fn test_cb_r15_copy_notice() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "osc52-remote-fallback"
)]
fn test_cb_r16_osc52() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "empty-selection-silent"
)]
fn test_cb_r17_empty_selection() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "editor-copy-path-only"
)]
fn test_cb_r18_editor_path() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "image-paste-from-host"
)]
async fn test_cb_r19_image_paste(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "text-read-via-host-pump"
)]
async fn test_cb_r20_text_read(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "single-config-type-pair"
)]
async fn test_cp_r1400_single_type(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "reserve-shares-footer-estimate"
)]
async fn test_cp_r1402_shared_estimate(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "threshold-route-after-settle"
)]
async fn test_cp_r1410_threshold_route(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "cut-on-leaf-branch"
)]
async fn test_cp_r1412_leaf_cut(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "one-estimate-per-settlement"
)]
async fn test_cp_r1413_one_estimate(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "summary-sections-from-kept-branch"
)]
async fn test_cp_r1414_summary_sections(agent: AgentState, ws: Workspace, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "floor-diagnostic-at-most-once"
)]
async fn test_cp_r1415_once(agent: AgentState, ws: Workspace, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "qa-six-ordered-gates"
)]
fn test_qa_r1827_sequence() {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "qa-e2e-adds-fifth-layer"
)]
fn test_qa_r1828_layers() {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "docs-name-qa-split"
)]
fn test_qa_r1829_docs() {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "check-scripts-all-wired"
)]
fn test_qa_r1830_wired() {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "pty-session-tree-registered"
)]
fn test_qa_r1831_pty() {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "complexity-gate-ssot"
)]
fn test_qa_r1832_ssot() {}

#[scenario(
    path = "llmanspec/specs/test-qa-gate/test-qa-gate.feature",
    name = "live-gate-serial-with-timeout"
)]
fn test_qa_r1833_serial() {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "assembly-includes-tool-guidelines"
)]
fn test_ap_r1015_guidelines(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "runtime-policy-fragments-injected"
)]
fn test_ap_r1016_runtime_policy(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "date-and-cwd-from-session-env"
)]
fn test_ap_r1018_session_env() {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "system-prompt-single-copy"
)]
fn test_ap_r1019_single_copy(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "skills-listed-after-trust"
)]
fn test_ap_r1020_skills(agent: AgentState, t2_caps_bdd: T2CapsBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "replacement-skip-default-fragments"
)]
fn test_ap_r1022_replacement(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "yolo-toggle-after-trust"
)]
async fn test_ai_r1314_yolo(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "steer-kept-in-transcript"
)]
async fn test_ai_r1322_steer(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "bang-prefix-single-block"
)]
async fn test_ai_r1323_bang(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "external-editor-chord-recorded"
)]
async fn test_ai_r1324_editor(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "history-seed-from-previous-session"
)]
fn test_ai_r1316_seed(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "resume-panel-keys"
)]
async fn test_ai_r1304_resume(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "echo-via-executor"
)]
fn test_bs_r1429_echo(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "overflow-to-temp-file"
)]
fn test_bs_r1431_truncate(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "trait-abstraction-mockable"
)]
fn test_bs_r1426_trait(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "escalating-timeout-kills"
)]
fn test_bs_r1428_timeout(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "abort-reachable-from-runtime"
)]
async fn test_bs_r1435_abort(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "optional-timeout-and-missing-arg"
)]
fn test_bs_r1436_missing_arg(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "thinking-streams-to-stderr-only"
)]
fn test_cp_thinking_stderr(t4_print_bdd: T4PrintBdd) {}
#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "three-app-surfaces-in-default-feature-set"
)]
fn test_at_three_surfaces() {}
#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "tui-imports-via-public-contract-only"
)]
fn test_at_public_contract() {}
#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "index-points-to-per-capability-rules"
)]
fn test_at_capability_index() {}
#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "trust-choice-theme-and-cancel"
)]
fn test_atq_theme_cancel() {}
#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "trust-slash-persists-without-auto-reload"
)]
fn test_atq_slash_persist() {}
#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "semantic-colors-from-single-token-source"
)]
fn test_atz_token_source() {}
#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "designing-lint-is-a-gate"
)]
fn test_atz_design_lint_gate() {}
#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "surface-agents-points-to-written-docs"
)]
fn test_atz_docs_ssot() {}
#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "demo-defaults-dark-with-explicit-auto"
)]
fn test_atz_demo_theme() {}
#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "diff-reuses-package-component"
)]
fn test_att_diff_package() {}
#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "left-rail-reuses-package-painter"
)]
fn test_att_rail_package() {}

// infra-mcp r1450-1453/1446（批 2）
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "connected-list-readonly"
)]
async fn test_t2_mcp_connected(t6_mcp_bdd: T6McpBdd) {}
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "mcp-tool-hard-barrier-scheduling"
)]
fn test_t2_mcp_barrier() {}
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "nonblocking-ui-parallel-startup"
)]
fn test_t2_mcp_nonblocking() {}
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "loaded-resources-live-mcp-source"
)]
fn test_t2_mcp_loaded_src() {}
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "call-phase-timeout-classified"
)]
fn test_t2_mcp_timeout() {}
#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "first-turn-tool-freeze-gate"
)]
fn test_t2_mcp_first_turn() {}

// infra-image r1440-1444（批 2）
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "constrained-image-resize"
)]
fn test_t2_image_resize() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "exif-orientation-boundary"
)]
fn test_t2_image_exif() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "overlimit-image-format-convert"
)]
fn test_t2_image_convert() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "multimodal-payload-bounds"
)]
fn test_t2_image_multimodal() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "path-to-image-part"
)]
fn test_t2_image_part() {}

// infra-process r1493-1496（批 2）
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "bash-path-discovery"
)]
fn test_t2_proc_bash() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "shell-env-path-lookup"
)]
fn test_t2_proc_shell_env() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "kill-process-tree-reaps"
)]
fn test_t2_proc_kill_tree() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "child-wait-with-reap-guard"
)]
fn test_t2_proc_child_wait() {}

// infra-observability r1461/1464/1465（批 2）
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "compose-root-single-obs-sink"
)]
fn test_t2_obs_sink() {}
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "fastrace-single-stack"
)]
fn test_t2_obs_stack() {}
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "provider-trace-correlated-spans"
)]
fn test_t2_obs_spans() {}

// infra-diagnostics r1437-1439（批 2）
#[scenario(
    path = "llmanspec/specs/infra-diagnostics/infra-diagnostics.feature",
    name = "timing-collector-gated"
)]
fn test_t2_timing_collector() {}
#[scenario(
    path = "llmanspec/specs/infra-diagnostics/infra-diagnostics.feature",
    name = "timing-points-on-critical-path"
)]
fn test_t2_timing_sites() {}
#[scenario(
    path = "llmanspec/specs/infra-diagnostics/infra-diagnostics.feature",
    name = "timing-output-millis"
)]
fn test_t2_timing_output() {}

// agent-todo r1118-1128/1120（批 3）
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-strict-model-shape"
)]
fn test_t3_todo_model() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-snapshot-outside-prefix"
)]
fn test_t3_todo_snapshot() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-concurrency-barrier"
)]
fn test_t3_todo_barrier() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "status-bar-read-only-boundary"
)]
fn test_t3_todo_status_bar() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-builtins-first-turn-freeze"
)]
fn test_t3_todo_freeze() {}

// agent-session-store r41/1114-1117/1116（批 3）
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "compaction-trigger-on-reserve-exceed"
)]
fn test_t3_compaction_trigger() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "cwd-check-before-restore"
)]
fn test_t3_cwd_check() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "session-manager-implements-port"
)]
fn test_t3_session_port() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "journal-read-recent-boundary"
)]
fn test_t3_journal_recent() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "export-io-port-injected"
)]
fn test_t3_export_io() {}

// agent-session r1077/1080/1083/1084（批 3）
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "session-store-port-layered"
)]
fn test_t3_session_layers(ws: Workspace, agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "trust-outside-agent-layer"
)]
fn test_t3_trust_agent(ws: Workspace, agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "compaction-aware-session-context"
)]
fn test_t3_compaction_context(ws: Workspace, agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "resume-single-import-path"
)]
fn test_t3_resume_path(ws: Workspace, agent: AgentState) {}

// agent-llm-projection r1543/1559（批 3）
#[scenario(
    path = "llmanspec/specs/agent-llm-projection/agent-llm-projection.feature",
    name = "session-vs-llm-vocab-boundary"
)]
fn test_t3_vocab_layers() {}
#[scenario(
    path = "llmanspec/specs/agent-llm-projection/agent-llm-projection.feature",
    name = "projection-via-llm-project"
)]
fn test_t3_projection() {}

// domain-security r66/71/1425（批 3）
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "bash-url-network-gate"
)]
fn test_t3_network_gate() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "trust-single-source-of-truth"
)]
fn test_t3_trust_ssot() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "permission-advice-only"
)]
fn test_t3_permission_advice() {}

// runtime-resource-discovery r1766-1768（批 3）
#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "resource-commands-reuse-loader"
)]
fn test_t3_resources_loader() {}
#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "source-info-public-shape"
)]
fn test_t3_source_info() {}
#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "scope-enum-variants"
)]
fn test_t3_scope_enum() {}

// test-infra r39-63（批 4）
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "faux-provider-no-network"
)]
fn test_t4_faux() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "bdd-harness-step-typed"
)]
fn test_t4_harness() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "temp-file-raii-cleanup"
)]
fn test_t4_temp_raii() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "async-test-timeout-guard"
)]
fn test_t4_timeout() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "no-fixed-tmp-path"
)]
fn test_t4_no_fixed_tmp() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "tui-e2e-workspace-isolation"
)]
fn test_t4_tui_e2e() {}

// test-provider-integration r1821-1826 + r1912（批 4 / c2836）
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "config-value-parser-boundary"
)]
fn test_t4_cfg_parser() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "provider-config-value-expression-boundary"
)]
fn test_t4_cfg_expr() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "env-var-interpolation-branch"
)]
fn test_t4_env_interp() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "shell-command-value-exec"
)]
fn test_t4_cfg_cmd() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "provider-registration-config"
)]
fn test_t4_provider_cfg() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "provider-impl-in-infra"
)]
fn test_t4_provider_layers() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "provider-port-injection"
)]
fn test_t4_provider_port() {}

// test-bdd r37/1811-1814（批 4）
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "bdd-suite-fully-wired"
)]
fn test_t4_bdd_suite() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "bdd-isolated-target-bound"
)]
fn test_t4_bdd_isolated_target() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "server-integration-scenarios-bound"
)]
fn test_t4_server_scn() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "rstest-bdd-current"
)]
fn test_t4_rstest_version() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "live-feature-partition-bound"
)]
fn test_t4_bdd_binding() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "config-behavior-unit-covered"
)]
fn test_t4_cfg_unit() {}

// test-standards r1834-1837（批 4）
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "bdd-unit-boundary-doc"
)]
fn test_t4_boundary_doc() {}
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "core-data-types-tested"
)]
fn test_t4_core_tests() {}
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "pure-logic-components-tested"
)]
fn test_t4_pure_logic() {}
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "session-subcomponents-tested"
)]
fn test_t4_subcomp() {}

// test-hooks-wiring r1817-1819（批 4）
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "smoke-hook-wired"
)]
fn test_t4_smoke_hook() {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "provider-matrix-scenarios"
)]
fn test_t4_provider_matrix() {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "curated-hook-bus-reexport"
)]
fn test_t4_hook_reexport() {}

// test-fake-provider r38/45（批 4）
#[scenario(
    path = "llmanspec/specs/test-fake-provider/test-fake-provider.feature",
    name = "fake-provider-adapter-path"
)]
fn test_t4_fake_path() {}
#[scenario(
    path = "llmanspec/specs/test-fake-provider/test-fake-provider.feature",
    name = "scenario-orchestration-shape"
)]
fn test_t4_scenario_shape() {}
