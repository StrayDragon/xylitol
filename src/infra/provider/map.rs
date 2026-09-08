//! Provider boundary maps (c1210 / c1220 / c1940).
//!
//! Session vocabulary stays on [`crate::protocol::message::AgentMessage`]
//! (`Llm` ∪ `Env`). Bridge DTOs are LLM-only; agent MUST call
//! [`crate::agent::llm_project::project_for_llm`] before crossing into
//! [`crate::protocol::ports::XyModel`]. `LlmMessage` ≡
//! [`xylitol_ai_bridge::dto::AiBridgeMessage`]; `XyChunk` ≡ `AiBridgeChunk`.
//!
//! This module maps error / token-estimate seams and stream error conversion —
//! no `AgentMessage` folding paths.

use futures::StreamExt;
use xylitol_ai_bridge::dto::{AiBridgeStream, AiBridgeToolSchema};
use xylitol_ai_bridge::error::AiBridgeError;

use crate::protocol::error::XyError;
use crate::protocol::model::XyToolSchema;
use crate::protocol::ports::XyStream;

/// `XyToolSchema` ≡ [`AiBridgeToolSchema`] — clone into owned vec for the bridge call.
pub fn to_bridge_tools(tools: &[XyToolSchema]) -> Vec<AiBridgeToolSchema> {
    tools.to_vec()
}

pub fn to_xy_error(err: AiBridgeError) -> XyError {
    match err {
        AiBridgeError::Provider(e) => XyError::Provider(e),
        AiBridgeError::Aborted => XyError::Aborted,
        AiBridgeError::Io(e) => XyError::Provider(anyhow::anyhow!("{e}")),
    }
}

pub fn to_bridge_error(err: XyError) -> AiBridgeError {
    match err {
        XyError::Provider(e) => AiBridgeError::Provider(e),
        XyError::Aborted => AiBridgeError::Aborted,
        other => AiBridgeError::Provider(anyhow::anyhow!("{other}")),
    }
}

pub fn to_xy_stream(stream: AiBridgeStream) -> XyStream {
    Box::pin(stream.map(|item| item.map_err(to_xy_error)))
}

#[cfg(test)]
mod tests {
    use crate::protocol::message::XyStopReason;
    use crate::protocol::model::XyChunk;
    use xylitol_ai_bridge::dto::{AiBridgeChunk, AiBridgeStopReason, AiBridgeUsage};

    #[test]
    fn chunk_alias_done_usage() {
        let chunk = AiBridgeChunk::Done {
            finish_reason: AiBridgeStopReason::Stop,
            usage: Some(AiBridgeUsage {
                input: 1,
                output: 2,
                total_tokens: 3,
                ..Default::default()
            }),
        };
        let as_xy: XyChunk = chunk;
        match as_xy {
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
