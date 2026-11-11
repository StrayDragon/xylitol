use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::LogSessionStore;
use super::append::{append, create, read_manifest, write_manifest};
use super::maintain::quick_check;
use super::page::{PageToken, load_last_turns, load_page};
use super::project::{build_context_entries, build_session_context, load_all, load_branch};
use crate::types::{EntryBase, MessageEntry, SessionEntry};
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::session::ForkPosition;

fn ts(id: &str) -> u64 {
    1_760_000_000_000u64
        + id.as_bytes()
            .iter()
            .fold(0u64, |acc, byte| acc * 31 + *byte as u64)
            % 1_000_000
}

fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: ts(id),
        },
        message: xylitol_plugin_sdk::session::fixture_message_json(role, text),
    })
}

fn normalize_entries(entries: &[SessionEntry], known: &HashSet<String>) -> Value {
    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut next = 0usize;
    let mut out = Vec::new();
    for entry in entries {
        let mut value = serde_json::to_value(entry).expect("serialize entry");
        let assign = |id: &str,
                      map: &mut std::collections::HashMap<String, String>,
                      next: &mut usize|
         -> String {
            if known.contains(id) {
                return id.to_string();
            }
            map.entry(id.to_string())
                .or_insert_with(|| {
                    *next += 1;
                    format!("gen-{next}")
                })
                .clone()
        };
        if let Some(obj) = value.as_object_mut() {
            obj.insert("timestamp".into(), json!(0));
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()).map(str::to_string) {
                let mapped = assign(&id, &mut map, &mut next);
                obj.insert("id".into(), json!(mapped));
            }
            if let Some(parent) = obj
                .get("parentId")
                .and_then(|v| v.as_str())
                .map(str::to_string)
            {
                let mapped = assign(&parent, &mut map, &mut next);
                obj.insert("parentId".into(), json!(mapped));
            }
            for key in ["targetId", "fromId"] {
                if let Some(id) = obj.get(key).and_then(|v| v.as_str()).map(str::to_string) {
                    let mapped = assign(&id, &mut map, &mut next);
                    obj.insert(key.into(), json!(mapped));
                }
            }
        }
        out.push(value);
    }
    Value::Array(out)
}

fn normalize_context(ctx: &xylitol_plugin_sdk::session::SessionContext) -> Value {
    json!({
        "thinking_level": ctx.thinking_level,
        "model": ctx.model,
        "messages": ctx.messages,
    })
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("session-log")
}

fn load_golden(name: &str) -> Value {
    let path = golden_dir().join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read golden {}: {error}", path.display()));
    serde_json::from_str(&raw).expect("parse golden")
}

fn entries_from_value(value: &Value) -> Vec<SessionEntry> {
    serde_json::from_value(value.clone()).expect("golden entries")
}

async fn seed(store: &LogSessionStore, id: &str, entries: &[SessionEntry]) {
    create(store, id, Some("."), None).await.expect("create");
    for entry in entries {
        append(store, id, entry, true).await.expect("append");
    }
}

async fn capture(
    store: &LogSessionStore,
    id: &str,
    known: &HashSet<String>,
    before: Option<&str>,
) -> Value {
    let all = load_all(store, id).await.expect("all");
    let branch = load_branch(store, id, None).await.expect("branch");
    let context = build_context_entries(store, id).await.expect("context");
    let ctx_full = build_session_context(store, id).await.expect("ctx");
    let page_tail = store
        .load_entries_page(id, Some(2), None)
        .await
        .expect("page tail");
    let page_before = match before {
        Some(before) => {
            let page = store
                .load_entries_page(id, Some(2), Some(before))
                .await
                .expect("page before");
            json!({
                "entries": normalize_entries(&page.0, known),
                "more": page.1,
            })
        }
        None => Value::Null,
    };
    json!({
        "all": normalize_entries(&all, known),
        "branch": normalize_entries(&branch, known),
        "context": normalize_entries(&context, known),
        "context_full": normalize_context(&ctx_full),
        "page_tail": {
            "entries": normalize_entries(&page_tail.0, known),
            "more": page_tail.1,
        },
        "page_before": page_before,
    })
}

