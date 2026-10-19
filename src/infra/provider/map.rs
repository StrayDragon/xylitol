//! Provider boundary maps.
//!
//! Session vocabulary stays on [`crate::protocol::message::AgentMessage`]
//! (`Llm` ∪ `Env`). Agent MUST call
//! [`crate::agent::llm_project::project_for_llm`] before crossing into
//! [`xylitol_plugin_sdk::XyModel`]. Dialect plugins use the same
//! [`xylitol_plugin_sdk::XyLlmMessage`] / [`xylitol_plugin_sdk::XyChunk`] /
//! [`xylitol_plugin_sdk::XyError`] types — no DTO conversion at this seam.

#[cfg(test)]
mod tests {
    use crate::protocol::message::{XyStopReason, XyUsage};
    use crate::protocol::model::XyChunk;

    #[test]
    fn chunk_done_usage_is_sdk_vocabulary() {
        let chunk = XyChunk::Done {
            finish_reason: XyStopReason::Stop,
            usage: Some(XyUsage {
                input: 1,
                output: 2,
                total_tokens: 3,
                ..Default::default()
            }),
        };
        match chunk {
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
