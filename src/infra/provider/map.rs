//! Provider boundary maps (c1210: LLM messages are identity / passthrough).
//!
//! Session vocabulary stays on [`AgentMessage`] (`Llm` ∪ `Env`). Bridge DTOs
//! are LLM-only; [`crate::domain::llm_project::project_for_llm`] folds Env
//! first. `LlmMessage` is a type alias of [`AiBridgeMessage`], so message maps
//! are identity.
//!
//! Remaining hand-written maps (true boundary seams):
//! - `TokenProvenance` / `ContextTokenEstimate` — `From` bridge → domain
//! - chunk / tool-schema / error boundary conversion
//!
//! Ownership: `src/AGENTS.md`「Provider 适配」；`src/_TODO.md` §F.

use futures::StreamExt;
use xylitol_ai_bridge::dto::{
    AiBridgeChunk, AiBridgeMessage, AiBridgeStream, AiBridgeToolSchema,
    ContextTokenEstimate as AiBridgeContextTokenEstimate,
    TokenProvenance as AiBridgeTokenProvenance,
};
use xylitol_ai_bridge::error::AiBridgeError;

use crate::domain::error::XyError;
use crate::domain::llm_project::project_for_llm;
use crate::domain::message::{AgentMessage, LlmMessage, XyUsage};
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

/// Project session messages to LLM bridge DTOs (provider entry).
pub fn to_bridge_messages(messages: Vec<AgentMessage>) -> Result<Vec<AiBridgeMessage>, XyError> {
    Ok(project_for_llm(&messages))
}

/// Map a single LLM-role [`AgentMessage`] to bridge DTO (identity on Llm arm).
pub fn to_bridge_message(msg: AgentMessage) -> Result<AiBridgeMessage, XyError> {
    match msg {
        AgentMessage::Llm(m) => Ok(m),
        AgentMessage::Env(e) => Err(XyError::Provider(anyhow::anyhow!(
            "to_bridge_message expects Llm after projection; got Env {}",
            e.role_name()
        ))),
    }
}

/// Identity: [`LlmMessage`] ≡ [`AiBridgeMessage`].
#[inline]
pub fn to_bridge_llm_message(msg: LlmMessage) -> AiBridgeMessage {
    msg
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

/// Identity clone (usage types are aliases).
pub fn to_bridge_usage(u: &XyUsage) -> xylitol_ai_bridge::dto::AiBridgeUsage {
    *u
}

pub fn to_xy_chunk(chunk: AiBridgeChunk) -> XyChunk {
    match chunk {
        AiBridgeChunk::TextDelta(t) => XyChunk::TextDelta(t),
        AiBridgeChunk::ThinkingDelta(t) => XyChunk::ThinkingDelta(t),
        AiBridgeChunk::ThinkingEnd {
            thinking,
            thinking_signature,
        } => XyChunk::ThinkingEnd {
            thinking,
            thinking_signature,
        },
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
            finish_reason,
            usage,
        },
    }
}

pub fn to_xy_stream(stream: AiBridgeStream) -> XyStream {
    Box::pin(stream.map(|item| item.map(to_xy_chunk).map_err(to_xy_error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::{AgentMessage, XyStopReason};
    use xylitol_ai_bridge::dto::{AiBridgeStopReason, AiBridgeUsage};

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
