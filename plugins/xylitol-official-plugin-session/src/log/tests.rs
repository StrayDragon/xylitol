use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use super::LogSessionStore;
use super::append::{append, create, read_manifest, write_manifest};
use super::page::{PageToken, load_last_turns, load_page};
use super::project::{build_context_entries, build_session_context, load_all, load_branch};
use crate::types::{EntryBase, MessageEntry, SessionEntry};
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::session::{ForkPosition, message_text};

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

fn reminder_row(id: &str, parent: Option<&str>, body: &str) -> SessionEntry {
    SessionEntry::CustomMessage(crate::types::CustomMessageEntry {
        base: EntryBase {
            entry_type: "customMessage".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: ts(id),
        },
        custom_type: "context_reminder".into(),
        content: json!(body),
        display: true,
        details: None,
    })
}

#[tokio::test]
async fn fork_branch_keeps_context_reminder_in_ancestor_chain() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(
        &store,
        "s",
        &[
            reminder_row(
                "r1",
                None,
                "<system-reminder>[skills] skill B 已变更</system-reminder>",
            ),
            msg("u1", Some("r1"), "user", "one"),
            msg("a1", Some("u1"), "assistant", "two"),
        ],
    )
    .await;
    store
        .fork("s", "s_fork_child", "u1", ForkPosition::Before)
        .await
        .expect("fork at the user message entry");
    let branch = store
        .load_leaf_branch("s_fork_child")
        .await
        .expect("fork branch");
    assert!(
        branch.iter().any(|entry| matches!(
            entry,
            SessionEntry::CustomMessage(custom)
                if custom.custom_type == "context_reminder"
        )),
        "context_reminder row must stay in the fork branch ancestor chain: {branch:?}"
    );
    let ids: Vec<_> = branch.iter().filter_map(|entry| entry.entry_id()).collect();
    assert!(
        ids.contains(&"r1"),
        "reminder row id must ride the ancestor chain: {ids:?}"
    );
    assert!(
        !ids.contains(&"u1"),
        "ForkPosition::Before must drop the forked-at user message: {ids:?}"
    );
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

async fn run_golden(name: &str, known: &HashSet<String>) -> Value {
    let golden = load_golden(name);
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    let entries = entries_from_value(&golden["entries"]);
    let session_id = match name {
        "linear" => "s_linear",
        "compaction" => "s_comp",
        "travel" => "s_travel",
        "fork" => "s_fork_parent",
        other => panic!("unknown golden {other}"),
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
    capture(&store, target, known, before).await
}

async fn assert_golden(name: &str, known: &HashSet<String>) {
    let golden = load_golden(name);
    let actual = run_golden(name, known).await;
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
#[ignore = "regenerates golden fixtures; run explicitly: cargo test -p xylitol-official-plugin-session -- --ignored golden_regenerate"]
async fn golden_regenerate() {
    let fixtures: [(&str, HashSet<String>); 4] = [
        ("linear", known(&["u1", "a1", "u2", "a2", "s_linear"])),
        (
            "compaction",
            known(&["u1", "a1", "u2", "a2", "u3", "a3", "s_comp"]),
        ),
        ("travel", known(&["u1", "a1", "u2", "a2", "s_travel"])),
        (
            "fork",
            known(&[
                "u1",
                "a_left",
                "a_right",
                "u_right",
                "s_fork_parent",
                "s_fork_child",
            ]),
        ),
    ];
    for (name, known_ids) in fixtures {
        let mut golden = load_golden(name);
        golden["expected"] = run_golden(name, &known_ids).await;
        let path = golden_dir().join(format!("{name}.json"));
        let body = serde_json::to_string_pretty(&golden).expect("serialize golden");
        std::fs::write(&path, body + "\n").expect("write golden");
    }
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
async fn partial_tail_is_repaired_on_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let path = store.layout.log_file("s", 1);
    let mut content = tokio::fs::read_to_string(&path).await.expect("read");
    content.push_str("{\"type\":\"message\",\"seq\":9,\"id\":\"half\"");
    tokio::fs::write(&path, &content).await.expect("write");
    let entries = load_all(&store, "s").await.expect("load");
    assert!(entries.iter().all(|entry| entry.entry_id() != Some("half")));
    let bytes = tokio::fs::read(&path).await.expect("read");
    assert_eq!(
        bytes,
        content.as_bytes(),
        "read path must not rewrite the log"
    );
    append(
        &store,
        "s",
        &msg("a1", Some("u1"), "assistant", "done"),
        true,
    )
    .await
    .expect("append");
    let entries = load_all(&store, "s").await.expect("load");
    assert!(entries.iter().all(|entry| entry.entry_id() != Some("half")));
    assert!(entries.iter().any(|entry| entry.entry_id() == Some("a1")));
    let bytes = tokio::fs::read(&path).await.expect("read");
    assert!(bytes.ends_with(b"\n"));
}

#[tokio::test]
async fn complete_tail_without_terminator_is_preserved_on_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let path = store.layout.log_file("s", 1);
    let mut value =
        serde_json::to_value(msg("a1", Some("u1"), "assistant", "in flight")).expect("json");
    value
        .as_object_mut()
        .expect("object")
        .insert("seq".into(), json!(3));
    let line = serde_json::to_string(&value).expect("line");
    let mut content = tokio::fs::read_to_string(&path).await.expect("read");
    content.push_str(&line);
    tokio::fs::write(&path, content).await.expect("write");
    append(
        &store,
        "s",
        &msg("a2", Some("a1"), "assistant", "next"),
        true,
    )
    .await
    .expect("append");
    let entries = load_all(&store, "s").await.expect("load");
    let ids: Vec<_> = entries
        .iter()
        .filter_map(|entry| entry.entry_id())
        .collect();
    assert_eq!(ids, vec!["u1", "a1", "a2"]);
}

#[tokio::test]
async fn projection_keeps_rows_without_entry_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let path = store.layout.log_file("s", 1);
    let mut content = tokio::fs::read_to_string(&path).await.expect("read");
    for (seq, text) in [(3u64, "first"), (4u64, "second")] {
        let mut value = serde_json::to_value(msg("", None, "user", text)).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("seq".into(), json!(seq));
        content.push_str(&serde_json::to_string(&value).expect("line"));
        content.push('\n');
    }
    tokio::fs::write(&path, content).await.expect("write");
    let entries = load_all(&store, "s").await.expect("load");
    let texts: Vec<_> = entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::Message(message) => Some(message_text(&message.message)),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["hello", "first", "second"]);
}

