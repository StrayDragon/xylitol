//! Provider boundary maps (c1210 / c1220).
//!
//! Session vocabulary stays on [`crate::protocol::message::AgentMessage`]
//! (`Llm` ∪ `Env`). Bridge DTOs are LLM-only; agent MUST call
//! [`crate::agent::llm_project::project_for_llm`] before crossing into
//! [`crate::protocol::ports::XyModel`]. `LlmMessage` ≡
//! [`xylitol_ai_bridge::dto::AiBridgeMessage`].
//!
//! This module maps chunk / tool-schema / error / token-estimate seams only —
//! no `AgentMessage` folding paths.

use futures::StreamExt;
use xylitol_ai_bridge::dto::{
    AiBridgeChunk, AiBridgeStream, AiBridgeToolSchema,
    ContextTokenEstimate as AiBridgeContextTokenEstimate,
    TokenProvenance as AiBridgeTokenProvenance,
};
use xylitol_ai_bridge::error::AiBridgeError;

use crate::protocol::error::XyError;
use crate::protocol::message::XyUsage;
use crate::protocol::model::{ContextTokenEstimate, TokenProvenance, XyChunk, XyToolSchema};
use crate::protocol::ports::XyStream;

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
    use crate::protocol::message::XyStopReason;
    use xylitol_ai_bridge::dto::{AiBridgeStopReason, AiBridgeUsage};

    #[test]
    fn chunk_maps_done_usage() {
        let chunk = AiBridgeChunk::Done {
            finish_reason: AiBridgeStopReason::Stop,
            usage: Some(AiBridgeUsage {
                input: 1,
                output: 2,
                total_tokens: 3,
                ..Default::default()
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
