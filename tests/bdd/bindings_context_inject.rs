use crate::bdd::steps_context_inject::{
    ContextInjectBdd, ReminderProjectionBdd, context_inject_bdd, reminder_projection,
};
use crate::bdd::steps_runtime_config::{RcSnap, rc_snap};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "技能变更在下一 turn_pre 注入提醒"
)]
async fn test_context_reminder_skill_change(context_inject_bdd: ContextInjectBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "resume 离开期间变更注入"
)]
async fn test_context_reminder_resume(context_inject_bdd: ContextInjectBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "context_inject 重复 id 报错"
)]
fn test_context_inject_duplicate_id(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/agent-llm-projection/agent-llm-projection.feature",
    name = "提醒行先于用户消息投影"
)]
async fn test_reminder_row_precedes_user(reminder_projection: ReminderProjectionBdd) {}
