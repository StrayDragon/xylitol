//! Persist fixture entries into an already-created session.

use serde_json::{Value, json};

use crate::app::core::driver_error::XyDriverError;
use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason};
use crate::protocol::ports::XySessionStore;
use crate::protocol::session::{
    EntryBase, LabelEntry, MessageEntry, SessionEntry, fixture_message_json,
};

use super::catalog::resolve_scene_id;

fn empty_base(entry_type: &str) -> EntryBase {
    EntryBase {
        entry_type: entry_type.into(),
        id: String::new(),
        parent_id: None,
        timestamp: 0,
    }
}

fn user_msg(text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: empty_base("message"),
        message: fixture_message_json("user", text),
    })
}

fn assistant_msg(text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: empty_base("message"),
        message: fixture_message_json("assistant", text),
    })
}

fn agent_entry(msg: AgentMessage) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: empty_base("message"),
        message: serde_json::to_value(&msg).unwrap_or(Value::Null),
    })
}

fn tool_call(id: &str, name: &str, args: Value) -> AgentPart {
    AgentPart::ToolCall {
        id: id.into(),
        name: name.into(),
        arguments: args,
    }
}

fn assistant_parts(content: Vec<AgentPart>) -> SessionEntry {
    agent_entry(AgentMessage::Llm(LlmMessage::AssistantMessage {
        content,
        stop_reason: Some(XyStopReason::Stop),
        usage: None,
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    }))
}

fn tool_result_entry(id: &str, name: &str, text: &str) -> SessionEntry {
    agent_entry(AgentMessage::tool_result(
        id,
        name,
        vec![AgentPart::text(text)],
        false,
    ))
}

const ASK_ANSWERED_JSON: &str = r#"{"status":"answered","answers":[{"id":"next","values":["continue"],"labels":["Continue"],"was_custom":false}]}"#;

/// Newest-turn rows matching the live-window tape, already completed (Ask answered).
fn activity_fold_resume_newest_turn() -> Vec<SessionEntry> {
    vec![
        user_msg("debug: activity-fold live window"),
        assistant_parts(vec![
            AgentPart::thinking("consider next edit"),
            tool_call("r1", "read", json!({ "path": "old.rs" })),
            AgentPart::text("mid-body"),
            tool_call("e1", "edit", json!({ "path": "a.rs" })),
            tool_call("e2", "edit", json!({ "path": "b.rs" })),
            tool_call(
                "ask1",
                "ask",
                json!({
                    "questions": [{
                        "id": "next",
                        "prompt": "activity-fold-live: next step?",
                        "mode": "single",
                        "options": [
                            { "value": "continue", "label": "Continue" },
                            { "value": "stop", "label": "Stop here" }
                        ]
                    }]
                }),
            ),
        ]),
        tool_result_entry("r1", "read", "ok"),
        tool_result_entry("e1", "edit", "ok"),
        tool_result_entry("e2", "edit", "ok"),
        tool_result_entry("ask1", "ask", ASK_ANSWERED_JSON),
        assistant_msg("Thanks — continuing from your answer."),
    ]
}

fn activity_fold_resume_raw() -> Vec<SessionEntry> {
    let mut rows = Vec::new();
    for i in 0..2 {
        rows.push(user_msg(&format!("debug: older turn {i}")));
        rows.push(assistant_parts(vec![
            AgentPart::thinking(format!("older thinking {i}")),
            tool_call(
                &format!("old-r{i}"),
                "read",
                json!({ "path": format!("old{i}.rs") }),
            ),
            AgentPart::text(format!("debug: older reply {i}")),
        ]));
        rows.push(tool_result_entry(&format!("old-r{i}"), "read", "ok"));
    }
    rows.extend(activity_fold_resume_newest_turn());
    rows
}

/// Stamp ids / parents / unix-ms clocks so harness rebuild paints `Worked for`.
#[cfg(test)]
pub fn activity_fold_resume_stamped_entries() -> Vec<SessionEntry> {
    let mut parent: Option<String> = None;
    let mut out = Vec::new();
    for (i, entry) in activity_fold_resume_raw().into_iter().enumerate() {
        let id = format!("af-resume-{i}");
        let ts = 1_781_827_200_000u64 + i as u64 * 1000; // 2026-06-19T00:00:00Z + i s
        out.push(rebase_message(entry, &id, parent.as_deref(), ts));
        parent = Some(id);
    }
    out
}

