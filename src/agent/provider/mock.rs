use async_trait::async_trait;

use crate::agent::error::XyError;
use crate::agent::message::AgentMessage;
use crate::agent::traits::{XyModel, XyStream};
use crate::agent::types::{XyChunk, XyFinishReason, XyToolSchema};

/// Drop-in mock for tests. Returns a fixed text response.
pub struct MockXyModel {
    model_name: String,
    response_text: String,
}

impl MockXyModel {
    pub fn new(name: &str) -> Self {
        Self {
            model_name: name.into(),
            response_text: "ok".into(),
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.response_text = text.into();
        self
    }
}

#[async_trait]
impl XyModel for MockXyModel {
    fn name(&self) -> &str {
        &self.model_name
    }

    async fn generate_stream(
        &self,
        _messages: Vec<AgentMessage>,
        _tools: &[XyToolSchema],
        _stream: bool,
    ) -> Result<XyStream, XyError> {
        let text = self.response_text.clone();
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(XyChunk::TextDelta(text)),
            Ok(XyChunk::Done {
                finish_reason: XyFinishReason::Stop,
                usage: None,
            }),
        ])))
    }
}
