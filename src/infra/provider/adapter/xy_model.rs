//! Generic `XyModel` wrapper around any [`crate::infra::provider::adapter::LlmAdapter`].

use async_trait::async_trait;

use crate::infra::provider::adapter::AdapterRef;
use crate::protocol::error::XyError;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyToolSchema;
use crate::protocol::ports::{XyGenerateOptions, XyModel, XyStream};

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
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        if stream {
            self.adapter.generate_stream(messages, tools, options).await
        } else {
            self.adapter.generate(messages, tools, options).await
        }
    }
}
