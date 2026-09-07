//! Provider adapter layer — maps xylitol-ai-bridge adapters into domain streams.
//!
//! Vendor HTTP/SSE implementations live in `packages/xylitol-ai-bridge`. This
//! module keeps the domain-facing [`LlmAdapter`] / [`AdapterXyModel`] surface.

use std::sync::Arc;

use async_trait::async_trait;
use xylitol_ai_bridge::provider::AdapterRef as AiBridgeAdapterRef;

use crate::infra::provider::map::{to_bridge_tools, to_xy_error, to_xy_stream};
use crate::protocol::error::XyError;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyModelKind;
use crate::protocol::model::XyToolSchema;
use crate::protocol::ports::{XyGenerateOptions, XyStream};

pub mod factory;
pub mod xy_model;

pub use xy_model::AdapterXyModel;

/// Adapter kind (bridge enum, re-exported so domain code never mirrors it).
pub use xylitol_ai_bridge::provider::AdapterKind;

/// Default adapter kind for a model kind. String SSOT:
/// `XyModelKind::default_adapter_api` (agent-safe).
pub fn default_for(kind: XyModelKind) -> AdapterKind {
    match kind.default_adapter_api() {
        "anthropic-messages" => AdapterKind::AnthropicMessages,
        _ => AdapterKind::OpenAiResponses,
    }
}

/// Domain-facing adapter: AiBridge DTO in, [`crate::protocol::model::XyChunk`] stream out.
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError>;

    async fn generate(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError>;
}

pub type AdapterRef = Arc<dyn LlmAdapter>;

/// Wrap a bridge adapter ([`xylitol_ai_bridge::provider::AiBridgeLlmAdapter`]) with domain mapping.
pub struct MappedBridgeAdapter {
    inner: AiBridgeAdapterRef,
}

impl MappedBridgeAdapter {
    pub fn new(inner: AiBridgeAdapterRef) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl LlmAdapter for MappedBridgeAdapter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let bridge_tools = to_bridge_tools(tools);
        let stream = self
            .inner
            .generate_stream(messages, &bridge_tools, options)
            .await
            .map_err(to_xy_error)?;
        Ok(to_xy_stream(stream))
    }

    async fn generate(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let bridge_tools = to_bridge_tools(tools);
        let stream = self
            .inner
            .generate(messages, &bridge_tools, options)
            .await
            .map_err(to_xy_error)?;
        Ok(to_xy_stream(stream))
    }
}
