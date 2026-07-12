//! Generic `XyModel` wrapper around any [`crate::infra::provider::adapter::LlmAdapter`].
//!
//! This lets the composition root build a provider purely from an adapter,
//! without separate `OpenAIProvider` / `AnthropicProvider` shells.

use async_trait::async_trait;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::types::XyToolSchema;
use crate::infra::provider::adapter::AdapterRef;
use crate::runtime_protocol::{XyModel, XyStream};

/// An [`XyModel`] backed by an [`AdapterRef`].
pub struct AdapterXyModel {
    adapter: AdapterRef,
}

impl AdapterXyModel {
    /// Wrap an adapter as an `XyModel`.
    pub fn new(adapter: AdapterRef) -> Self {
        Self { adapter }
    }
}

#[async_trait]
impl XyModel for AdapterXyModel {
    fn name(&self) -> &str {
        self.adapter.name()
    }

    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError> {
        if stream {
            self.adapter.generate_stream(messages, tools).await
        } else {
            self.adapter.generate(messages, tools).await
        }
    }
}