async fn assert_golden(name: &str, known: &HashSet<String>) {
    let golden = load_golden(name);
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    let entries = entries_from_value(&golden["entries"]);
    let scenario = golden["scenario"].as_str().unwrap_or(name);
    let session_id = match name {
        "linear" => "s_linear",
        "compaction" => "s_comp",
        "travel" => "s_travel",
        "fork" => "s_fork_parent",
        other => panic!("unknown golden {other} {scenario}"),
    };
    seed(&store, session_id, &entries).await;
    if name == "travel" {
        LogSessionStore::set_leaf(&store, session_id, Some("a1".to_string()));
    }
    if name == "fork" {
        store
            .fork("s_fork_parent", "s_fork_child", "u_right", ForkPosition::At)
            .await
            .expect("fork");
    }
    let target = if name == "fork" {
        "s_fork_child"
    } else {
        session_id
    };
    let before = match name {
        "linear" => Some("a2"),
        "compaction" => Some("a3"),
        _ => None,
    };
    let actual = capture(&store, target, known, before).await;
    let expected = &golden["expected"];
    for key in ["all", "branch", "context"] {
        assert_eq!(
            actual[key], expected[key],
            "golden {name} mismatch on {key}\nactual: {}\nexpected: {}",
            actual[key], expected[key]
        );
    }
    assert_eq!(
        actual["context_full"], expected["context_full"],
        "golden {name} mismatch on context_full"
    );
    assert_eq!(
        actual["page_tail"]["more"], expected["page_tail"]["more"],
        "golden {name} page_tail.more"
    );
    assert_eq!(
        actual["page_tail"]["entries"], expected["page_tail"]["entries"],
        "golden {name} page_tail.entries"
    );
    assert_eq!(
        actual["page_before"], expected["page_before"],
        "golden {name} page_before"
    );
}

fn known(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[tokio::test]
async fn golden_linear_projection_matches_old_store() {
    assert_golden("linear", &known(&["u1", "a1", "u2", "a2", "s_linear"])).await;
}

#[tokio::test]
async fn golden_compaction_projection_matches_old_store() {
    assert_golden(
        "compaction",
        &known(&["u1", "a1", "u2", "a2", "u3", "a3", "s_comp"]),
    )
    .await;
}

#[tokio::test]
async fn golden_travel_projection_matches_old_store() {
    assert_golden("travel", &known(&["u1", "a1", "u2", "a2", "s_travel"])).await;
}

#[tokio::test]
async fn golden_fork_projection_matches_old_store() {
    assert_golden(
        "fork",
        &known(&[
            "u1",
            "a_left",
            "a_right",
            "u_right",
            "s_fork_parent",
            "s_fork_child",
        ]),
    )
    .await;
}

#[tokio::test]
async fn partial_tail_is_truncated_on_open() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let path = store.layout.log_file("s", 1);
    let mut content = tokio::fs::read_to_string(&path).await.expect("read");
    content.push_str("{\"type\":\"message\",\"seq\":9,\"id\":\"half\"");
    tokio::fs::write(&path, content).await.expect("write");
    quick_check(&store, "s").await.expect("quick check");
    let entries = load_all(&store, "s").await.expect("load");
    assert!(entries.iter().all(|entry| entry.entry_id() != Some("half")));
    let bytes = tokio::fs::read(&path).await.expect("read");
    assert!(bytes.ends_with(b"\n"));
}

#[tokio::test]
async fn missing_manifest_is_rebuilt_from_log() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    tokio::fs::remove_file(store.layout.manifest_path("s"))
        .await
        .expect("remove manifest");
    let entries = load_all(&store, "s").await.expect("load");
    assert_eq!(entries.len(), 2);
    assert!(store.layout.manifest_path("s").exists());
}

#[tokio::test]
async fn rotation_keeps_branch_projection() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let mut manifest = read_manifest(&store, "s").await.expect("manifest");
    manifest.rotation.threshold_bytes = 1;
    write_manifest(&store, "s", &manifest).await.expect("write");
    append(
        &store,
        "s",
        &msg("a1", Some("u1"), "assistant", "answer"),
        true,
    )
    .await
    .expect("append");
    append(&store, "s", &msg("u2", Some("a1"), "user", "again"), true)
        .await
        .expect("append");
    append(
        &store,
        "s",
        &msg("a2", Some("u2"), "assistant", "done"),
        true,
    )
    .await
    .expect("append");
    let files = super::append::list_log_files(&store, "s")
        .await
        .expect("files");
    assert!(files.len() >= 2, "expected rotation, got {files:?}");
    let branch = load_branch(&store, "s", None).await.expect("branch");
    let ids: Vec<_> = branch.iter().filter_map(|entry| entry.entry_id()).collect();
    assert_eq!(ids, vec!["u1", "a1", "u2", "a2"]);
}

