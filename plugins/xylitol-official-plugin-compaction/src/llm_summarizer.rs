use anyhow::Result;
use futures::StreamExt;

use xylitol_plugin_sdk::XyModel;
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::model::XyChunk;
use xylitol_plugin_sdk::project_for_llm;

pub const SUMMARIZATION_PROMPT: &str = r#"The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]
- [Or "(none)" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, or references needed to continue]
- [Or "(none)" if not applicable]

Keep each section concise. Preserve exact file paths, function names, and error messages."#;

pub const UPDATE_SUMMARIZATION_PROMPT: &str = r#"The messages above are NEW conversation messages to incorporate into the existing summary provided in <previous-summary> tags.

Update the existing structured summary with new information. RULES:
- PRESERVE all existing information from the previous summary
- ADD new progress, decisions, and context from the new messages
- UPDATE the Progress section: move items from "In Progress" to "Done" when completed
- UPDATE "Next Steps" based on what was accomplished
- PRESERVE exact file paths, function names, and error messages
- If something is no longer relevant, you may remove it

Use this EXACT format:

## Goal
[Preserve existing goals, add new ones if the task expanded]

## Constraints & Preferences
- [Preserve existing, add new ones discovered]

## Progress
### Done
- [x] [Include previously done items AND newly completed items]

### In Progress
- [ ] [Current work - update based on progress]

### Blocked
- [Current blockers - remove if resolved]

## Key Decisions
- **[Decision]**: [Brief rationale] (preserve all previous, add new)

## Next Steps
1. [Update based on current state]

## Critical Context
- [Preserve important context, add new if needed]

Keep each section concise. Preserve exact file paths, function names, and error messages."#;

pub const TURN_PREFIX_SUMMARIZATION_PROMPT: &str = r#"This is the PREFIX of a turn that was too large to keep. The SUFFIX (recent work) is retained.

Summarize the prefix to provide context for the retained suffix:

## Original Request
[What did the user ask for in this turn?]

## Early Progress
- [Key decisions and work done in the prefix]

## Context for Suffix
- [Information needed to understand the retained recent work]

Be concise. Focus on what's needed to understand the kept suffix."#;

pub(super) async fn generate_complete(
    model: &dyn XyModel,
    messages: Vec<XyLlmMessage>,
    max_tokens: u32,
    obs_parent: Option<fastrace::prelude::SpanContext>,
    base_options: &xylitol_plugin_sdk::XyGenerateOptions,
) -> Result<String> {
    let mut options = base_options.clone();
    options.max_output_tokens = Some(max_tokens);
    options.obs_parent = obs_parent;
    let mut stream = model
        .generate_stream(messages, &[], false, options)
        .await
        .map_err(|e| anyhow::anyhow!("summarization model error: {e}"))?;

    let mut text = String::new();
    let mut thinking = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk.map_err(|e| anyhow::anyhow!("summarization stream error: {e}"))? {
            XyChunk::TextDelta(delta) => text.push_str(&delta),
            XyChunk::ThinkingDelta(delta) => thinking.push_str(&delta),
            XyChunk::ThinkingEnd { .. } => {}
            XyChunk::Done { .. } => break,
            XyChunk::ToolCallStart { .. }
            | XyChunk::ToolCallDelta { .. }
            | XyChunk::ToolCallEnd { .. } => {}
        }
    }

    if text.trim().is_empty() {
        let thinking = thinking.trim();
        if thinking.is_empty() {
            return Err(anyhow::anyhow!("summarization returned empty response"));
        }
        return Ok(thinking.to_string());
    }

    Ok(text)
}

pub fn serialize_conversation(messages: &[AgentMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        let role = msg.role_name();
        let content = msg.content();
        match role {
            "user" | "toolResult" | "bashExecution" => {
                for part in content {
                    if let Some(t) = part.as_text() {
                        parts.push(format!("[{role}]: {t}"));
                    }
                }
            }
            "assistant" => {
                let mut text_parts = Vec::new();
                let mut thinking_parts = Vec::new();
                let mut tool_calls = Vec::new();
                for part in content {
                    match part {
                        AgentPart::Text { text } => text_parts.push(text.as_str()),
                        AgentPart::Thinking { thinking, .. } => {
                            thinking_parts.push(thinking.as_str())
                        }
                        AgentPart::ToolCall {
                            name, arguments, ..
                        } => {
                            let args_str = if let Some(obj) = arguments.as_object() {
                                obj.iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else {
                                arguments.to_string()
                            };
                            tool_calls.push(format!("{name}({args_str})"));
                        }
                        _ => {}
                    }
                }
                if !thinking_parts.is_empty() {
                    parts.push(format!(
                        "[Assistant thinking]: {}",
                        thinking_parts.join("\n")
                    ));
                }
                if !text_parts.is_empty() {
                    parts.push(format!("[Assistant]: {}", text_parts.join("\n")));
                }
                if !tool_calls.is_empty() {
                    parts.push(format!("[Assistant tool calls]: {}", tool_calls.join("; ")));
                }
            }
            _ => {
                parts.push(format!("[{role}]: {}", msg.text()));
            }
        }
    }
    parts.join("\n\n")
}

pub fn with_additional_focus(base_prompt: &str, custom_instructions: Option<&str>) -> String {
    match custom_instructions.map(str::trim).filter(|s| !s.is_empty()) {
        Some(instr) => format!("{base_prompt}\n\nAdditional focus: {instr}"),
        None => base_prompt.to_string(),
    }
}

pub async fn generate_summary(
    messages: &[AgentMessage],
    model: &dyn XyModel,
    _reserve_tokens: u64,
    previous_summary: Option<&str>,
    custom_instructions: Option<&str>,
    obs_parent: Option<fastrace::prelude::SpanContext>,
    generate_options: &xylitol_plugin_sdk::XyGenerateOptions,
) -> Result<String> {
    let conversation_text = serialize_conversation(messages);

    let base_prompt = if previous_summary.is_some() {
        UPDATE_SUMMARIZATION_PROMPT
    } else {
        SUMMARIZATION_PROMPT
    };
    let base_prompt = with_additional_focus(base_prompt, custom_instructions);

    let mut prompt_text = format!("<conversation>\n{conversation_text}\n</conversation>\n\n");
    if let Some(prev) = previous_summary {
        prompt_text.push_str(&format!(
            "<previous-summary>\n{prev}\n</previous-summary>\n\n"
        ));
    }
    prompt_text.push_str(&base_prompt);

    let summarization_messages = project_for_llm(&[AgentMessage::user(prompt_text.clone())]);

    let max_tokens = ((_reserve_tokens as f64) * 0.8) as u32;
    generate_complete(
        model,
        summarization_messages,
        max_tokens.max(256),
        obs_parent,
        generate_options,
    )
    .await
}

pub async fn generate_turn_prefix_summary(
    messages: &[AgentMessage],
    model: &dyn XyModel,
    _reserve_tokens: u64,
    obs_parent: Option<fastrace::prelude::SpanContext>,
    generate_options: &xylitol_plugin_sdk::XyGenerateOptions,
) -> Result<String> {
    let conversation_text = serialize_conversation(messages);
    let prompt_text = format!(
        "<conversation>\n{conversation_text}\n</conversation>\n\n{TURN_PREFIX_SUMMARIZATION_PROMPT}"
    );
    let summarization_messages = project_for_llm(&[AgentMessage::user(prompt_text)]);
    let max_tokens = ((_reserve_tokens as f64) * 0.5) as u32;
    generate_complete(
        model,
        summarization_messages,
        max_tokens.max(256),
        obs_parent,
        generate_options,
    )
    .await
}
