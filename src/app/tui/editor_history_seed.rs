//! Editor ↑/↓ send-history seeds from session store (c1560 / ati41).

use crate::app::core::driver::{XyDriver, XyDriverError};
use crate::app::tui::session_resume::cwd_matches;
use crate::protocol::session::{SessionEntry, message_role, message_text};

/// Extract user prompt texts from session entries (chrono order; skip `/…`).
pub fn user_prompt_texts_from_entries(entries: &[SessionEntry]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        let SessionEntry::Message(m) = entry else {
            continue;
        };
        if message_role(&m.message) != Some("user") {
            continue;
        }
        let text = message_text(&m.message);
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.starts_with('/') {
            continue;
        }
        out.push(text);
    }
    out
}

/// Prior same-cwd sessions (mtime desc, exclude `current_id`), take `n`, oldest→newest texts.
pub async fn collect_new_session_seed(
    driver: &dyn XyDriver,
    current_cwd: &str,
    current_id: Option<&str>,
    n: u32,
) -> Result<Vec<String>, XyDriverError> {
    if n == 0 {
        return Ok(Vec::new());
    }
    let listed = driver.list_sessions().await.unwrap_or_default();
    let mut matched: Vec<_> = listed
        .into_iter()
        .filter(|e| cwd_matches(e.cwd.as_deref(), current_cwd))
        .filter(|e| current_id.is_none_or(|id| e.id != id))
        .collect();
    matched.sort_by(|a, b| {
        b.modified_unix
            .unwrap_or(0)
            .cmp(&a.modified_unix.unwrap_or(0))
    });
    matched.truncate(n as usize);
    // Process older sessions first so ↑ lands on globally newest prompt.
    matched.reverse();
    let mut texts = Vec::new();
    for entry in matched {
        match driver.load_session_entries(&entry.id).await {
            Ok(entries) => texts.extend(user_prompt_texts_from_entries(&entries)),
            Err(e) => {
                log::debug!(
                    target: "xylitol::tui",
                    "editor history seed skip session {}: {e}",
                    entry.id
                );
            }
        }
    }
    Ok(texts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::{EntryBase, MessageEntry, fixture_message_json};

    fn msg(role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "e".into(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: fixture_message_json(role, text),
        })
    }

    #[test]
    fn extracts_user_skips_slash_and_assistant() {
        let entries = vec![
            msg("user", "hello"),
            msg("assistant", "hi"),
            msg("user", "/session-name x"),
            msg("user", "  /model "),
            msg("user", "world"),
        ];
        assert_eq!(
            user_prompt_texts_from_entries(&entries),
            vec!["hello".to_string(), "world".to_string()]
        );
    }
}