#[tokio::test]
async fn compaction_reemits_kept_tail() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    let base = [
        msg("u1", None, "user", "one"),
        msg("a1", Some("u1"), "assistant", "two"),
        msg("u2", Some("a1"), "user", "three"),
        msg("a2", Some("u2"), "assistant", "four"),
    ];
    seed(&store, "s", &base).await;
    let compaction = SessionEntry::Compaction(xylitol_plugin_sdk::session::CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: "c1".into(),
            parent_id: Some("a2".into()),
            timestamp: ts("c1"),
        },
        summary: "summary".into(),
        first_kept_entry_id: "u2".into(),
        tokens_before: 10,
        details: None,
        from_hook: None,
        policy: None,
    });
    store
        .commit_compaction("s", &compaction)
        .await
        .expect("compaction");
    let files = super::append::list_log_files(&store, "s")
        .await
        .expect("files");
    assert_eq!(files.len(), 2);
    let compaction_id = store.leaf_id("s").expect("compaction id");
    let branch = load_branch(&store, "s", None).await.expect("branch");
    let ids: Vec<_> = branch.iter().filter_map(|entry| entry.entry_id()).collect();
    assert_eq!(ids, vec!["u2", "a2", compaction_id.as_str()]);
    let all = load_all(&store, "s").await.expect("all");
    let ids: Vec<_> = all.iter().filter_map(|entry| entry.entry_id()).collect();
    assert_eq!(ids, vec!["u1", "a1", "u2", "a2", compaction_id.as_str()]);
}

#[tokio::test]
async fn page_token_returns_older_turns() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    let entries = [
        msg("u1", None, "user", "one"),
        msg("a1", Some("u1"), "assistant", "two"),
        msg("u2", Some("a1"), "user", "three"),
        msg("a2", Some("u2"), "assistant", "four"),
    ];
    seed(&store, "s", &entries).await;
    let last = load_last_turns(&store, "s", 1).await.expect("last turns");
    let ids: Vec<_> = last.iter().filter_map(|entry| entry.entry_id()).collect();
    assert_eq!(ids, vec!["u2", "a2"]);
    let token = PageToken::new("a2", "a2", "older", "turn", 1);
    let older = load_page(&store, "s", &token.encode())
        .await
        .expect("older page");
    let ids: Vec<_> = older.iter().filter_map(|entry| entry.entry_id()).collect();
    assert_eq!(ids, vec!["u1", "a1"]);
    let stale = PageToken {
        v: "log-v0".into(),
        ..token
    };
    let fallback = load_page(&store, "s", &stale.encode())
        .await
        .expect("fallback");
    let ids: Vec<_> = fallback
        .iter()
        .filter_map(|entry| entry.entry_id())
        .collect();
    assert_eq!(ids, vec!["u1", "a1", "u2", "a2"]);
}

#[cfg(unix)]
#[tokio::test]
async fn cross_process_write_lock_is_exclusive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    create(&store, "s", Some("."), None).await.expect("create");
    store.acquire_write_lock("s").expect("hold");
    let lock_path = store.layout.session_dir("s").join("session.lock");
    let script = format!(
        "import fcntl, sys\nf=open({path:?}, 'r+')\ntry:\n fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)\n sys.exit(0)\nexcept BlockingIOError:\n sys.exit(2)\n",
        path = lock_path
    );
    let status = std::process::Command::new("python3")
        .arg("-c")
        .arg(&script)
        .status()
        .expect("python3 flock probe");
    assert_eq!(status.code(), Some(2), "other process MUST be blocked");
    store.release_write_lock("s");
    let status = std::process::Command::new("python3")
        .arg("-c")
        .arg(&script)
        .status()
        .expect("python3 flock probe");
    assert_eq!(status.code(), Some(0), "lock must be released");
}

#[tokio::test]
async fn session_name_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    store.create("s", Some("."), None).await.expect("create");
    append(&store, "s", &msg("u1", None, "user", "hi"), true)
        .await
        .expect("append");
    let name = store
        .set_session_name("s", " remote name\n")
        .await
        .expect("set name");
    assert_eq!(name, "remote name");
    let fetched = store.get_session_name("s").await.expect("get name");
    assert_eq!(fetched.as_deref(), Some("remote name"));
}

#[tokio::test]
async fn maintenance_is_idempotent_and_heals_dirty_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sessions_dir = dir.path().join("sessions");
    let store = LogSessionStore::new(sessions_dir.clone());
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let orphan = store.layout.session_dir("s").join("orphan.tmp");
    tokio::fs::write(&orphan, "stale").await.expect("plant tmp");

    let first = super::maintain_sessions(sessions_dir.clone())
        .await
        .expect("maintain");
    assert_eq!(first.sessions.len(), 1);
    assert!(!orphan.exists(), "maintenance must clean .tmp files");

    let second = super::maintain_sessions(sessions_dir)
        .await
        .expect("maintain twice");
    assert_eq!(first.sessions[0].id, second.sessions[0].id);
    assert_eq!(first.sessions[0].files, second.sessions[0].files);
    assert_eq!(first.sessions[0].entries, second.sessions[0].entries);
    assert_eq!(
        first.sessions[0].suggested_threshold_bytes,
        second.sessions[0].suggested_threshold_bytes
    );
}
