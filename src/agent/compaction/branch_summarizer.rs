//! Branch summarization — LLM-powered summaries for session forks.

use crate::agent::compaction::file_ops::{compute_file_lists, format_file_ops_xml, FileOps};
use crate::agent::compaction::llm_summarizer::{generate_complete, serialize_conversation};
use crate::core::traits::XyModel;
use crate::core::message::AgentMessage;
use crate::infra::session::types::SessionEntry;

const BRANCH_SUMMARY_PROMPT: &str = "Create a structured summary of this conversation branch for context when returning later.\n\nUse this EXACT format:\n\n## Goal\n[What was the user trying to accomplish in this branch?]\n\n## Constraints & Preferences\n- [Any constraints, preferences, or requirements mentioned]\n- [Or \"(none)\" if none were mentioned]\n\n## Progress\n### Done\n- [x] [Completed tasks/changes]\n\n### In Progress\n- [ ] [Work that was started but not finished]\n\n### Blocked\n- [Issues preventing progress, if any]\n\n## Key Decisions\n- **[Decision]**: [Brief rationale]\n\n## Next Steps\n1. [What should happen next to continue this work]\n\nKeep each section concise. Preserve exact file paths, function names, and error messages.";

const BRANCH_SUMMARY_PREAMBLE: &str = "The user explored a different conversation branch before returning here.\nSummary of that exploration:\n\n";

/// Result from branch summarization.
#[derive(Debug, Clone)]
pub struct BranchSummaryResult {
    pub summary: String,
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

/// Result from preparing branch entries.
#[derive(Debug)]
pub struct BranchPreparation {
    pub messages: Vec<AgentMessage>,
    pub file_ops: FileOps,
    pub total_tokens: u64,
}

/// Prepare entries for branch summarization with a token budget.
pub fn prepare_branch_entries(entries: &[SessionEntry], token_budget: u64) -> BranchPreparation {
    let mut messages: Vec<AgentMessage> = Vec::new();
    let mut file_ops = FileOps::default();
    let mut total_tokens: u64 = 0;

    for entry in entries.iter().rev() {
        let msg = match entry.as_agent_message() {
            Some(m) => m,
            None => continue,
        };

        if msg.role_name() == "toolResult" {
            continue;
        }

        extract_single_message_file_ops(&msg, &mut file_ops);

        let tokens =
            (serde_json::to_string(&msg).unwrap_or_default().len() as u64).div_ceil(4);

        if token_budget > 0 && total_tokens + tokens > token_budget {
            if matches!(
                entry,
                SessionEntry::Compaction(_) | SessionEntry::BranchSummary(_)
            ) && total_tokens < token_budget * 9 / 10
            {
                messages.insert(0, msg);
                total_tokens += tokens;
            }
            break;
        }

        messages.insert(0, msg);
        total_tokens += tokens;
    }

    BranchPreparation {
        messages,
        file_ops,
        total_tokens,
    }
}

fn extract_single_message_file_ops(msg: &AgentMessage, ops: &mut FileOps) {
    use crate::core::message::AgentPart;

    for part in msg.content() {
        let (name, path) = match part {
            AgentPart::ToolCall { name, arguments, .. } =>
                match arguments.get("path").and_then(|v| v.as_str()) {
                    Some(p) => (name.as_str(), p.to_string()),
                    None => continue,
                },
            _ => continue,
        };
        match name {
            "read" if !ops.read.contains(&path) => ops.read.push(path),
            "write" if !ops.written.contains(&path) => ops.written.push(path),
            "edit" if !ops.edited.contains(&path) => ops.edited.push(path),
            _ => {}
        }
    }
}

/// Generate an LLM-powered branch summary.
pub async fn generate_branch_summary_llm(
    entries: &[SessionEntry],
    model: &dyn XyModel,
    reserve_tokens: u64,
) -> Option<BranchSummaryResult> {
    if entries.is_empty() {
        return Some(BranchSummaryResult {
            summary: "No content to summarize".to_string(),
            read_files: vec![],
            modified_files: vec![],
        });
    }

    let token_budget = 128_000u64.saturating_sub(reserve_tokens);
    let preparation = prepare_branch_entries(entries, token_budget);

    if preparation.messages.is_empty() {
        return Some(BranchSummaryResult {
            summary: "No content to summarize".to_string(),
            read_files: vec![],
            modified_files: vec![],
        });
    }

    let conversation_text = serialize_conversation(&preparation.messages);
    let prompt_text =
        format!("<conversation>\n{conversation_text}\n</conversation>\n\n{BRANCH_SUMMARY_PROMPT}");

    let messages = vec![AgentMessage::user(prompt_text.clone())];

    let result = generate_complete(model, messages, 2048).await;

    match result {
        Ok(mut summary) => {
            summary = format!("{BRANCH_SUMMARY_PREAMBLE}{summary}");

            let (read_files, modified_files) = compute_file_lists(&preparation.file_ops);
            summary.push_str(&format_file_ops_xml(&read_files, &modified_files));

            Some(BranchSummaryResult {
                summary: if summary.is_empty() {
                    "No summary generated".to_string()
                } else {
                    summary
                },
                read_files,
                modified_files,
            })
        }
        Err(_) => None,
    }
}


