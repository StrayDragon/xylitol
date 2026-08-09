//! Provider adapter layer — maps xylitol-ai-bridge adapters into domain streams.
//!
//! Vendor HTTP/SSE implementations live in `packages/xylitol-ai-bridge`. This
//! module keeps the domain-facing [`LlmAdapter`] / [`AdapterXyModel`] surface and
//! thin constructors used by tests and the composition root.

use std::sync::Arc;

use async_trait::async_trait;
use xylitol_ai_bridge::provider::{
    AdapterKind as AiBridgeAdapterKind, AdapterRef as AiBridgeAdapterRef,
};

use crate::infra::provider::map::{to_bridge_tools, to_xy_error, to_xy_stream};
use crate::protocol::error::XyError;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyModelKind;
use crate::protocol::model::XyToolSchema;
use crate::protocol::ports::{XyGenerateOptions, XyStream};

pub mod factory;
pub mod xy_model;

pub use xy_model::AdapterXyModel;

/// Supported adapter kinds (domain-facing mirror of the bridge enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    OpenAiResponses,
    OpenAiCompletions,
    AnthropicMessages,
}

impl AdapterKind {
    pub fn default_for(kind: XyModelKind) -> Self {
        // String SSOT: `XyModelKind::default_adapter_api` (agent-safe).
        match kind.default_adapter_api() {
            "anthropic-messages" => AdapterKind::AnthropicMessages,
            _ => AdapterKind::OpenAiResponses,
        }
    }

    pub fn from_config_str(s: &str) -> Option<Self> {
        AiBridgeAdapterKind::from_config_str(s).map(Into::into)
    }

    pub fn to_bridge(self) -> AiBridgeAdapterKind {
        match self {
            Self::OpenAiResponses => AiBridgeAdapterKind::OpenAiResponses,
            Self::OpenAiCompletions => AiBridgeAdapterKind::OpenAiCompletions,
            Self::AnthropicMessages => AiBridgeAdapterKind::AnthropicMessages,
        }
    }
}

impl From<AiBridgeAdapterKind> for AdapterKind {
    fn from(value: AiBridgeAdapterKind) -> Self {
        match value {
            AiBridgeAdapterKind::OpenAiResponses => Self::OpenAiResponses,
            AiBridgeAdapterKind::OpenAiCompletions => Self::OpenAiCompletions,
            AiBridgeAdapterKind::AnthropicMessages => Self::AnthropicMessages,
        }
    }
}

impl std::fmt::Display for AdapterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_bridge())
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

fn to_bridge_options(options: XyGenerateOptions) -> xylitol_ai_bridge::AiBridgeGenerateOptions {
    xylitol_ai_bridge::AiBridgeGenerateOptions {
        thinking_level: options.thinking_level,
        level_map: options.level_map,
        thinking_budgets: options.thinking_budgets,
        system_prompt: options.system_prompt,
        obs_parent: options.obs_parent,
    }
}

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
            .generate_stream(messages, &bridge_tools, to_bridge_options(options))
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
            .generate(messages, &bridge_tools, to_bridge_options(options))
            .await
            .map_err(to_xy_error)?;
        Ok(to_xy_stream(stream))
    }
}

/// Anthropic Messages adapter (implementation in xylitol-ai-bridge).
pub struct AnthropicMessagesAdapter {
    inner: MappedBridgeAdapter,
}

impl AnthropicMessagesAdapter {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn xylitol_ai_bridge::hooks::HttpHooks>>,
    ) -> Self {
        let inner = Arc::new(xylitol_ai_bridge::provider::AnthropicMessagesAdapter::new(
            api_key, model, base_url, hooks,
        ));
        Self {
            inner: MappedBridgeAdapter::new(inner),
        }
    }
}

#[async_trait]
impl LlmAdapter for AnthropicMessagesAdapter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.inner.generate_stream(messages, tools, options).await
    }

    async fn generate(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.inner.generate(messages, tools, options).await
    }
}

/// OpenAI Responses adapter (implementation in xylitol-ai-bridge).
pub struct OpenAiResponsesAdapter {
    inner: MappedBridgeAdapter,
}

impl OpenAiResponsesAdapter {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn xylitol_ai_bridge::hooks::HttpHooks>>,
    ) -> Self {
        let inner = Arc::new(xylitol_ai_bridge::provider::OpenAiResponsesAdapter::new(
            api_key, model, base_url, hooks,
        ));
        Self {
            inner: MappedBridgeAdapter::new(inner),
        }
    }
}

#[async_trait]
impl LlmAdapter for OpenAiResponsesAdapter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.inner.generate_stream(messages, tools, options).await
    }

    async fn generate(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.inner.generate(messages, tools, options).await
    }
}

/// OpenAI Chat Completions adapter (implementation in xylitol-ai-bridge).
pub struct OpenAiCompletionsAdapter {
    inner: MappedBridgeAdapter,
}

impl OpenAiCompletionsAdapter {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn xylitol_ai_bridge::hooks::HttpHooks>>,
    ) -> Self {
        let inner = Arc::new(xylitol_ai_bridge::provider::OpenAiCompletionsAdapter::new(
            api_key, model, base_url, hooks,
        ));
        Self {
            inner: MappedBridgeAdapter::new(inner),
        }
    }
}

#[async_trait]
impl LlmAdapter for OpenAiCompletionsAdapter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.inner.generate_stream(messages, tools, options).await
    }

    async fn generate(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.inner.generate(messages, tools, options).await
    }
}
