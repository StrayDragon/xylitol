use std::collections::BTreeMap;
use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xylitol_plugin_sdk::session::{CustomEntry, EntryBase, SessionEntry};
use xylitol_plugin_sdk::session_env::CUSTOM_TYPE_CONTEXT_FINGERPRINT;
use xylitol_plugin_sdk::{ToolFingerprint, XySessionStore};

use crate::config::{DedupMode, InjectorPoint, Placement};
use crate::registry::{InjectorContext, NativeInjector};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FingerprintSnapshot {
    pub skills: BTreeMap<String, String>,
    pub tools: Option<ToolFingerprint>,
    pub prepare: Option<String>,
}

pub struct DriftInjector {
    id: String,
}

impl Default for DriftInjector {
    fn default() -> Self {
        Self::new("drift")
    }
}

impl DriftInjector {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl NativeInjector for DriftInjector {
    fn id(&self) -> &str {
        &self.id
    }

    fn point(&self) -> InjectorPoint {
        InjectorPoint::TurnPre
    }

    fn placement(&self) -> Placement {
        Placement::HistoryRow
    }

    fn dedup(&self) -> DedupMode {
        DedupMode::SuppressIfUnchanged
    }

    fn group(&self) -> Option<&str> {
        Some("system-reminder")
    }

    async fn produce(&self, ctx: &InjectorContext<'_>) -> Option<String> {
        let session_id = ctx.session_id?;
        let store = ctx.store?;
        let current = FingerprintSnapshot {
            skills: ctx
                .before
                .skills
                .iter()
                .map(|skill| (skill.name.clone(), hash_file(&skill.source_info.path)))
                .collect(),
            tools: ctx.before.tools_fingerprint.clone(),
            prepare: ctx.before.prepare_fingerprint.clone(),
        };
        let previous = read_snapshot(store, session_id).await;
        let Some(previous) = previous else {
            write_snapshot(store, session_id, &current).await;
            return None;
        };
        let lines = diff(&previous, &current);
        if lines.is_empty() {
            return None;
        }
        write_snapshot(store, session_id, &current).await;
        Some(lines.join("\n"))
    }
}

fn hash_file(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex_lower(hasher.finalize())
}

fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn short(hash: &str) -> String {
    hash.chars().take(4).collect()
}

async fn read_snapshot(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Option<FingerprintSnapshot> {
    let entries = store.load_leaf_branch(session_id).await.ok()?;
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_CONTEXT_FINGERPRINT => {
            serde_json::from_value(custom.data.clone()).ok()
        }
        _ => None,
    })
}

async fn write_snapshot(
    store: &dyn XySessionStore,
    session_id: &str,
    snapshot: &FingerprintSnapshot,
) {
    let entry = SessionEntry::Custom(CustomEntry {
        base: EntryBase {
            entry_type: "custom".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        custom_type: CUSTOM_TYPE_CONTEXT_FINGERPRINT.into(),
        data: serde_json::to_value(snapshot).unwrap_or_default(),
    });
    if let Err(error) = store.append_session_entry(session_id, &entry).await {
        log::warn!(
            target: "xylitol::context_inject",
            "drift snapshot append failed session={session_id} error={error}"
        );
    }
}

fn diff(previous: &FingerprintSnapshot, current: &FingerprintSnapshot) -> Vec<String> {
    let mut lines = Vec::new();
    for (name, hash) in &current.skills {
        match previous.skills.get(name) {
            Some(old) if old != hash => lines.push(format!(
                "[skills] skill \"{name}\" 内容已变更（{} → {}），本轮展开将更新；此前对话中对 {name} 的引用可能已过期",
                short(old),
                short(hash)
            )),
            None => lines.push(format!("[skills] skill \"{name}\" 新增（{}）", short(hash))),
            _ => {}
        }
    }
    for name in previous.skills.keys() {
        if !current.skills.contains_key(name) {
            lines.push(format!("[skills] skill \"{name}\" 已移除"));
        }
    }
    if previous.tools != current.tools {
        let old_digest = previous
            .tools
            .as_ref()
            .map(|tools| short(&tools.content_digest))
            .unwrap_or_else(|| "-".into());
        let new_digest = current
            .tools
            .as_ref()
            .map(|tools| short(&tools.content_digest))
            .unwrap_or_else(|| "-".into());
        let removed: Vec<&str> = match (&previous.tools, &current.tools) {
            (Some(old), Some(new)) => old
                .names
                .iter()
                .filter(|name| !new.names.contains(name))
                .map(String::as_str)
                .collect(),
            (Some(old), None) => old.names.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        };
        let suffix = if removed.is_empty() {
            String::new()
        } else {
            format!("：禁用 {}", removed.join(" / "))
        };
        lines.push(format!(
            "[tools] 工具清单已变更（{old_digest} → {new_digest}）{suffix}"
        ));
    }
    if previous.prepare != current.prepare {
        lines.push("[prepare] 会话准备已变更".into());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(
        skills: &[(&str, &str)],
        tools: Option<&str>,
        prepare: Option<&str>,
    ) -> FingerprintSnapshot {
        FingerprintSnapshot {
            skills: skills
                .iter()
                .map(|(name, hash)| ((*name).into(), (*hash).into()))
                .collect(),
            tools: tools.map(|digest| ToolFingerprint {
                names: vec!["read".into()],
                content_digest: digest.into(),
            }),
            prepare: prepare.map(str::to_string),
        }
    }

    #[test]
    fn hash_file_rehashes_every_call() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("B.md");
        std::fs::write(&path, "B").expect("write");
        let first = hash_file(&path);
        let second = hash_file(&path);
        assert_eq!(first, second);
        std::fs::write(&path, "BB").expect("write");
        let third = hash_file(&path);
        assert_ne!(third, first, "changed content must rehash");
    }

    #[test]
    fn unchanged_snapshot_produces_no_lines() {
        let a = snapshot(&[("B", "aaa")], Some("t1"), Some("p1"));
        assert!(diff(&a, &a).is_empty());
    }

    #[test]
    fn skill_change_produces_a_reminder() {
        let previous = snapshot(&[("B", "3f2a1111")], Some("t1"), None);
        let current = snapshot(&[("B", "91c4ffff")], Some("t1"), None);
        let lines = diff(&previous, &current);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("[skills] skill \"B\""));
        assert!(lines[0].contains("3f2a"));
        assert!(lines[0].contains("91c4"));
    }

    #[test]
    fn tool_change_lists_removed_tools() {
        let previous = snapshot(&[], Some("8d10aaaa"), None);
        let mut current = snapshot(&[], Some("bb37bbbb"), None);
        current.tools = Some(ToolFingerprint {
            names: vec!["read".into(), "write".into()],
            content_digest: "bb37bbbb".into(),
        });
        let lines = diff(&previous, &current);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("[tools]"));
        assert!(lines[0].contains("8d10"));
        assert!(lines[0].contains("bb37"));
    }

    #[test]
    fn prepare_change_is_reported() {
        let previous = snapshot(&[], None, Some("p1"));
        let current = snapshot(&[], None, Some("p2"));
        assert_eq!(diff(&previous, &current), vec!["[prepare] 会话准备已变更"]);
    }
}
