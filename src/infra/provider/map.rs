//! AiBridge DTO ↔ domain type mapping (c1030).
//!
//! `xylitol-ai-bridge` owns vendor wiring; this module is the only place that
//! converts between package DTOs and xylitol domain / runtime_protocol types.

use futures::StreamExt;
use xylitol_ai_bridge::dto::{
    AiBridgeChunk, AiBridgeMessage, AiBridgeStopReason, AiBridgeStream, AiBridgeToolSchema,
    AiBridgeUsage, AiBridgeUsageCost, ContextTokenEstimate as AiBridgeContextTokenEstimate,
    TokenProvenance as AiBridgeTokenProvenance,
};
use xylitol_ai_bridge::error::AiBridgeError;

use crate::domain::error::XyError;
use crate::domain::message::{AgentMessage, XyStopReason, XyUsage, XyUsageCost};
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

pub fn to_bridge_messages(messages: Vec<AgentMessage>) -> Result<Vec<AiBridgeMessage>, XyError> {
    messages.into_iter().map(to_bridge_message).collect()
}

pub fn to_bridge_message(msg: AgentMessage) -> Result<AiBridgeMessage, XyError> {
    let value = serde_json::to_value(&msg)
        .map_err(|e| XyError::Provider(anyhow::anyhow!("map AgentMessage→json: {e}")))?;
    serde_json::from_value(value)
        .map_err(|e| XyError::Provider(anyhow::anyhow!("map json→AiBridgeMessage: {e}")))
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
        AiBridgeChunk::FunctionCall { name, args, id } => XyChunk::FunctionCall { name, args, id },
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
    fn agent_message_roundtrip_user() {
        let msg = AgentMessage::user("hello");
        let bridge = to_bridge_message(msg.clone()).expect("to bridge");
        let value = serde_json::to_value(&bridge).unwrap();
        let back: AgentMessage = serde_json::from_value(value).unwrap();
        assert_eq!(back.role_name(), "user");
        assert_eq!(back.text(), "hello");
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
