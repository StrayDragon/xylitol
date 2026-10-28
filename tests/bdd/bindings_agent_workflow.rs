use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-workflow/agent-workflow.feature",
    name = "workflow_write 后进入步骤指令"
)]
fn test_workflow_write_then_step_instruction() {}
