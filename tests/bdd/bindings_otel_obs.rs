//! infra-otel BDD 绑定（otel6/7/8/10/11/12/22 低频观测族）。

use crate::bdd::fixtures::*;
use crate::bdd::fixtures::{Workspace, ws};
use crate::bdd::steps_otel_obs::{OtelBdd, otel_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP会话标识时回合根无头"
)]
#[serial]
async fn test_otel6_session_id(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP会话名称元数据无头"
)]
#[serial]
async fn test_otel7_session_name(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP观测类型无头"
)]
#[serial]
async fn test_otel8_observation_types(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP回合跨度树无头"
)]
#[serial]
async fn test_otel11_span_tree(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP产品跨度名称无头"
)]
#[serial]
async fn test_otel12_product_names(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP观测泳道模型无头"
)]
#[serial]
async fn test_otel22_obs_lane(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP观测IO档无头"
)]
#[serial]
async fn test_otel10_io_tier(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP缺省空无跨度"
)]
#[serial]
async fn test_c2826_otel_default_none(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[cfg(feature = "otel")]
#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP配置武装报告器"
)]
#[serial]
async fn test_c2826_otlp_arms(otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP坏端点回落返回带诊断"
)]
#[serial]
async fn test_c2826_otlp_bad_endpoint(otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "工具IO档已截断仅工具"
)]
#[serial]
async fn test_c2826_tool_tier_only(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "压缩跨度越过准备"
)]
#[serial]
async fn test_c2826_compaction_span(sess: XySessionStore, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "回合正常完成不错误"
)]
#[serial]
async fn test_c2826_turn_not_error(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "结算单一词元估算"
)]
#[serial]
async fn test_c2826_single_estimate(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

// ── c2835 后继：infra-otel 裸规则回填 ──────────────────────────────

#[cfg(feature = "otel")]
#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP退出已构建时fastrace"
)]
#[serial]
async fn test_otel_r1487_fastrace_only(otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "文件与OTLP扇出出"
)]
#[serial]
async fn test_otel_r1488_fan_out(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "生成跨度携带模型泳道"
)]
#[serial]
async fn test_otel_r1492_generation_lane(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "并行工具共享轮次树"
)]
#[serial]
async fn test_otel_r1475_parallel_tree(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "会话标识每重叠处理"
)]
#[serial]
async fn test_otel_r1481_per_processing(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "整棵树一会话标识"
)]
#[serial]
async fn test_otel_r1482_whole_tree(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "观测槽已写按写者路径"
)]
#[serial]
async fn test_otel_r1483_slot_discipline(sess: XySessionStore, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "双会话身份键"
)]
#[serial]
async fn test_otel_r1484_dual_identity(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "已跳过压缩单一估算"
)]
#[serial]
async fn test_otel_r1485_skipped_compaction(sess: XySessionStore, otel_bdd: OtelBdd) {}
#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "所有者限定OTLP断言"
)]
#[serial]
async fn test_otel_r1918_owner_scoped(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}
