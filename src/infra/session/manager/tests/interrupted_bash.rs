//! Interrupted-bash fold in `build_session_context` (c2770 / as-bang1): the
//! session-context assembly path MUST apply the same orphan-running fold as
//! the ReAct seeding path (as48 single path); pairing is session-scoped, so a
//! done row anywhere suppresses the notice.

use super::super::*;
use crate::protocol::message::BashExecutionStatus;
use crate::protocol::session::bash_execution_message_entry;

async fn append_bash(
    mgr: &SessionManager,
    sid: &str,
    id: &str,
    bash_id: &str,
    command: &str,
    status: BashExecutionStatus,
    exclude: bool,
) {
    let mut entry = bash_execution_message_entry(
        bash_id,
        command,
        "",
        Some(0),
        false,
        false,
        None,
        exclude,
        status,
    );
    if let crate::protocol::session::SessionEntry::Message(ref mut m) = entry {
        m.base.id = id.into();
    }
    mgr.append(sid, &entry).await.unwrap();
}

fn context_text(context: &SessionContext) -> String {
    context
        .messages
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn context_projects_orphan_running_as_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("orphan", Some("."), None).await.unwrap();
    append_bash(
        &mgr,
        "orphan",
        "r1",
        "b1",
        "serve",
        BashExecutionStatus::Running,
        false,
    )
    .await;

    let context = mgr.build_session_context("orphan").await.unwrap();
    let text = context_text(&context);
    assert!(
        text.contains("[interrupted] $ serve"),
        "session context MUST carry the interrupted notice: {text}"
    );
}

#[tokio::test]
async fn context_suppresses_notice_when_done_exists_anywhere() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("paired", Some("."), None).await.unwrap();
    append_bash(
        &mgr,
        "paired",
        "r1",
        "b1",
        "serve",
        BashExecutionStatus::Running,
        false,
    )
    .await;
    append_bash(
        &mgr,
        "paired",
        "d1",
        "b1",
        "serve",
        BashExecutionStatus::Done,
        false,
    )
    .await;

    let context = mgr.build_session_context("paired").await.unwrap();
    let text = context_text(&context);
    assert!(
        !text.contains("[interrupted]"),
        "a done row anywhere in the session MUST suppress the notice: {text}"
    );
    let has_done = context.messages.iter().any(|v| {
        v.get("role").and_then(|r| r.as_str()) == Some("bashExecution")
            && v.get("status").and_then(|s| s.as_str()) == Some("done")
    });
    assert!(
        has_done,
        "the done row keeps its context entry: {:?}",
        context.messages
    );
}

#[tokio::test]
async fn context_never_projects_excluded_orphan() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("excluded", Some("."), None).await.unwrap();
    append_bash(
        &mgr,
        "excluded",
        "r1",
        "b1",
        "clean",
        BashExecutionStatus::Running,
        true,
    )
    .await;

    let context = mgr.build_session_context("excluded").await.unwrap();
    assert!(
        context.messages.is_empty(),
        "`!!` orphan MUST NOT reach the model context: {:?}",
        context.messages
    );
}