#[cfg(test)]
fn rebase_message(entry: SessionEntry, id: &str, parent: Option<&str>, ts: u64) -> SessionEntry {
    match entry {
        SessionEntry::Message(m) => SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: ts,
            },
            message: m.message,
        }),
        other => other,
    }
}

async fn seed_activity_fold_resume(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<(), XyDriverError> {
    for entry in activity_fold_resume_raw() {
        store.append_session_entry(session_id, &entry).await?;
    }
    Ok(())
}

/// Seed `session_id` for a known scene id (canonical or alias).
pub async fn seed_scene(
    store: &dyn XySessionStore,
    session_id: &str,
    scene: &str,
) -> Result<&'static str, XyDriverError> {
    let id = resolve_scene_id(scene).ok_or_else(|| {
        XyDriverError::invalid_input(format!(
            "unknown debug scene: {scene}\n{}",
            super::catalog::list_note()
        ))
    })?;
    match id {
        "session-tree-multiturn" => seed_multiturn(store, session_id).await?,
        "session-tree-labeled" => seed_labeled(store, session_id).await?,
        "session-tree-branched" => seed_branched(store, session_id).await?,
        "ao-perf-scroll" => seed_ao_perf_scroll(store, session_id).await?,
        "activity-fold-resume" => seed_activity_fold_resume(store, session_id).await?,
        _ => {
            return Err(XyDriverError::invalid_input(format!(
                "unhandled debug scene id: {id}"
            )));
        }
    }
    Ok(id)
}

