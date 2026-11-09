use crate::bdd::fixtures::{AgentState, agent};
use crate::bdd::steps_agent_todo_extra::{T2TodoBdd, t2_todo_bdd};
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

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办工具写入与拒绝"
)]
async fn test_td_write_reject(t2_todo_bdd: T2TodoBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "多个于进行中已接受"
)]
async fn test_td_multi_in_progress(t2_todo_bdd: T2TodoBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办内容越过80拒绝"
)]
async fn test_td_overlong(t2_todo_bdd: T2TodoBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办快照最新胜出"
)]
async fn test_td_latest_wins(t2_todo_bdd: T2TodoBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "压缩再追加待办快照"
)]
async fn test_td_compact_ensure(t2_todo_bdd: T2TodoBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "导出包含agent待办"
)]
async fn test_td_export(t2_todo_bdd: T2TodoBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "待办更新发出类型化事件"
)]
async fn test_td_typed_event(agent: AgentState) {}
