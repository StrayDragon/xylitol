//! 结构探针与测试基建 BDD bindings（c2853 按域从 bindings_c2827 拆出）。

use rstest_bdd_macros::scenario;

// infra-observability r1461/1464/1465（批 2）
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "组合根单一观测出口"
)]
fn test_t2_obs_sink() {}
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "fastrace单一栈"
)]
fn test_t2_obs_stack() {}
#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "厂商追踪关联跨度"
)]
fn test_t2_obs_spans() {}

// infra-diagnostics r1437-1439（批 2）
#[scenario(
    path = "llmanspec/specs/infra-diagnostics/infra-diagnostics.feature",
    name = "计时收集器门控"
)]
fn test_t2_timing_collector() {}
#[scenario(
    path = "llmanspec/specs/infra-diagnostics/infra-diagnostics.feature",
    name = "计时指向时关键路径"
)]
fn test_t2_timing_sites() {}
#[scenario(
    path = "llmanspec/specs/infra-diagnostics/infra-diagnostics.feature",
    name = "计时输出毫秒"
)]
fn test_t2_timing_output() {}

// agent-todo r1118-1128/1120（批 3）
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办严格模型形状"
)]
fn test_t3_todo_model() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办快照之外前缀"
)]
fn test_t3_todo_snapshot() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办并发屏障"
)]
fn test_t3_todo_barrier() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "状态栏读取仅边界"
)]
fn test_t3_todo_status_bar() {}
#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办内置首次回合冻结"
)]
fn test_t3_todo_freeze() {}

// agent-session-store r41/1114-1117/1116（批 3）
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "压缩触发时预留超出"
)]
fn test_t3_compaction_trigger() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "工作目录校验之前恢复"
)]
fn test_t3_cwd_check() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "日志读取最近边界"
)]
fn test_t3_journal_recent() {}
#[scenario(
    path = "llmanspec/specs/agent-session-store/agent-session-store.feature",
    name = "导出IO端口注入"
)]
fn test_t3_export_io() {}

// agent-session：分层扫描闸已撤（会话行为测仍在 bindings_agent_session）。

// agent-llm-projection r1543/1559（批 3）
#[scenario(
    path = "llmanspec/specs/agent-llm-projection/agent-llm-projection.feature",
    name = "会话对照模型词表边界"
)]
fn test_t3_vocab_layers() {}
#[scenario(
    path = "llmanspec/specs/agent-llm-projection/agent-llm-projection.feature",
    name = "投影经模型项目"
)]
fn test_t3_projection() {}

// domain-security r66/71/1425（批 3）
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "bash网址网络门禁"
)]
fn test_t3_network_gate() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "信任单一来源真源"
)]
fn test_t3_trust_ssot() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "权限建议仅"
)]
fn test_t3_permission_advice() {}

// runtime-resource-discovery r1767-1768
#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "来源信息公开形状"
)]
fn test_t3_source_info() {}
#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "作用域枚举变体"
)]
fn test_t3_scope_enum() {}

// test-infra r39-63（批 4）
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "伪厂商无网络"
)]
fn test_t4_faux() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "BDD夹具步进类型化"
)]
fn test_t4_harness() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "临时文件RAII清理"
)]
fn test_t4_temp_raii() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "异步测试超时守卫"
)]
fn test_t4_timeout() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "无固定临时路径路径"
)]
fn test_t4_no_fixed_tmp() {}
#[scenario(
    path = "llmanspec/specs/test-infra/test-infra.feature",
    name = "TUI端到端工作区隔离"
)]
fn test_t4_tui_e2e() {}

// test-provider-integration r1821-1826 + r1912（批 4 / c2836）
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "配置值解析器边界"
)]
fn test_t4_cfg_parser() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "厂商配置值表达式边界"
)]
fn test_t4_cfg_expr() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "环境变量变量插值分支"
)]
fn test_t4_env_interp() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "shell命令值执行"
)]
fn test_t4_cfg_cmd() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "厂商登记配置"
)]
fn test_t4_provider_cfg() {}
#[scenario(
    path = "llmanspec/specs/test-provider-integration/test-provider-integration.feature",
    name = "厂商端口注入"
)]
fn test_t4_provider_port() {}

// test-bdd r37/1811-1814（批 4）
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "BDD套件全量接线"
)]
fn test_t4_bdd_suite() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "BDD隔离目标已绑"
)]
fn test_t4_bdd_isolated_target() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "服务端集成场景已绑"
)]
fn test_t4_server_scn() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "rstestBDD当前"
)]
fn test_t4_rstest_version() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "在线feature分区已绑"
)]
fn test_t4_bdd_binding() {}
#[scenario(
    path = "llmanspec/specs/test-bdd/test-bdd.feature",
    name = "配置行为单测覆盖"
)]
fn test_t4_cfg_unit() {}

// test-standards r1834-1837（批 4）
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "BDD单测边界文档"
)]
fn test_t4_boundary_doc() {}
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "核心数据类型已测"
)]
fn test_t4_core_tests() {}
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "纯逻辑组件已测"
)]
fn test_t4_pure_logic() {}
#[scenario(
    path = "llmanspec/specs/test-standards/test-standards.feature",
    name = "会话子组件已测"
)]
fn test_t4_subcomp() {}

// test-hooks-wiring r1817-1819（批 4）
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "冒烟钩子接线"
)]
fn test_t4_smoke_hook() {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "厂商矩阵场景"
)]
fn test_t4_provider_matrix() {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "精选钩子总线再导出"
)]
fn test_t4_hook_reexport() {}

// test-hooks-wiring r1820（c2922：模型操作经观察矩阵覆盖）
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "模型操作覆盖按观察矩阵"
)]
fn test_t4_model_ops_covered() {}

// test-fake-provider r38/45（批 4）
#[scenario(
    path = "llmanspec/specs/test-fake-provider/test-fake-provider.feature",
    name = "假厂商适配器路径"
)]
fn test_t4_fake_path() {}
#[scenario(
    path = "llmanspec/specs/test-fake-provider/test-fake-provider.feature",
    name = "场景编排形状"
)]
fn test_t4_scenario_shape() {}
