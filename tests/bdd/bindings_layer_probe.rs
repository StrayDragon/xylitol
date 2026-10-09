//! 结构探针与测试基建 BDD bindings（c2853 按域从 bindings_c2827 拆出）。

use crate::bdd::fixtures::{AgentState, Workspace, agent, ws};
use rstest_bdd_macros::scenario;

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
