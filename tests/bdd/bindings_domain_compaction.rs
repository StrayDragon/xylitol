use crate::bdd::fixtures::*;
use crate::bdd::steps_c2844::compaction_probe;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "阈值预留门禁"
)]
fn test_compaction_threshold_gate(
    agent: AgentState,
    tokens: u32,
    enabled: bool,
    expect_compact: bool,
    ws: Workspace,
) {
}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "下限感知阈值"
)]
fn test_comp_floor_outline(
    agent: AgentState,
    window: u64,
    reserve: u64,
    keep: u64,
    overhead: u64,
    tokens: u32,
    expect_compact: bool,
    ws: Workspace,
) {
}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "保留最近"
)]
fn test_compaction_keep_recent(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "写入入口"
)]
fn test_compaction_write(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "分支摘要"
)]
fn test_compaction_branch(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "估算使用优先级"
)]
fn test_comp_est_priority(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "估算回退启发式"
)]
fn test_comp_est_fallback(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "查找裁切"
)]
fn test_comp_find_cut(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "来源可用"
)]
fn test_comp_provenance(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "预留触发共享页脚估算"
)]
fn test_comp_reserve_trigger_shared(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "摘要"
)]
async fn test_comp_summarize_c3(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "生成摘要"
)]
async fn test_comp_generate_summary(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "场景agent"
)]
async fn test_comp_agent_c12(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "持久化"
)]
async fn test_comp_persist(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "分支"
)]
fn test_comp_branch_c5(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "迭代"
)]
async fn test_comp_iterative(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "文件"
)]
async fn test_comp_files(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "入口"
)]
async fn test_comp_entry(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "拆分按职责"
)]
async fn test_comp_split(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "自动越过阈值"
)]
fn test_comp_auto_over(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "自动低于阈值"
)]
fn test_comp_auto_under(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "自动关闭无压缩"
)]
fn test_comp_auto_disabled(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "手动强制绕过预留"
)]
fn test_comp_manual_force(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "压缩使用叶子分支路径"
)]
async fn test_comp_leaf_branch(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "过期守卫之后压缩"
)]
fn test_comp_stale_guard(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "裁切助手"
)]
fn test_comp_cut_assistant(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "从不工具结果"
)]
fn test_comp_never_tool(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "保留预算"
)]
fn test_comp_keep_budget(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "拆分双摘要"
)]
async fn test_comp_split_dual(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "词元之前"
)]
async fn test_comp_tokens_before(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "溢出重试成功"
)]
fn test_comp_overflow_retry_ok(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "溢出一次"
)]
fn test_comp_overflow_once(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "错误模型"
)]
fn test_comp_overflow_wrong_model(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "原因溢出"
)]
fn test_comp_reason_overflow(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "摘要模型入口"
)]
async fn test_c7_summary_model_entry(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "摘要模型回退提示"
)]
async fn test_c7_summary_fallback(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "摘要思考覆盖出设置"
)]
async fn test_c7_thinking_override(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "回合结束溢出事件之前agent结束"
)]
async fn test_c2844_turn_end_compaction_events(
    compaction_probe: crate::bdd::steps_c2844::CompactionProbe,
    agent: AgentState,
    ws: Workspace,
) {
}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "推理仅摘要不空"
)]
async fn test_c2844_reasoning_only_summary(
    compaction_probe: crate::bdd::steps_c2844::CompactionProbe,
    sess: XySessionStore,
    agent: AgentState,
    ws: Workspace,
) {
}
#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "压缩裁切指标同一来源"
)]
fn test_c2848_cut_metric_same_source(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/domain-compaction/domain-compaction.feature",
    name = "压缩后保留 plan 模式"
)]
fn test_comp_preserves_plan_mode(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