#[tokio::test]
async fn concurrent_reader_does_not_lose_appends() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(LogSessionStore::new(dir.path().join("sessions")));
    create(&store, "s", Some("."), None).await.expect("create");
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let store = store.clone();
        let stop = stop.clone();
        tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                let _ = load_all(&store, "s").await;
                tokio::task::yield_now().await;
            }
        })
    };
    for idx in 0..40u32 {
        let entry = SessionEntry::Custom(crate::types::CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: format!("e{idx}"),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: "probe".into(),
            data: json!({ "idx": idx }),
        });
        append(&store, "s", &entry, true).await.expect("append");
    }
    stop.store(true, Ordering::Relaxed);
    reader.await.expect("reader");
    let entries = load_all(&store, "s").await.expect("load");
    for idx in 0..40u32 {
        let id = format!("e{idx}");
        assert!(
            entries
                .iter()
                .any(|entry| entry.entry_id() == Some(id.as_str())),
            "append {id} lost under a concurrent reader"
        );
    }
}

#[tokio::test]
async fn session_name_reads_back_right_after_create() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    store.create("s", Some("."), None).await.expect("create");
    store
        .set_session_name("s", " remote name\n")
        .await
        .expect("set name");
    let fetched = store.get_session_name("s").await.expect("get name");
    assert_eq!(fetched.as_deref(), Some("remote name"));
}

#[tokio::test]
async fn load_all_does_not_truncate_in_progress_tail() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    seed(&store, "s", &[msg("u1", None, "user", "hello")]).await;
    let path = store.layout.log_file("s", 1);
    let mut value =
        serde_json::to_value(msg("a1", Some("u1"), "assistant", "mid-write")).expect("json");
    value
        .as_object_mut()
        .expect("object")
        .insert("seq".into(), json!(3));
    let line = serde_json::to_string(&value).expect("line");
    let mut bytes = tokio::fs::read(&path).await.expect("read");
    bytes.extend_from_slice(line.as_bytes());
    tokio::fs::write(&path, bytes).await.expect("write");
    let _ = load_all(&store, "s").await.expect("load");
    let after = tokio::fs::read(&path).await.expect("read");
    assert!(
        after.ends_with(line.as_bytes()),
        "read path must not truncate a complete in-progress tail"
    );
    let mut bytes = tokio::fs::read(&path).await.expect("read");
    bytes.push(b'\n');
    tokio::fs::write(&path, bytes).await.expect("write");
    let entries = load_all(&store, "s").await.expect("load");
    assert!(
        entries.iter().any(|entry| entry.entry_id() == Some("a1")),
        "terminated line must be visible"
    );
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
async fn manifest_flushes_at_turn_boundary_not_per_entry() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = LogSessionStore::new(dir.path().join("sessions"));
    create(&store, "s", Some("."), None).await.expect("create");
    let manifest_path = store.layout.manifest_path("s");
    let before = tokio::fs::read(&manifest_path).await.expect("manifest");
    for idx in 0..3 {
        let entry = SessionEntry::Custom(crate::types::CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: "probe".into(),
            data: json!({ "idx": idx }),
        });
        append(&store, "s", &entry, true).await.expect("append");
    }
    let during = tokio::fs::read(&manifest_path).await.expect("manifest");
    assert_eq!(
        before, during,
        "non-turn-end appends must not rewrite the manifest"
    );
    append(&store, "s", &msg("u1", None, "user", "hello"), true)
        .await
        .expect("append user");
    let after = tokio::fs::read(&manifest_path).await.expect("manifest");
    assert_ne!(during, after, "turn boundary must flush the manifest");
    let manifest: Value = serde_json::from_slice(&after).expect("manifest json");
    assert_eq!(manifest["lineCount"].as_u64(), Some(5));
    assert_eq!(manifest["nextSeq"].as_u64(), Some(6));
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
