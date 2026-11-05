use serde_json::json;

use super::super::*;
use xylitol_plugin_sdk::session::{EdgeKind, EdgeRecord, ForkPosition};

fn message(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: 0,
        },
        message: json!({
            "role": "user",
            "content": [{"type": "text", "text": text}],
            "timestamp": 0
        }),
    })
}

fn edge(id: &str, from: &str, to: &str, order: u32, enabled: bool, timestamp: u64) -> EdgeRecord {
    EdgeRecord {
        id: id.into(),
        edge_kind: EdgeKind::Context,
        from: from.into(),
        to: to.into(),
        order,
        enabled,
        timestamp,
    }
}

fn context_text(context: &SessionContext) -> String {
    context
        .messages
        .iter()
        .filter_map(|value| value.get("content"))
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn explicit_context_edges_order_projection() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("g", Some("."), None).await.unwrap();
    mgr.append_with_id("g", &message("a", None, "first"))
        .await
        .unwrap();
    mgr.append_with_id("g", &message("b", None, "second"))
        .await
        .unwrap();
    mgr.append_with_id("g", &message("head", None, "answer"))
        .await
        .unwrap();
    mgr.append_edge("g", &edge("e2", "b", "head", 1, true, 0))
        .await
        .unwrap();
    mgr.append_edge("g", &edge("e1", "a", "head", 0, true, 0))
        .await
        .unwrap();
    mgr.set_leaf("g", Some("head".to_string()));

    let context = mgr.build_session_context("g").await.unwrap();
    let text = context_text(&context);
    let first = text.find("first").expect("first");
    let second = text.find("second").expect("second");
    let answer = text.find("answer").expect("answer");
    assert!(first < second && second < answer, "order: {text}");
}

#[tokio::test]
async fn tombstone_edge_excludes_branch_from_context() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("g", Some("."), None).await.unwrap();
    mgr.append_with_id("g", &message("a", None, "first"))
        .await
        .unwrap();
    mgr.append_with_id("g", &message("head", None, "answer"))
        .await
        .unwrap();
    mgr.append_edge("g", &edge("e1", "a", "head", 0, true, 0))
        .await
        .unwrap();
    mgr.append_edge("g", &edge("e2", "a", "head", 0, false, 1))
        .await
        .unwrap();
    mgr.set_leaf("g", Some("head".to_string()));

    let context = mgr.build_session_context("g").await.unwrap();
    let text = context_text(&context);
    assert!(!text.contains("first"), "tombstone must exclude: {text}");
    assert!(text.contains("answer"));
}

#[tokio::test]
async fn parent_chain_is_derived_as_parent_edges() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("g", Some("."), None).await.unwrap();
    mgr.append_with_id("g", &message("a", None, "first"))
        .await
        .unwrap();
    mgr.append_with_id("g", &message("b", Some("a"), "second"))
        .await
        .unwrap();
    mgr.set_leaf("g", Some("b".to_string()));

    let context = mgr.build_session_context("g").await.unwrap();
    let text = context_text(&context);
    assert!(text.contains("first") && text.contains("second"), "{text}");
}

#[tokio::test]
async fn fork_records_branch_and_fork_edge() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("parent", Some("."), None).await.unwrap();
    mgr.append_with_id("parent", &message("u1", None, "root"))
        .await
        .unwrap();
    mgr.append_with_id("parent", &message("a1", Some("u1"), "answer"))
        .await
        .unwrap();
    mgr.fork("parent", "child", "u1", ForkPosition::At)
        .await
        .unwrap();

    let entries = mgr.load("child").await.unwrap();
    let edges = mgr.read_edges("child", &entries).await.unwrap();
    assert!(
        edges
            .iter()
            .any(|edge| edge.edge_kind == EdgeKind::Fork && edge.from == "u1"),
        "fork edge must be recorded: {edges:?}"
    );
    if mgr.manifest_path("child").exists() {
        let manifest = mgr.read_manifest("child").await.unwrap();
        assert_eq!(manifest.branches.len(), 1);
        assert_eq!(
            manifest.branches[0].parent_session.as_deref(),
            Some("parent")
        );
    }
}
