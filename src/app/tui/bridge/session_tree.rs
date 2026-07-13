//! Rebuild live scrollback after MessageHistory travel (c615).

use crate::domain::session_types::{SessionEntry, SessionTreeTravel, message_role, message_text};

use super::{UiEntry, UiModel, UiPhase};

/// Replace transcript with entries on the ancestry path to `travel.leaf_id`.
pub fn rebuild_scrollback_from_travel(
    ui_model: &mut UiModel,
    entries: &[SessionEntry],
    travel: &SessionTreeTravel,
) {
    ui_model.entries.clear();
    ui_model.phase = UiPhase::Idle;
    ui_model.status = None;
    ui_model.streaming_assistant.clear();
    ui_model.streaming_thinking.clear();
    ui_model.current_role = None;

    let leaf = travel.leaf_id.as_deref();
    let path = ancestry_path_ids(entries, leaf);
    let path_label = if path.is_empty() {
        "(root)".to_string()
    } else {
        path.join(" → ")
    };
    ui_model.entries.push(UiEntry::System {
        text: format!(
            "history @ {} · leaf={} · path: {path_label}",
            travel.selected_id,
            leaf.unwrap_or("(root)")
        ),
    });

    for id in path {
        let Some(entry) = entries.iter().find(|e| e.entry_id() == Some(id.as_str())) else {
            continue;
        };
        if let Some(ui) = session_entry_to_ui(entry) {
            ui_model.entries.push(ui);
        }
    }
}

fn ancestry_path_ids(entries: &[SessionEntry], leaf_id: Option<&str>) -> Vec<String> {
    let Some(mut cur) = leaf_id.map(str::to_string) else {
        return Vec::new();
    };
    let mut path = vec![cur.clone()];
    while let Some(parent) = entries
        .iter()
        .find(|e| e.entry_id() == Some(cur.as_str()))
        .and_then(|e| e.parent_id())
        .map(str::to_string)
    {
        path.push(parent.clone());
        cur = parent;
    }
    path.reverse();
    path
}

fn session_entry_to_ui(entry: &SessionEntry) -> Option<UiEntry> {
    match entry {
        SessionEntry::Message(m) => {
            let text = message_text(&m.message);
            match message_role(&m.message)? {
                "user" => Some(UiEntry::User { text }),
                "assistant" => Some(UiEntry::Assistant { text }),
                "tool" => Some(UiEntry::Tool {
                    id: m.base.id.clone(),
                    name: "tool".into(),
                    args_preview: String::new(),
                    output: text,
                    is_error: false,
                    done: true,
                }),
                _ => Some(UiEntry::System { text }),
            }
        }
        SessionEntry::BashExecution(b) => Some(UiEntry::System {
            text: format!("$ {}\n{}", b.command, b.output),
        }),
        SessionEntry::Compaction(c) => Some(UiEntry::System {
            text: format!("[compaction] {}", c.summary),
        }),
        SessionEntry::BranchSummary(b) => Some(UiEntry::System {
            text: format!("[branch] {}", b.summary),
        }),
        _ => None,
    }
}
