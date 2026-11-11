use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-workflow/agent-workflow.feature",
    name = "workflow_write 后进入任务指令"
)]
fn test_workflow_write_then_task_instruction() {}

#[scenario(
    path = "llmanspec/specs/agent-workflow/agent-workflow.feature",
    name = "workflow_fail 标记任务失败"
)]
fn test_workflow_fail_marks_task() {}

#[scenario(
    path = "llmanspec/specs/agent-workflow/agent-workflow.feature",
    name = "嵌套子 workflow 完成后父任务标记完成"
)]
fn test_nested_workflow_completion_marks_parent() {}
