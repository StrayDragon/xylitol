use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "ReAct结束"
)]
async fn test_ar_react_terminates(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "流为XyEvent"
)]
async fn test_ar_stream_is_xyevent(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "继续之后工具"
)]
async fn test_ar_continues_after_tools(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "意图之前执行"
)]
async fn test_ar_intent_before_execution(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "中止丢弃SSE"
)]
async fn test_ar_abort_drops_sse(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "改向之前模型"
)]
async fn test_ar_steer_before_model(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "追问延长"
)]
async fn test_ar_followup_extends(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "队列更新"
)]
async fn test_ar_queue_update(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "中止清除改向"
)]
async fn test_ar_abort_clears_steer(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "第二次运行之后中止"
)]
async fn test_ar_second_run_after_abort(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "中止取消bang"
)]
async fn test_ar_abort_cancels_bang(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "之前拒绝"
)]
async fn test_ar_before_denies(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "应当停止发出agent结束"
)]
async fn test_ar_should_stop_emits_agent_end(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "应当停止跳过追问"
)]
async fn test_ar_should_stop_skips_followup(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "无钩子打开结束"
)]
async fn test_ar_no_hook_open_end(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "最大回合停止运行"
)]
async fn test_ar_max_turns_stops_run(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "批缺省屏障并行"
)]
async fn test_ar_batch_default_barrier_parallel(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "批屏障并行重叠"
)]
async fn test_ar_batch_barrier_parallel_overlap(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "批屏障保留来源窗口"
)]
async fn test_ar_batch_barrier_preserves_windows(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "批MCP从不并行"
)]
async fn test_ar_batch_mcp_never_parallel(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "批历史来源顺序"
)]
async fn test_ar_batch_history_source_order(agent: AgentState, ws: Workspace) {}

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "首次回合历史为真实用户"
)]
async fn test_c2826_first_turn_history(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "上下文重载信任门控"
)]
async fn test_c2826_context_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "技能重载信任门控与可查询"
)]
async fn test_c2826_skills_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "中止持久化部分带已中止原因"
)]
async fn test_c2826_abort_persist(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "工具回合两轮次一结算"
)]
async fn test_c2826_two_iterations(agent: AgentState, ws: Workspace) {}

// ── c2835 后继：agent-runtime 裸规则回填 ────────────────────────────
use crate::bdd::steps_app_tui_bridge::{BridgeBdd, bridge_bdd};
use crate::bdd::steps_app_tui_interaction::{TuiInteraction, tui_interaction};
use crate::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use crate::bdd::steps_bridge::{PromptBdd, prompt_bdd};
use crate::bdd::steps_c2827::{T2CapsBdd, T2EstBdd, T6McpBdd, t2_caps_bdd, t2_est_bdd, t6_mcp_bdd};
use crate::bdd::steps_protocol::{ProtocolBdd, protocol_bdd};
use crate::bdd::steps_runtime_config::{RcSnap, rc_snap};

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "构建器端口驱动一回合"
)]
async fn test_ar1_builder_ports(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "端口产出Xy事件"
)]
async fn test_ar2_ports_yield_events(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "热已应用槽重建提示词"
)]
async fn test_ar3_hot_slots_next_turn(t2_caps_bdd: T2CapsBdd, agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "未知队列类型降级"
)]
async fn test_ar4_unknown_event_degrades(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "集中缺省思考档位"
)]
fn test_ar5_centralised_defaults(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "完成用量锚点已持久化"
)]
async fn test_ar6_done_usage_persisted(t2_est_bdd: T2EstBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "下一回合重读模型与思考"
)]
async fn test_ar7_next_turn_reresolve(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "回合结束阈值自动压缩"
)]
async fn test_ar8_turn_end_threshold(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "溢出情形一压缩重试"
)]
async fn test_ar9_overflow_case1(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "代码首次上下文策略缺省"
)]
async fn test_ar10_context_policy_defaults(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "首次生成冻结工具表"
)]
async fn test_ar11_mcp_freeze_gate(t6_mcp_bdd: T6McpBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "错误种类存活投影"
)]
async fn test_ar12_error_kind(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "已持久化节点计时结算思考"
)]
async fn test_ar13_stream_node_timings(
    transcript_bdd: TranscriptBdd,
    tui_interaction: TuiInteraction,
) {
}

// c2924：RuntimePhase 迁移表（xstate E1-E4 执行侧；sync fn 保全 case 展开）
#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "阶段转换矩阵"
)]
fn test_ar_phase_transition_matrix(from: String, event: String, guard: String, to: String) {}
