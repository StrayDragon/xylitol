use crate::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办栏缺省显示于进行中"
)]
fn test_todo_bar_default_shows_in_progress(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办栏续跑匹配在线"
)]
fn test_todo_bar_resume_matches_live(transcript_bdd: TranscriptBdd) {}
