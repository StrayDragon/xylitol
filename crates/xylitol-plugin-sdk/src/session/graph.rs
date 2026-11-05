use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::llm_project::project_for_llm;
use crate::message::{AgentMessage, XyLlmMessage};
use crate::session::entries::SessionEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    Parent,
    Context,
    Fork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeRecord {
    pub id: String,
    pub edge_kind: EdgeKind,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub order: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub timestamp: u64,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRef {
    pub backend: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_entry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_entry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_ref: Option<WorkspaceRef>,
}

#[derive(Debug, Clone)]
pub struct ContextView {
    pub nodes: Vec<SessionEntry>,
    pub messages: Vec<AgentMessage>,
    pub llm_messages: Vec<XyLlmMessage>,
    pub epoch_id: Option<String>,
    pub prefix_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    #[error("session graph cycle at edge {from} -> {to}")]
    Cycle { from: String, to: String },
    #[error("session graph head {0} is not present")]
    MissingHead(String),
    #[error("session graph edge references missing node {0}")]
    MissingNode(String),
}

fn newer(candidate: &EdgeRecord, current: &EdgeRecord) -> bool {
    candidate
        .timestamp
        .cmp(&current.timestamp)
        .then(candidate.id.cmp(&current.id))
        .is_gt()
}

fn is_epoch(entry: &SessionEntry) -> bool {
    matches!(entry, SessionEntry::Compaction(_))
}

pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                let next = if key == "timestamp" {
                    Value::from(0)
                } else {
                    canonicalize(&map[key])
                };
                out.insert(key.clone(), next);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

pub fn prefix_hash(messages: &[XyLlmMessage]) -> String {
    let value = serde_json::to_value(messages).unwrap_or(Value::Null);
    let canonical = canonicalize(&value);
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn build_context(
    head: &str,
    entries: &[SessionEntry],
    edges: &[EdgeRecord],
    done_bash_ids: &HashSet<String>,
) -> Result<ContextView, GraphError> {
    let mut nodes: HashMap<&str, &SessionEntry> = HashMap::new();
    for entry in entries {
        if let Some(id) = entry.entry_id() {
            nodes.insert(id, entry);
        }
    }
    if !nodes.contains_key(head) {
        return Err(GraphError::MissingHead(head.to_string()));
    }

    let mut latest: HashMap<(&str, &str), &EdgeRecord> = HashMap::new();
    for edge in edges {
        let key = (edge.from.as_str(), edge.to.as_str());
        match latest.get(&key) {
            Some(current) if !newer(edge, current) => {}
            _ => {
                latest.insert(key, edge);
            }
        }
    }

    let mut incoming: HashMap<&str, Vec<&EdgeRecord>> = HashMap::new();
    for edge in latest.values().filter(|edge| edge.enabled) {
        if !nodes.contains_key(edge.from.as_str()) {
            return Err(GraphError::MissingNode(edge.from.clone()));
        }
        if !nodes.contains_key(edge.to.as_str()) {
            return Err(GraphError::MissingNode(edge.to.clone()));
        }
        incoming.entry(edge.to.as_str()).or_default().push(edge);
    }
    for list in incoming.values_mut() {
        list.sort_by(|a, b| {
            a.order
                .cmp(&b.order)
                .then(a.timestamp.cmp(&b.timestamp))
                .then(a.id.cmp(&b.id))
        });
    }

    let mut ordered: Vec<String> = Vec::new();
    let mut visiting: HashSet<String> = HashSet::new();
    let mut done: HashSet<String> = HashSet::new();
    visit(
        head,
        &nodes,
        &incoming,
        &mut visiting,
        &mut done,
        &mut ordered,
    )?;

    let epoch_idx = ordered
        .iter()
        .rposition(|id| nodes.get(id.as_str()).is_some_and(|entry| is_epoch(entry)));
    let start = epoch_idx.unwrap_or(0);
    let slice = &ordered[start..];

    let mut messages = Vec::new();
    let mut selected = Vec::with_capacity(slice.len());
    for id in slice {
        let Some(entry) = nodes.get(id.as_str()) else {
            continue;
        };
        if let Some(message) = entry.as_agent_message() {
            messages.push(message);
        }
        selected.push((*entry).clone());
    }
    let messages = crate::session::fold_interrupted_bash_rows(messages, done_bash_ids);
    let projected = project_for_llm(&messages);
    let hash = prefix_hash(&projected);
    Ok(ContextView {
        nodes: selected,
        messages,
        llm_messages: projected,
        epoch_id: epoch_idx.map(|index| ordered[index].clone()),
        prefix_hash: hash,
    })
}

fn visit(
    node: &str,
    nodes: &HashMap<&str, &SessionEntry>,
    incoming: &HashMap<&str, Vec<&EdgeRecord>>,
    visiting: &mut HashSet<String>,
    done: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) -> Result<(), GraphError> {
    if done.contains(node) {
        return Ok(());
    }
    if !visiting.insert(node.to_string()) {
        return Err(GraphError::Cycle {
            from: node.to_string(),
            to: node.to_string(),
        });
    }
    if let Some(edges) = incoming.get(node) {
        for edge in edges {
            if visiting.contains(edge.from.as_str()) {
                return Err(GraphError::Cycle {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                });
            }
            visit(&edge.from, nodes, incoming, visiting, done, ordered)?;
        }
    }
    visiting.remove(node);
    done.insert(node.to_string());
    if nodes.contains_key(node) {
        ordered.push(node.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::session::{EntryBase, MessageEntry};

    fn message(id: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            message: json!({
                "role": "user",
                "content": [{"type": "text", "text": text}],
                "timestamp": 0
            }),
        })
    }

    fn edge(id: &str, from: &str, to: &str, order: u32, enabled: bool) -> EdgeRecord {
        EdgeRecord {
            id: id.into(),
            edge_kind: EdgeKind::Context,
            from: from.into(),
            to: to.into(),
            order,
            enabled,
            timestamp: 0,
        }
    }

    #[test]
    fn orders_multi_parent_edges_by_order() {
        let entries = vec![
            message("a", "first"),
            message("b", "second"),
            message("head", "answer"),
        ];
        let edges = vec![
            edge("e2", "b", "head", 1, true),
            edge("e1", "a", "head", 0, true),
        ];
        let view = build_context("head", &entries, &edges, &HashSet::new()).expect("graph");
        let ids: Vec<_> = view.nodes.iter().filter_map(|n| n.entry_id()).collect();
        assert_eq!(ids, vec!["a", "b", "head"]);
        assert_eq!(view.messages.len(), 3);
    }

    #[test]
    fn tombstone_edge_excludes_branch() {
        let entries = vec![message("a", "first"), message("head", "answer")];
        let edges = vec![
            edge("e1", "a", "head", 0, true),
            edge("e2", "a", "head", 0, false),
        ];
        let view = build_context("head", &entries, &edges, &HashSet::new()).expect("graph");
        let ids: Vec<_> = view.nodes.iter().filter_map(|n| n.entry_id()).collect();
        assert_eq!(ids, vec!["head"]);
    }

    #[test]
    fn cycle_is_reported() {
        let entries = vec![message("a", "a"), message("b", "b")];
        let edges = vec![edge("e1", "a", "b", 0, true), edge("e2", "b", "a", 0, true)];
        let err = build_context("a", &entries, &edges, &HashSet::new()).expect_err("cycle");
        assert!(matches!(err, GraphError::Cycle { .. }));
    }

    #[test]
    fn prefix_hash_is_stable_when_tail_is_appended() {
        let entries = vec![
            SessionEntry::Compaction(crate::session::CompactionEntry {
                base: EntryBase {
                    entry_type: "compaction".into(),
                    id: "epoch".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                summary: "summary".into(),
                first_kept_entry_id: "a".into(),
                tokens_before: 10,
                details: None,
                from_hook: None,
                policy: None,
            }),
            message("a", "first"),
            message("b", "second"),
        ];
        let edges = vec![
            edge("e1", "epoch", "a", 0, true),
            edge("e2", "a", "b", 0, true),
        ];
        let first = build_context("b", &entries, &edges, &HashSet::new()).expect("first");
        let tail = message("c", "third");
        let mut next_entries = entries.clone();
        next_entries.push(tail);
        let mut next_edges = edges.clone();
        next_edges.push(edge("e3", "b", "c", 0, true));
        let second =
            build_context("c", &next_entries, &next_edges, &HashSet::new()).expect("second");
        assert_eq!(second.epoch_id.as_deref(), Some("epoch"));
        assert_eq!(
            first.prefix_hash,
            prefix_hash(&second.llm_messages[..first.llm_messages.len()])
        );
    }
}
