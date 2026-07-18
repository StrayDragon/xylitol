//! [`LlmMessage`] ↔ xylitol-ai-bridge DTO mapping (c1030 / c1070).
//!
//! Session vocabulary stays on [`AgentMessage`] (`Llm` ∪ `Env`). Bridge DTOs
//! are LLM-only; [`crate::domain::llm_project::project_for_llm`] folds Env
//! first. This module maps [`LlmMessage`] ↔ package DTOs for the provider path.

use futures::StreamExt;
use xylitol_ai_bridge::dto::{
    AiBridgeChunk, AiBridgeImageContent, AiBridgeMessage, AiBridgePart, AiBridgeStopReason,
    AiBridgeStream, AiBridgeToolSchema, AiBridgeUsage, AiBridgeUsageCost,
    ContextTokenEstimate as AiBridgeContextTokenEstimate, Diagnostic as AiBridgeDiagnostic,
    TokenProvenance as AiBridgeTokenProvenance,
};
use xylitol_ai_bridge::error::AiBridgeError;

use crate::domain::error::XyError;
use crate::domain::llm_project::project_for_llm;
use crate::domain::message::{
    AgentMessage, AgentPart, Diagnostic, ImageContent, LlmMessage, XyStopReason, XyUsage,
    XyUsageCost,
};
use crate::domain::types::{ContextTokenEstimate, TokenProvenance, XyChunk, XyToolSchema};
use crate::runtime_protocol::XyStream;

impl From<AiBridgeTokenProvenance> for TokenProvenance {
    fn from(value: AiBridgeTokenProvenance) -> Self {
        match value {
            AiBridgeTokenProvenance::Api => Self::Api,
            AiBridgeTokenProvenance::RemoteCount => Self::RemoteCount,
            AiBridgeTokenProvenance::LocalTokenizer => Self::LocalTokenizer,
            AiBridgeTokenProvenance::Heuristic => Self::Heuristic,
            AiBridgeTokenProvenance::Unknown => Self::Unknown,
        }
    }
}

impl From<AiBridgeContextTokenEstimate> for ContextTokenEstimate {
    fn from(value: AiBridgeContextTokenEstimate) -> Self {
        Self {
            tokens: value.tokens,
            provenance: value.provenance.into(),
            usage_tokens: value.usage_tokens,
            trailing_tokens: value.trailing_tokens,
            last_usage_index: value.last_usage_index,
        }
    }
}

/// Project session messages then map to LLM bridge DTOs (provider entry).
pub fn to_bridge_messages(messages: Vec<AgentMessage>) -> Result<Vec<AiBridgeMessage>, XyError> {
    Ok(project_for_llm(&messages)
        .into_iter()
        .map(to_bridge_llm_message)
        .collect())
}

/// Map a single [`LlmMessage`] (or LLM-role [`AgentMessage`]) to bridge DTO.
pub fn to_bridge_message(msg: AgentMessage) -> Result<AiBridgeMessage, XyError> {
    match msg {
        AgentMessage::Llm(m) => Ok(to_bridge_llm_message(m)),
        AgentMessage::Env(e) => Err(XyError::Provider(anyhow::anyhow!(
            "to_bridge_message expects Llm after projection; got Env {}",
            e.role_name()
        ))),
    }
}

pub fn to_bridge_llm_message(msg: LlmMessage) -> AiBridgeMessage {
    match msg {
        LlmMessage::UserMessage { content, timestamp } => AiBridgeMessage::UserMessage {
            content: content.into_iter().map(to_bridge_part).collect(),
            timestamp,
        },
        LlmMessage::AssistantMessage {
            content,
            stop_reason,
            usage,
            api,
            provider,
            model,
            response_id,
            error_message,
            timestamp,
            diagnostics,
        } => AiBridgeMessage::AssistantMessage {
            content: content.into_iter().map(to_bridge_part).collect(),
            stop_reason: stop_reason.map(to_bridge_stop_reason),
            usage: usage.as_ref().map(to_bridge_usage),
            api,
            provider,
            model,
            response_id,
            error_message,
            timestamp,
            diagnostics: diagnostics.into_iter().map(to_bridge_diagnostic).collect(),
        },
        LlmMessage::ToolResultMessage {
            tool_use_id,
            tool_name,
            content,
            details,
            is_error,
            timestamp,
        } => AiBridgeMessage::ToolResultMessage {
            tool_use_id,
            tool_name,
            content: content.into_iter().map(to_bridge_part).collect(),
            details,
            is_error,
            timestamp,
        },
    }
}

fn to_bridge_part(part: AgentPart) -> AiBridgePart {
    match part {
        AgentPart::Text { text } => AiBridgePart::Text { text },
        AgentPart::Image(img) => AiBridgePart::Image(to_bridge_image(img)),
        AgentPart::Thinking {
            thinking,
            redacted,
            thinking_signature,
        } => AiBridgePart::Thinking {
            thinking,
            redacted,
            thinking_signature,
        },
        AgentPart::ToolCall {
            id,
            name,
            arguments,
        } => AiBridgePart::ToolCall {
            id,
            name,
            arguments,
        },
    }
}

fn to_bridge_image(img: ImageContent) -> AiBridgeImageContent {
    AiBridgeImageContent {
        url: img.url,
        data: img.data,
        media_type: img.media_type,
    }
}

fn to_bridge_diagnostic(d: Diagnostic) -> AiBridgeDiagnostic {
    AiBridgeDiagnostic {
        message: d.message,
        source: d.source,
    }
}

