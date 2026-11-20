use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeNode};

pub fn harness_sample_message_history_tree() -> Vec<SessionTreeNode> {
    use xylitol_plugin_sdk::session::{EntryBase, MessageEntry};

    fn harness_entry_ms(id: &str) -> u64 {
        1_781_827_200_000u64
            + id.as_bytes()
                .iter()
                .fold(0u64, |acc, b| acc * 31 + *b as u64)
                % 1_000_000
    }

    fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionTreeNode {
        SessionTreeNode {
            entry: SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: parent.map(str::to_string),
                    timestamp: harness_entry_ms(id),
                },
                message: xylitol_plugin_sdk::session::fixture_message_json(role, text),
            }),
            children: Vec::new(),
            label: None,
        }
    }

    let mut u1 = msg("u1", None, "user", "hello");
    let mut a1 = msg("a1", Some("u1"), "assistant", "plan");
    let t1 = msg("t1", Some("a1"), "tool", "read");
    let mut a2 = msg("a2", Some("a1"), "assistant", "done");
    let u2 = msg("u2", Some("a2"), "user", "next");
    a2.children.push(u2);
    a1.children.extend([t1, a2]);
    u1.children.push(a1);
    vec![u1]
}

pub fn harness_sample_session_messages() -> Vec<SessionEntry> {
    harness_sample_message_history_tree()
        .into_iter()
        .flat_map(flatten_session_tree_entries)
        .collect()
}

fn flatten_session_tree_entries(node: SessionTreeNode) -> Vec<SessionEntry> {
    let mut out = vec![node.entry];
    for child in node.children {
        out.extend(flatten_session_tree_entries(child));
    }
    out
}
