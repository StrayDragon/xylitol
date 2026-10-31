use crate::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办栏默认只占一行摘要"
)]
fn test_todo_bar_default_shows_in_progress(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "点击摘要行展开为分组列表"
)]
fn test_todo_bar_expands_on_click(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "状态字符与亮度三档"
)]
fn test_todo_bar_status_glyphs(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "超长清单在栏内滚动"
)]
fn test_todo_bar_overflow(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办栏续跑匹配在线"
)]
fn test_todo_bar_resume_matches_live(transcript_bdd: TranscriptBdd) {}
