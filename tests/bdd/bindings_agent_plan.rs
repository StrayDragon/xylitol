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
