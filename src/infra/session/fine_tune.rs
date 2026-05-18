use anyhow::Result;

use super::types::*;

/// Fine-tune operation to apply to a snapshot.
#[derive(Debug, Clone)]
pub enum FineTuneOp {
    /// Remove a conversation turn by index.
    RemoveTurn(usize),
    /// Deprecate a conversation turn by index.
    DeprecateTurn(usize),
    /// Remove a code summary by file key.
    RemoveCodeSummary(String),
    /// Add or update a tag.
    AddTag(String),
    /// Remove a tag.
    RemoveTag(String),
}

/// Result of a fine-tune editing session.
#[derive(Debug, Clone)]
pub struct FineTuneResult {
    pub modified: bool,
    pub applied_ops: Vec<String>,
}

/// Apply a series of fine-tune operations to a snapshot, producing a new
/// snapshot (immutable — edits always produce a new version).
pub fn apply_fine_tune(
    snapshot: &Snapshot,
    ops: &[FineTuneOp],
) -> Result<(Snapshot, FineTuneResult)> {
    let mut edited = snapshot.clone();
    let mut applied_ops = Vec::new();

    // Generate new ID.
    edited.id = uuid::Uuid::new_v4().to_string();
    edited.parent_snapshot_id = Some(snapshot.id.clone());

    for op in ops {
        match op {
            FineTuneOp::RemoveTurn(idx) => {
                if *idx < edited.conversation.len() {
                    edited.conversation.remove(*idx);
                    applied_ops.push(format!("removed turn {idx}"));
                }
            }
            FineTuneOp::DeprecateTurn(idx) => {
                if *idx < edited.conversation.len() {
                    edited.conversation[*idx].deprecated = true;
                    applied_ops.push(format!("deprecated turn {idx}"));
                }
            }
            FineTuneOp::RemoveCodeSummary(key) => {
                if edited
                    .project_cognition
                    .code_summaries
                    .remove(key)
                    .is_some()
                {
                    applied_ops.push(format!("removed code summary: {key}"));
                }
            }
            FineTuneOp::AddTag(tag) => {
                if !edited.meta.tags.contains(tag) {
                    edited.meta.tags.push(tag.clone());
                    applied_ops.push(format!("added tag: {tag}"));
                }
            }
            FineTuneOp::RemoveTag(tag) => {
                edited.meta.tags.retain(|t| t != tag);
                applied_ops.push(format!("removed tag: {tag}"));
            }
        }
    }

    let modified = !applied_ops.is_empty();
    Ok((
        edited,
        FineTuneResult {
            modified,
            applied_ops,
        },
    ))
}

/// Render a human-readable tree view of a snapshot's structure.
pub fn render_snapshot_tree(snapshot: &Snapshot) -> String {
    let mut lines = Vec::new();

    let short_id = if snapshot.id.len() > 8 {
        &snapshot.id[..8]
    } else {
        &snapshot.id
    };
    lines.push(format!("📸 Snapshot {short_id}"));
    lines.push(format!("├── created: {}", snapshot.created.millis()));
    lines.push(format!(
        "├── parent: {}",
        snapshot.parent_snapshot_id.as_deref().unwrap_or("(root)")
    ));
    lines.push(format!("├── model: {}", snapshot.meta.model_id));
    lines.push(format!("├── tags: [{}]", snapshot.meta.tags.join(", ")));
    lines.push(format!(
        "├── project: {}",
        snapshot.meta.project_root.display()
    ));
    lines.push(format!(
        "├── conversation: {} turns",
        snapshot.conversation.len()
    ));

    // Show first few conversation turns.
    for (i, turn) in snapshot.conversation.iter().enumerate().take(5) {
        let dep = if turn.deprecated { " [deprecated]" } else { "" };
        let preview: String = turn.content.chars().take(60).collect();
        lines.push(format!("│   {}. [{:?}]{dep}: {}", i, turn.role, preview));
    }
    if snapshot.conversation.len() > 5 {
        lines.push(format!(
            "│   ... ({} more turns)",
            snapshot.conversation.len() - 5
        ));
    }

    lines.push(format!(
        "├── cognition: {} code summaries",
        snapshot.project_cognition.code_summaries.len()
    ));
    for (key, summary) in &snapshot.project_cognition.code_summaries {
        let st = if summary.stale { " [stale]" } else { "" };
        lines.push(format!("│   ├── {key}{st}"));
    }

    lines.push(format!("└── tool calls: {}", snapshot.tool_call_log.len()));

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample() -> Snapshot {
        Snapshot {
            id: "01234567-0000-0000-0000-000000000000".into(),
            created: TimestampMillis::now(),
            parent_snapshot_id: None,
            meta: SnapshotMeta {
                project_root: PathBuf::from("/p"),
                project_hash: "h".into(),
                model_id: "m".into(),
                tags: vec!["initial".into()],
            },
            conversation: vec![
                ConversationTurn {
                    role: ConversationRole::User,
                    content: "first".into(),
                    tool_calls: None,
                    timestamp: TimestampMillis::now(),
                    deprecated: false,
                },
                ConversationTurn {
                    role: ConversationRole::Assistant,
                    content: "second".into(),
                    tool_calls: None,
                    timestamp: TimestampMillis::now(),
                    deprecated: false,
                },
            ],
            project_cognition: {
                let mut pc = ProjectCognition::empty();
                pc.code_summaries.insert(
                    "src/main.rs".into(),
                    CodeSummary {
                        summary: "main".into(),
                        symbols: vec!["main".into()],
                        last_indexed: TimestampMillis::now(),
                        stale: false,
                    },
                );
                pc
            },
            tool_call_log: vec![],
            config_fingerprint: ConfigFingerprint {
                features: vec![],
                config_hash: "ch".into(),
            },
        }
    }

    #[test]
    fn test_remove_turn() {
        let snap = sample();
        let (edited, result) = apply_fine_tune(&snap, &[FineTuneOp::RemoveTurn(0)]).unwrap();
        assert!(result.modified);
        assert_eq!(edited.conversation.len(), 1);
        assert_eq!(edited.conversation[0].content, "second");
    }

    #[test]
    fn test_deprecate_turn() {
        let snap = sample();
        let (edited, _) = apply_fine_tune(&snap, &[FineTuneOp::DeprecateTurn(0)]).unwrap();
        assert!(edited.conversation[0].deprecated);
    }

    #[test]
    fn test_remove_code_summary() {
        let snap = sample();
        let (edited, _) = apply_fine_tune(
            &snap,
            &[FineTuneOp::RemoveCodeSummary("src/main.rs".into())],
        )
        .unwrap();
        assert!(edited.project_cognition.code_summaries.is_empty());
    }

    #[test]
    fn test_add_tag() {
        let snap = sample();
        let (edited, _) = apply_fine_tune(&snap, &[FineTuneOp::AddTag("refined".into())]).unwrap();
        assert!(edited.meta.tags.contains(&"refined".into()));
    }

    #[test]
    fn test_id_changed_on_edit() {
        let snap = sample();
        let (edited, _) = apply_fine_tune(&snap, &[FineTuneOp::AddTag("x".into())]).unwrap();
        assert_ne!(edited.id, snap.id);
        assert_eq!(edited.parent_snapshot_id, Some(snap.id));
    }

    #[test]
    fn test_render_tree() {
        let snap = sample();
        let tree = render_snapshot_tree(&snap);
        assert!(tree.contains("01234567"));
        assert!(tree.contains("conversation"));
    }
}
