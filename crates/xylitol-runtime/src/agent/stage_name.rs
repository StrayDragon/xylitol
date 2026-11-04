use serde_json::json;

use xylitol_plugin_sdk::session::{
    SessionEntry, is_assistant_message, is_user_message, message_text,
};
use xylitol_plugin_sdk::truncate_utf8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageNameConfig {
    pub enabled: bool,
    pub target_cjk_chars: usize,
    pub target_words: usize,
    pub max_title_bytes: usize,
    pub max_input_bytes: usize,
    pub max_output_tokens: u32,
    pub timeout_ms: u64,
    pub provider: Option<String>,
    pub model: Option<String>,
}

impl Default for StageNameConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_cjk_chars: 15,
            target_words: 20,
            max_title_bytes: 120,
            max_input_bytes: 8192,
            max_output_tokens: 64,
            timeout_ms: 8000,
            provider: None,
            model: None,
        }
    }
}

pub(crate) fn stage_name_system_prompt() -> &'static str {
    "Create a concise session stage name. Return only the name on one line, no quotes, no markdown.      Aim for 12-15 CJK characters or at most 20 English words. Use the language of the conversation.      Name the main work, not a greeting. Prefer the user's leading words when the result is unclear."
}

pub(crate) fn extract_stage_name_inputs(entries: &[SessionEntry]) -> (String, String) {
    let mut user = String::new();
    let mut assistant = String::new();
    for entry in entries {
        let SessionEntry::Message(message) = entry else {
            continue;
        };
        if is_user_message(entry) {
            if user.is_empty() {
                user = message_text(&message.message);
            }
            continue;
        }
        if is_assistant_message(entry)
            && assistant.is_empty()
            && entry
                .as_agent_message()
                .is_some_and(|message| !message.is_error())
        {
            assistant = message_text(&message.message);
        }
        if !user.is_empty() && !assistant.is_empty() {
            break;
        }
    }
    (user, assistant)
}

pub(crate) fn stage_name_input_json(user: &str, assistant: &str, max_input_bytes: usize) -> String {
    let budget = max_input_bytes.saturating_sub(32);
    let half = (budget / 2).max(1);
    let user = truncate_utf8(user, half);
    let assistant = truncate_utf8(assistant, half);
    json!({ "user": user, "assistant": assistant }).to_string()
}
