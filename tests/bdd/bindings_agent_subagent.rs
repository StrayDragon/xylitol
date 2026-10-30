use crate::bdd::fixtures::*;
use crate::bdd::steps_agent_subagent::{AgentSubagentBdd, agent_subagent_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-subagent/agent-subagent.feature",
    name = "未配置子代理零装配"
)]
async fn test_agent_subagent_zero(ws: Workspace, agent_subagent_bdd: AgentSubagentBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-subagent/agent-subagent.feature",
    name = "配置子代理注册只读工具"
)]
async fn test_agent_subagent_configured(ws: Workspace, agent_subagent_bdd: AgentSubagentBdd) {}
