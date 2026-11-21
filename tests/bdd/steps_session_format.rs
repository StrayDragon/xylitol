use crate::bdd::fixtures::*;
use crate::bdd::prelude::*;
use rstest_bdd_macros::{then, when};
use xylitol_official_plugin_session::{
    CustomEntry, CustomMessageEntry, EntryBase, LabelEntry, MessageEntry, ModelChangeEntry,
    SessionEntry, SessionInfoEntry, ThinkingLevelChangeEntry,
};
use xylitol_plugin_sdk::XySessionStore as _;
use xylitol_plugin_sdk::session::{SESSION_VERSION, SessionHeader};

fn sessions_root(sess: &XySessionStore) -> std::path::PathBuf {
    sess.sessions_dir
        .borrow()
        .clone()
        .expect("sessions dir captured")
}

fn base(id: &str, parent: Option<String>) -> EntryBase {
    EntryBase {
        entry_type: String::new(),
        id: id.to_string(),
        parent_id: parent,
        timestamp: 1_700_000_000_000,
    }
}

#[when("目录中植入版本号低于当前的会话文件")]
fn when_plant_lower_version_session(sess: &XySessionStore) {
    sess.ensure_mgr();
    let log_dir = sessions_root(sess).join("old-version").join("log");
    std::fs::create_dir_all(&log_dir).expect("create log dir");
    let header = serde_json::json!({
        "type": "session",
        "version": SESSION_VERSION - 1,
        "format": "log-v1",
        "id": "old-version",
        "timestamp": 1,
        "cwd": "."
    });
    std::fs::write(log_dir.join("000000.jsonl"), format!("{header}\n"))
        .expect("plant lower version session");
    sess.current_id.replace(Some("old-version".into()));
}

#[when("加载该会话")]
async fn when_load_that_session(sess: &XySessionStore) {
    let id = sess
        .current_id
        .borrow()
        .clone()
        .expect("planted session id");
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    match mgr.load_entries(&id).await {
        Ok(entries) => {
            sess.entries.replace(entries);
            sess.last_result.replace(None);
        }
        Err(error) => {
            sess.last_result
                .replace(Some(Err(XyDriverError::unsupported(error.to_string()))));
        }
    }
}

#[then("拒载错误含文件版本号")]
fn then_refusal_names_file_version(sess: &XySessionStore) {
    let result = sess.last_result.borrow();
    let Some(Err(error)) = result.as_ref() else {
        panic!("a lower-version session must be refused, got {:?}", result);
    };
    let message = error.to_string();
    assert!(
        message.contains(&format!("version {}", SESSION_VERSION - 1)),
        "refusal must name the file version: {message}"
    );
    assert!(
        message.contains("not supported")
            || message.contains(&format!("require {SESSION_VERSION}")),
        "refusal must name the current version: {message}"
    );
}

#[when("创建当前版本会话并追加各类型条目")]
async fn when_create_current_version_session(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    mgr.create("all-types", Some("."), None).await.unwrap();
    let mut leaf: Option<String> = None;
    let entries: Vec<SessionEntry> = vec![
        SessionEntry::Message(MessageEntry {
            base: base("m1", leaf.clone()),
            message: serde_json::json!({
                "role": "user",
                "content": [{"type": "text", "text": "hi"}]
            }),
        }),
        SessionEntry::ModelChange(ModelChangeEntry {
            base: base("mc1", leaf.clone()),
            provider: "openai".into(),
            model_id: "gpt".into(),
        }),
        SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            base: base("tl1", leaf.clone()),
            thinking_level: "high".into(),
        }),
        SessionEntry::Label(LabelEntry {
            base: base("lb1", leaf.clone()),
            label: Some("tag".into()),
            target_id: "m1".into(),
        }),
        SessionEntry::Custom(CustomEntry {
            base: base("cu1", leaf.clone()),
            custom_type: "note".into(),
            data: serde_json::json!({"k": 1}),
        }),
        SessionEntry::CustomMessage(CustomMessageEntry {
            base: base("cm1", leaf.clone()),
            custom_type: "note".into(),
            content: serde_json::json!("body"),
            display: true,
            details: None,
        }),
        SessionEntry::SessionInfo(SessionInfoEntry {
            base: base("si1", leaf.clone()),
            name: Some("all-types".into()),
        }),
        SessionEntry::ContextInjected(xylitol_plugin_sdk::session::ContextInjectedEntry {
            base: base("ci1", leaf.clone()),
            injector_id: "skills".into(),
            point: "turn_pre".into(),
            fingerprint: Some("fp".into()),
            preview: Some("prev".into()),
        }),
    ];
    for entry in entries {
        let id = entry.entry_id().unwrap_or_default().to_string();
        mgr.append_session_entry("all-types", &entry).await.unwrap();
        leaf = Some(id);
    }
    let _ = leaf;
    sess.current_id.replace(Some("all-types".into()));
}

#[then("全部条目完整解析无丢失")]
fn then_all_entries_parse(sess: &XySessionStore) {
    let entries = sess.entries.borrow();
    let types: Vec<&str> = entries.iter().map(|e| e.entry_type()).collect();
    for expected in [
        "session",
        "message",
        "modelChange",
        "thinkingLevelChange",
        "label",
        "custom",
        "customMessage",
        "sessionInfo",
        "contextInjected",
    ] {
        assert!(
            types.contains(&expected),
            "entry type {expected} must round-trip: {types:?}"
        );
    }
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.entry_type() != "session")
            .count(),
        8,
        "exactly the appended entries must load back: {types:?}"
    );
    let header_version = entries
        .iter()
        .find_map(|e| match e {
            SessionEntry::Header(SessionHeader { version, .. }) => Some(*version),
            _ => None,
        })
        .expect("header");
    assert_eq!(header_version, SESSION_VERSION);
}
