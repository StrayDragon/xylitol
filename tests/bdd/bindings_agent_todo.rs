//! agent-todo BDD — 待办栏投影（r1129 / r1130）。

use crate::tests::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-bar-default-shows-in-progress"
)]
fn test_todo_bar_default_shows_in_progress(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-todo/agent-todo.feature",
    name = "todo-bar-resume-matches-live"
)]
fn test_todo_bar_resume_matches_live(transcript_bdd: TranscriptBdd) {}
