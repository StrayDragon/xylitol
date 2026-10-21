use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "拒绝不支持"
)]
fn test_m10_reject_unsupported(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "拒绝情形变体"
)]
fn test_m15_reject_case_variant(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "任务模型解析独立"
)]
fn test_m18_task_model_ok(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "任务模型构建失败回落返回"
)]
fn test_m18_task_model_fallback(agent: AgentState) {}

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_runtime_config::{RcSnap, rc_snap};
use crate::bdd::steps_tokenizer::{TokenizerBdd, tokenizer_bdd};

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "厂商登记驱动回合"
)]
async fn test_m20_provider_registration(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "可用模型表面于列表"
)]
async fn test_m21_available_models(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "内置缺省两族有序"
)]
fn test_m22_builtin_provider_defaults() {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "解析器保留已宣告档位"
)]
fn test_m23_resolver_keeps_levels(rc_snap: RcSnap, tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "解析器回落返回至会话模型"
)]
async fn test_m24_task_model_fallback(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "生成携带思考档位"
)]
async fn test_m25_generate_carries_level(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "选择胜出越过设置缺省"
)]
fn test_m26_select_wins_over_default(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "入口API与映射存活解析"
)]
fn test_m27_entry_api_map(rc_snap: RcSnap, tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "清单API缺省按厂商"
)]
fn test_m28_manifest_api_default() {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "显式入口登记独立模型"
)]
async fn test_m29_explicit_entry_independent(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "解析器回退保留意图"
)]
async fn test_m30_resolver_fallback_intent(agent: AgentState) {}
