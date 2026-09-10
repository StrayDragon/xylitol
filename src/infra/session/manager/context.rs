use super::BashExecutionParams;
use super::SessionManager;
use crate::infra::session::types::*;
use crate::protocol::error::{XySessionError, XySessionStoreError};

impl SessionManager {
    /// Build session context from the stored entries.
    /// Walks from leaf to root, then applies compaction-aware cut (pi `buildContextEntries`).
    pub async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError> {
        let leaf_id = self.get_leaf(session_id);
        let branch = self.get_branch(session_id, leaf_id.as_deref()).await?;

        // No recorded choice starts disabled. A recorded string is restored
        // verbatim below, even if a newer config no longer declares it.
        let mut thinking_level = String::from("off");
        let mut model: Option<(String, String)> = None;
        for entry in &branch {
            match entry {
                SessionEntry::ModelChange(mc) => {
                    model = Some((mc.provider.clone(), mc.model_id.clone()));
                }
                SessionEntry::ThinkingLevelChange(tc) => {
                    thinking_level = tc.thinking_level.clone();
                }
                _ => {}
            }
        }

        let context_entries = crate::protocol::session::build_context_entries(&branch);
        let mut messages = Vec::new();
        for entry in &context_entries {
            // Unified entry→AgentMessage (honors exclude; nested bang-bash only).
            if let Some(msg) = entry.as_agent_message()
                && let Ok(v) = serde_json::to_value(&msg)
            {
                messages.push(v);
            }
        }

        Ok(SessionContext {
            messages,
            thinking_level,
            model,
        })
    }

    // ── Change tracking helpers ─────────────────────────────────

    /// Append a model change entry.
    #[allow(dead_code)]
    pub async fn append_model_change(
        &self,
        session_id: &str,
        provider: &str,
        model_id: &str,
    ) -> Result<(), XySessionStoreError> {
        let entry = SessionEntry::ModelChange(ModelChangeEntry {
            base: EntryBase {
                entry_type: "model_change".into(),
                id: String::new(), // will be filled by append
                parent_id: None,
                timestamp: 0,
            },
            provider: provider.to_string(),
            model_id: model_id.to_string(),
        });
        self.append(session_id, &entry).await
    }

    /// Append a thinking level change entry.
    #[allow(dead_code)]
    pub async fn append_thinking_level_change(
        &self,
        session_id: &str,
        level: &str,
    ) -> Result<(), XySessionStoreError> {
        let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            base: EntryBase {
                entry_type: "thinking_level_change".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            thinking_level: level.to_string(),
        });
        self.append(session_id, &entry).await
    }

    // ── Branch summary (text fallback) ──────────────────────────

    /// Generate a branch summary for cut-point entries.
    #[allow(dead_code)]
    pub fn generate_branch_summary(&self, skipped_entries: &[SessionEntry]) -> String {
        use crate::protocol::session::{
            count_tool_calls, is_user_message, message_text, tool_file_paths,
        };

        if skipped_entries.is_empty() {
            return String::new();
        }

        let total = skipped_entries.len();
        let user_count = skipped_entries
            .iter()
            .filter(|e| is_user_message(e))
            .count();
        let assistant_count = skipped_entries
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SessionEntry::Message(m)
                        if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant")
                )
            })
            .count();

        let tool_calls: usize = skipped_entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => Some(count_tool_calls(&m.message)),
                _ => None,
            })
            .sum();

        let mut files: Vec<String> = Vec::new();
        for entry in skipped_entries {
            if let SessionEntry::Message(m) = entry {
                for path in tool_file_paths(&m.message) {
                    if !files.contains(&path) {
                        files.push(path);
                    }
                }
            }
        }

        let last_user_msg = skipped_entries.iter().rev().find_map(|e| {
            if !is_user_message(e) {
                return None;
            }
            let SessionEntry::Message(m) = e else {
                return None;
            };
            let text = message_text(&m.message);
            if text.is_empty() {
                return None;
            }
            let truncated: String = text.chars().take(100).collect();
            Some(if text.chars().count() > 100 {
                format!("{truncated}...")
            } else {
                truncated
            })
        });

        let mut summary = format!("分支摘要:\n- 跳过 {total} 条记录\n");
        summary.push_str(&format!(
            "- 包含: {user_count} 条用户消息, {assistant_count} 条助手消息"
        ));
        if tool_calls > 0 {
            summary.push_str(&format!(", {tool_calls} 次工具调用"));
        }
        summary.push('\n');

        if let Some(ref msg) = last_user_msg {
            summary.push_str(&format!("- 最后一条用户消息: \"{msg}\"\n"));
        }

        if !files.is_empty() {
            files.sort();
            files.dedup();
            summary.push_str(&format!("- 涉及文件: {}\n", files.join(", ")));
        }

        summary
    }

    // ── Label and session info ─────────────────────────────────

    /// Append a label change entry.
    /// Labels are user-defined bookmarks/markers on entries.
    #[allow(dead_code)]
    pub async fn append_label_change(
        &self,
        session_id: &str,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XySessionError> {
        // Verify target exists
        let _ = self
            .get_entry(session_id, target_id)
            .await?
            .ok_or_else(|| XySessionError::entry_not_found(target_id))?;

        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            target_id: target_id.to_string(),
            label: label.map(String::from),
        });
        self.append(session_id, &entry).await?;
        Ok(())
    }

    /// Get the label for an entry, if any.
    #[allow(dead_code)]
    pub async fn get_label(
        &self,
        session_id: &str,
        target_id: &str,
    ) -> Result<Option<String>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        // Walk in reverse to find the latest label for this target
        for entry in entries.iter().rev() {
            if let SessionEntry::Label(l) = entry
                && l.target_id == target_id
            {
                return Ok(l.label.clone());
            }
        }
        Ok(None)
    }

    /// Append a bash-execution entry (`!cmd` / `!!cmd`) as nested Message (c1210).
    ///
    /// Stored on disk; the `exclude_from_context` flag controls whether it
    /// participates in LLM context (see `build_session_context`).
    #[allow(dead_code)]
    pub async fn append_bash_execution(
        &self,
        params: BashExecutionParams<'_>,
    ) -> Result<(), XySessionStoreError> {
        let entry = crate::protocol::session::bash_execution_message_entry(
            String::new(),
            params.command,
            params.output,
            params.exit_code,
            params.cancelled,
            params.truncated,
            params.full_output_path.map(|s| s.to_string()),
            params.exclude_from_context,
            crate::protocol::message::BashExecutionStatus::Done,
        );
        self.append(params.session_id, &entry).await
    }
}
