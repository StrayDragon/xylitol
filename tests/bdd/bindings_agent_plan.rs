use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-plan/agent-plan.feature",
    name = "计划模式拒绝写入工具"
)]
fn test_plan_gate_denies_write() {}

#[scenario(
    path = "llmanspec/specs/agent-plan/agent-plan.feature",
    name = "切回执行模式后写入恢复"
)]
fn test_plan_gate_restores_write() {}

#[scenario(
    path = "llmanspec/specs/agent-plan/agent-plan.feature",
    name = "项目计划跨会话可读"
)]
fn test_plan_project_scope_resumes() {}

#[scenario(
    path = "llmanspec/specs/agent-plan/agent-plan.feature",
    name = "未信任项目计划被拒绝"
)]
fn test_plan_project_scope_requires_trust() {}