fn to_bridge_stop_reason(r: XyStopReason) -> AiBridgeStopReason {
    match r {
        XyStopReason::Stop => AiBridgeStopReason::Stop,
        XyStopReason::MaxTokens => AiBridgeStopReason::MaxTokens,
        XyStopReason::Error => AiBridgeStopReason::Error,
        XyStopReason::Aborted => AiBridgeStopReason::Aborted,
        XyStopReason::ToolUse => AiBridgeStopReason::ToolUse,
    }
}

pub fn to_bridge_tools(tools: &[XyToolSchema]) -> Vec<AiBridgeToolSchema> {
    tools
        .iter()
        .map(|t| AiBridgeToolSchema {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        })
        .collect()
}

pub fn to_xy_error(err: AiBridgeError) -> XyError {
    match err {
        AiBridgeError::Provider(e) => XyError::Provider(e),
        AiBridgeError::Aborted => XyError::Aborted,
    }
}

pub fn to_bridge_error(err: XyError) -> AiBridgeError {
    match err {
        XyError::Provider(e) => AiBridgeError::Provider(e),
        XyError::Aborted => AiBridgeError::Aborted,
        other => AiBridgeError::Provider(anyhow::anyhow!("{other}")),
    }
}

fn to_xy_stop_reason(r: AiBridgeStopReason) -> XyStopReason {
    match r {
        AiBridgeStopReason::Stop => XyStopReason::Stop,
        AiBridgeStopReason::MaxTokens => XyStopReason::MaxTokens,
        AiBridgeStopReason::Error => XyStopReason::Error,
        AiBridgeStopReason::Aborted => XyStopReason::Aborted,
        AiBridgeStopReason::ToolUse => XyStopReason::ToolUse,
    }
}

fn to_xy_usage(u: AiBridgeUsage) -> XyUsage {
    XyUsage {
        input: u.input,
        output: u.output,
        cache_read: u.cache_read,
        cache_write: u.cache_write,
        cache_write_1h: u.cache_write_1h,
        total_tokens: u.total_tokens,
        cost: u.cost.map(|c| XyUsageCost {
            input: c.input,
            output: c.output,
            cache_read: c.cache_read,
            cache_write: c.cache_write,
            total: c.total,
        }),
    }
}

pub fn to_bridge_usage(u: &XyUsage) -> AiBridgeUsage {
    AiBridgeUsage {
        input: u.input,
        output: u.output,
        cache_read: u.cache_read,
        cache_write: u.cache_write,
        cache_write_1h: u.cache_write_1h,
        total_tokens: u.total_tokens,
        cost: u.cost.map(|c| AiBridgeUsageCost {
            input: c.input,
            output: c.output,
            cache_read: c.cache_read,
            cache_write: c.cache_write,
            total: c.total,
        }),
    }
}

pub fn to_xy_chunk(chunk: AiBridgeChunk) -> XyChunk {
    match chunk {
        AiBridgeChunk::TextDelta(t) => XyChunk::TextDelta(t),
        AiBridgeChunk::ThinkingDelta(t) => XyChunk::ThinkingDelta(t),
        AiBridgeChunk::ToolCallStart { id, name } => XyChunk::ToolCallStart { id, name },
        AiBridgeChunk::ToolCallDelta {
            id,
            name,
            args_delta,
            args,
        } => XyChunk::ToolCallDelta {
            id,
            name,
            args_delta,
            args,
        },
        AiBridgeChunk::ToolCallEnd { id, name, args } => XyChunk::ToolCallEnd { id, name, args },
        AiBridgeChunk::Done {
            finish_reason,
            usage,
        } => XyChunk::Done {
            finish_reason: to_xy_stop_reason(finish_reason),
            usage: usage.map(to_xy_usage),
        },
    }
}

pub fn to_xy_stream(stream: AiBridgeStream) -> XyStream {
    Box::pin(stream.map(|item| item.map(to_xy_chunk).map_err(to_xy_error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::AgentMessage;

    #[test]
    fn llm_user_maps_to_bridge() {
        let msg = AgentMessage::user("hello");
        let bridge = to_bridge_message(msg).expect("to bridge");
        assert_eq!(bridge.role_name(), "user");
        assert_eq!(bridge.text(), "hello");
    }

    #[test]
    fn bash_projects_then_maps() {
        let msgs = vec![AgentMessage::bash("echo hi", "hi", Some(0))];
        let bridge = to_bridge_messages(msgs).expect("project+map");
        assert_eq!(bridge.len(), 1);
        assert_eq!(bridge[0].role_name(), "user");
        assert!(bridge[0].text().contains("echo hi"));
    }

    #[test]
    fn env_rejected_without_projection() {
        let msg = AgentMessage::bash("x", "y", None);
        assert!(to_bridge_message(msg).is_err());
    }

    #[test]
    fn chunk_maps_done_usage() {
        let chunk = AiBridgeChunk::Done {
            finish_reason: AiBridgeStopReason::Stop,
            usage: Some(AiBridgeUsage {
                input: 1,
                output: 2,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: 0,
                total_tokens: 3,
                cost: None,
            }),
        };
        match to_xy_chunk(chunk) {
            XyChunk::Done {
                finish_reason,
                usage,
            } => {
                assert_eq!(finish_reason, XyStopReason::Stop);
                assert_eq!(usage.unwrap().total_tokens, 3);
            }
            _ => panic!("expected Done"),
        }
    }
}
