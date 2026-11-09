use crate::bdd::fixtures::{AgentState, Workspace, agent, ws};
use crate::bdd::steps_bridge::{PromptBdd, prompt_bdd};
use crate::bdd::steps_runtime_resource_discovery_extra::{T2ResBdd, t2_res_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "资源列表技能主题无提示词"
)]
fn test_rd_list(agent: AgentState, ws: Workspace, prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "上下文热重载信任门控"
)]
async fn test_rd_context_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "技能热重载信任门控"
)]
async fn test_rd_skills_reload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature",
    name = "主题列出按词干"
)]
fn test_rd_themes(t2_res_bdd: T2ResBdd) {}