/// Long spine for ApplicationOwned wheel / select CPU hand-tests (~80 turns).
async fn seed_ao_perf_scroll(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<(), XyDriverError> {
    const TURNS: usize = 80;
    let chunk = "字".repeat(48);
    for i in 0..TURNS {
        let user = format!("debug ao-perf #{i}: ask {chunk}");
        let assistant = format!(
            "debug ao-perf #{i}: reply {chunk} — padding for scroll/select CPU ({i}/{TURNS})"
        );
        store
            .append_session_entry(session_id, &user_msg(&user))
            .await?;
        store
            .append_session_entry(session_id, &assistant_msg(&assistant))
            .await?;
    }
    Ok(())
}

async fn seed_multiturn(store: &dyn XySessionStore, session_id: &str) -> Result<(), XyDriverError> {
    store
        .append_session_entry(session_id, &user_msg("debug: hi"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: hello from fixture"))
        .await?;
    store
        .append_session_entry(session_id, &user_msg("debug: 你好"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: 有什么可以帮你？"))
        .await?;
    Ok(())
}

/// Same-session sibling branches: rewind leaf after the first spine, then grow an alt path.
///
/// ```text
/// u_root → a_root ─┬─ u_main → a_main
///                  └─ u_alt  → a_alt
/// ```
async fn seed_branched(store: &dyn XySessionStore, session_id: &str) -> Result<(), XyDriverError> {
    store
        .append_session_entry(session_id, &user_msg("debug: root"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: root reply"))
        .await?;
    let fork_parent = store.leaf_id(session_id).ok_or_else(|| {
        XyDriverError::message("debug session-tree-branched: missing fork parent leaf")
    })?;

    store
        .append_session_entry(session_id, &user_msg("debug: main branch"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: main leaf"))
        .await?;

    store.set_leaf(session_id, Some(&fork_parent));
    store
        .append_session_entry(session_id, &user_msg("debug: alt branch"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: alt leaf"))
        .await?;
    Ok(())
}

async fn seed_labeled(store: &dyn XySessionStore, session_id: &str) -> Result<(), XyDriverError> {
    store
        .append_session_entry(session_id, &user_msg("debug: labeled root"))
        .await?;
    store
        .append_session_entry(session_id, &assistant_msg("debug: labeled reply"))
        .await?;
    let entries = store.load_entries(session_id).await?;
    let target = entries
        .iter()
        .find_map(|e| match e {
            SessionEntry::Message(m)
                if m.message
                    .get("role")
                    .and_then(|r| r.as_str())
                    .is_some_and(|r| r == "user") =>
            {
                e.entry_id().map(str::to_string)
            }
            _ => None,
        })
        .ok_or_else(|| XyDriverError::message("debug session-tree-labeled: missing user entry"))?;
    store
        .append_session_entry(
            session_id,
            &SessionEntry::Label(LabelEntry {
                base: empty_base("label"),
                target_id: target,
                label: Some("bookmark".into()),
            }),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionManager;
    use crate::protocol::session::message_text;

    #[tokio::test]
    async fn seed_multiturn_has_user_text() {
        let mgr = SessionManager::in_memory();
        mgr.create("d1", Some("."), None).await.unwrap();
        let id = seed_scene(&mgr, "d1", "session-tree-multiturn")
            .await
            .unwrap();
        assert_eq!(id, "session-tree-multiturn");
        let entries = mgr.load_entries("d1").await.unwrap();
        let texts: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => {
                    let t = message_text(&m.message);
                    (!t.is_empty()).then_some(t)
                }
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("debug: hi")), "{texts:?}");
    }

    #[tokio::test]
    async fn seed_labeled_alias_has_label() {
        let mgr = SessionManager::in_memory();
        mgr.create("d2", Some("."), None).await.unwrap();
        seed_scene(&mgr, "d2", "tree-labeled").await.unwrap();
        let entries = mgr.load_entries("d2").await.unwrap();
        assert!(
            entries.iter().any(|e| matches!(
                e,
                SessionEntry::Label(LabelEntry {
                    label: Some(l),
                    ..
                }) if l == "bookmark"
            )),
            "{entries:?}"
        );
    }

    #[tokio::test]
    async fn seed_branched_has_sibling_children() {
        use crate::protocol::session::build_session_tree;

        let mgr = SessionManager::in_memory();
        mgr.create("d3", Some("."), None).await.unwrap();
        let id = seed_scene(&mgr, "d3", "branched").await.unwrap();
        assert_eq!(id, "session-tree-branched");
        let tree = build_session_tree(&mgr.load_entries("d3").await.unwrap());
        let max_siblings = tree
            .iter()
            .flat_map(|n| n.children.iter())
            .map(|n| n.children.len())
            .max()
            .unwrap_or(0);
        assert!(
            max_siblings >= 2,
            "expected a parent with ≥2 children, got max={max_siblings}; tree={tree:#?}"
        );
    }

    #[tokio::test]
    async fn seed_ao_perf_scroll_has_many_messages() {
        let mgr = SessionManager::in_memory();
        mgr.create("d4", Some("."), None).await.unwrap();
        let id = seed_scene(&mgr, "d4", "long-transcript").await.unwrap();
        assert_eq!(id, "ao-perf-scroll");
        let entries = mgr.load_entries("d4").await.unwrap();
        let msgs = entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(_)))
            .count();
        assert!(msgs >= 160, "expected ~160 messages, got {msgs}");
    }

    #[tokio::test]
    async fn seed_activity_fold_resume_has_tools_ask_and_older_turns() {
        use crate::protocol::session::{count_tool_calls, is_tool_call_part, message_parts};

        let mgr = SessionManager::in_memory();
        mgr.create("d5", Some("."), None).await.unwrap();
        let id = seed_scene(&mgr, "d5", "activity-fold-resume")
            .await
            .unwrap();
        assert_eq!(id, "activity-fold-resume");
        let entries = mgr.load_entries("d5").await.unwrap();
        let users = entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(m) if message_text(&m.message).contains("debug:")))
            .count();
        assert!(
            users >= 3,
            "expected older turns + live turn, got {entries:?}"
        );

        let assistant_tools: usize = entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => Some(count_tool_calls(&m.message)),
                _ => None,
            })
            .sum();
        assert!(
            assistant_tools >= 4,
            "expected read/edit/edit/ask tool calls, got {assistant_tools}"
        );

        let has_ask_call = entries.iter().any(|e| match e {
            SessionEntry::Message(m) => message_parts(&m.message).is_some_and(|parts| {
                parts.iter().any(|p| {
                    is_tool_call_part(p) && p.get("name").and_then(|n| n.as_str()) == Some("ask")
                })
            }),
            _ => false,
        });
        assert!(has_ask_call, "missing ask toolCall: {entries:?}");

        let has_ask_result = entries.iter().any(|e| match e {
            SessionEntry::Message(m) => {
                m.message.get("role").and_then(|r| r.as_str()) == Some("toolResult")
                    && m.message.get("toolName").and_then(|n| n.as_str()) == Some("ask")
            }
            _ => false,
        });
        assert!(has_ask_result, "missing ask toolResult: {entries:?}");
        assert!(
            entries.iter().any(|e| match e {
                SessionEntry::Message(m) => {
                    message_text(&m.message).contains("continuing from your answer")
                }
                _ => false,
            }),
            "missing closing assistant text: {entries:?}"
        );
    }
}
