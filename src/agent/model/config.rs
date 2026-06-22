/// LLM provider configuration and model registry.
///
/// Supports OpenAI-compatible and Anthropic providers via direct HTTP integration.
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Mock model state (BDD tests only) ──────────────────────────────
//
// Thread-locals let FakeProvider scenarios be configured from BDD step
// functions without refactoring the provider construction pipeline.
// Every test that touches mock state should call reset_fake_state() first.

use std::cell::RefCell;

thread_local! {
    static FAKE_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static FAKE_TOOL_CALL: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    static FAKE_TOOL_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Reset all mock state (call in test setup when needed).
pub fn reset_fake_state() {
    FAKE_TEXT.with(|c| c.replace(None));
    FAKE_TOOL_CALL.with(|c| c.replace(None));
    FAKE_TOOL_RESULT.with(|c| c.replace(None));
}

/// Set the text the fake model should return.
pub fn set_fake_text(text: &str) {
    FAKE_TEXT.with(|c| c.replace(Some(text.to_string())));
}

/// Set the tool call the fake model should return.
pub fn set_fake_tool_call(name: &str, args: &str) {
    FAKE_TOOL_CALL.with(|c| c.replace(Some((name.to_string(), args.to_string()))));
}

/// Set the result a tool execution should return.
pub fn set_fake_tool_result(text: &str) {
    FAKE_TOOL_RESULT.with(|c| c.replace(Some(text.to_string())));
}

use crate::agent::traits::XyModel;
use schemars::JsonSchema;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    #[serde(rename = "openai")]
    #[default]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[cfg(feature = "dev-fake-provider")]
    #[serde(rename = "fake")]
    Fake,
}

impl ModelKind {
    /// Parse from a provider name string (case-insensitive).
    pub fn from_provider_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            #[cfg(feature = "dev-fake-provider")]
            "fake" => Some(Self::Fake),
            _ => None,
        }
    }

    /// Provider identifier for display and serialization.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            #[cfg(feature = "dev-fake-provider")]
            Self::Fake => "fake",
        }
    }
}

/// Configuration for building an LLM provider instance.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub kind: ModelKind,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

impl ModelConfig {
    /// Build the LLM provider from this configuration.
    pub fn build(&self) -> Result<Arc<dyn XyModel>, String> {
        match self.kind {
            ModelKind::OpenAi => {
                let provider = crate::agent::provider::openai::OpenAIProvider::new(
                    self.api_key.clone(),
                    self.model.clone(),
                    self.base_url.clone(),
                );
                Ok(Arc::new(provider) as Arc<dyn XyModel>)
            }
            ModelKind::Anthropic => {
                let provider = crate::agent::provider::anthropic::AnthropicProvider::new(
                    self.api_key.clone(),
                    self.model.clone(),
                    self.base_url.clone(),
                );
                Ok(Arc::new(provider) as Arc<dyn XyModel>)
            }
            #[cfg(feature = "dev-fake-provider")]
            ModelKind::Fake => {
                use crate::agent::provider::{FakeProvider, ScenarioStep};
                let steps = {
                    // Check thread-local mock configuration first.
                    let tool = FAKE_TOOL_CALL.with(|c| c.borrow_mut().take());
                    let text = FAKE_TEXT.with(|c| c.borrow_mut().take());
                    if let Some((tool_name, tool_args)) = tool {
                        let args: serde_json::Value =
                            serde_json::from_str(&tool_args).unwrap_or(serde_json::json!({}));
                        let tool_result =
                            FAKE_TOOL_RESULT.with(|c| c.borrow_mut().take());
                        vec![
                            ScenarioStep::tool_call(tool_name, args),
                            ScenarioStep::tool_result(
                                "tool-1",
                                serde_json::json!({"content": tool_result.unwrap_or_else(|| "done".into())}),
                            ),
                            ScenarioStep::text("Tool execution complete"),
                        ]
                    } else if let Some(t) = text {
                        vec![ScenarioStep::text(t)]
                    } else {
                        vec![ScenarioStep::text("Hello from fake provider")]
                    }
                };
                let fake = FakeProvider::new("__fake__", steps);
                Ok(Arc::new(fake) as Arc<dyn XyModel>)
            }
        }
    }

    /// Provider identifier for display purposes.
    pub fn provider_name(&self) -> &'static str {
        self.kind.provider_name()
    }
}
