use crate::bdd::fixtures::{AgentState, agent};
use crate::bdd::steps_agent_prompt_extra::{T2CapsBdd, T2SkillBdd, t2_caps_bdd, t2_skill_bdd};
use crate::bdd::steps_bridge::{PromptBdd, ProviderBdd, prompt_bdd, provider_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "可用工具省略MCP名称"
)]
fn test_ap_mcp_omitted(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "应用提示词资源下一运行"
)]
fn test_ap_apply_resources(agent: AgentState, t2_caps_bdd: T2CapsBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "应用技能重建目录"
)]
fn test_ap_apply_skills(agent: AgentState, t2_caps_bdd: T2CapsBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "美元符技能展开正文"
)]
fn test_ap_dollar_expand(t2_skill_bdd: T2SkillBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "装配包含工具指南"
)]
fn test_ap_r1015_guidelines(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "运行时策略片段注入"
)]
fn test_ap_r1016_runtime_policy(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "日期与工作目录来自会话环境变量"
)]
fn test_ap_r1018_session_env() {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "系统提示词单一复制"
)]
fn test_ap_r1019_single_copy(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "技能列出之后信任"
)]
fn test_ap_r1020_skills(agent: AgentState, t2_caps_bdd: T2CapsBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "替换跳过缺省片段"
)]
fn test_ap_r1022_replacement(prompt_bdd: PromptBdd) {}
