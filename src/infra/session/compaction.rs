use anyhow::Result;

use super::types::{ConversationRole, ConversationTurn, Snapshot};

/// Which compaction strategy was triggered.
#[derive(Debug, Clone, Copy)]
pub enum CompactionKind {
    /// Automatic — triggered when conversation exceeds token threshold.
    Intra,
    /// User-requested compaction.
    Manual,
    /// Compaction during snapshot derivation.
    Derive,
}

/// Compact a snapshot's conversation by summarising older turns and folding
/// tool-call detail.
///
/// All three strategies (`Intra`, `Manual`, `Derive`) share the same logic;
/// only the trigger differs.
///
/// The result is a new `Vec<ConversationTurn>` where:
/// - The oldest turns beyond `keep_turns` are replaced by a single compacted
///   system turn.
/// - Individual tool calls are folded into a summary string.
/// - `project_cognition` (the agent's "long-term memory") is preserved.
pub fn compact_conversation(
    snapshot: &Snapshot,
    keep_turns: usize,
    _kind: CompactionKind,
) -> Result<Vec<ConversationTurn>> {
    if snapshot.conversation.len() <= keep_turns {
        return Ok(snapshot.conversation.clone());
    }

    let mut turns = snapshot.conversation.clone();

    // Split: compact first N-oldest turns, keep the rest as-is.
    let compact_boundary = turns.len().saturating_sub(keep_turns);
    let old_turns: Vec<ConversationTurn> = turns.drain(..compact_boundary).collect();

    // Build a summary description of the old turns.
    let tool_count: usize = old_turns
        .iter()
        .filter_map(|t| t.tool_calls.as_ref().map(|c| c.len()))
        .sum();

    let user_messages: Vec<&str> = old_turns
        .iter()
        .filter(|t| matches!(t.role, ConversationRole::User))
        .map(|t| t.content.as_str())
        .collect();

    let summary_text = if user_messages.is_empty() {
        format!(
            "[Compacted: {} turns, {tool_count} tool calls]",
            old_turns.len()
        )
    } else {
        format!(
            "[Compacted: {} turns, {tool_count} tools]\nKey user topics:\n- {}",
            old_turns.len(),
            user_messages.join("\n- ")
        )
    };

    // Insert a single system turn representing the compacted history.
    let compacted_turn = ConversationTurn {
        role: ConversationRole::System,
        content: summary_text,
        tool_calls: None,
        timestamp: old_turns[0].timestamp,
        deprecated: false,
    };
    turns.insert(0, compacted_turn);

    Ok(turns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::types::*;

    fn make_turn(role: ConversationRole, content: &str) -> ConversationTurn {
        ConversationTurn {
            role,
            content: content.to_string(),
            tool_calls: None,
            timestamp: TimestampMillis::now(),
            deprecated: false,
        }
    }

    fn sample_snapshot(turns: Vec<ConversationTurn>) -> Snapshot {
        Snapshot {
            id: "test".into(),
            created: TimestampMillis::now(),
            parent_snapshot_id: None,
            meta: SnapshotMeta {
                project_root: "/tmp".into(),
                project_hash: "h".into(),
                model_id: "m".into(),
                tags: vec![],
            },
            conversation: turns,
            project_cognition: ProjectCognition::empty(),
            tool_call_log: vec![],
            config_fingerprint: ConfigFingerprint {
                features: vec![],
                config_hash: "c".into(),
            },
        }
    }

    #[test]
    fn test_no_compaction_needed() {
        let turns = vec![make_turn(ConversationRole::User, "hi")];
        let snap = sample_snapshot(turns.clone());
        let result = compact_conversation(&snap, 5, CompactionKind::Intra).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_compaction_reduces_turns() {
        let mut turns = Vec::new();
        for i in 0..10 {
            turns.push(make_turn(ConversationRole::User, &format!("msg {i}")));
            turns.push(make_turn(ConversationRole::Assistant, &format!("resp {i}")));
        }
        let snap = sample_snapshot(turns);
        let result = compact_conversation(&snap, 4, CompactionKind::Manual).unwrap();
        // 8 turns compacted → 1 summary turn, 4 kept → total 5
        assert_eq!(result.len(), 5);
        assert!(matches!(result[0].role, ConversationRole::System));
    }
}
