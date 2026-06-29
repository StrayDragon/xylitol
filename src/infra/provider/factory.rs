//! Provider factory — constructs a concrete provider (`Arc<dyn XyModel>`) from
//! a [`ModelConfig`]. This is the only place that names concrete provider
//! types; the agent holds the result as a trait object.
//!
//! Also hosts the thread-local fake-model state used by BDD tests to script
//! `FakeProvider` scenarios without touching the construction pipeline.

use std::cell::RefCell;
use std::sync::Arc;

use crate::domain::model::{ModelConfig, ModelKind};
use crate::infra::provider::anthropic::AnthropicProvider;
use crate::infra::provider::openai::OpenAIProvider;
use crate::infra::provider::{FakeProvider, ScenarioStep};
use crate::runtime_protocol::XyModel;

// ── Mock model state (BDD tests only) ──────────────────────────────
//
// Thread-locals let FakeProvider scenarios be configured from BDD step
// functions without refactoring the provider construction pipeline.
// Every test that touches mock state should call reset_fake_state() first.

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

// ── Factory ────────────────────────────────────────────────────────

/// Build a provider instance from a model config.
///
/// The agent never names concrete provider types; it receives the result as
/// `Arc<dyn XyModel>`. Add new providers by extending this match.
pub fn build_provider(config: &ModelConfig) -> Result<Arc<dyn XyModel>, String> {
    match config.kind {
        ModelKind::OpenAi => {
            let provider = OpenAIProvider::new(
                config.api_key.clone(),
                config.model.clone(),
                config.base_url.clone(),
            );
            Ok(Arc::new(provider) as Arc<dyn XyModel>)
        }
        ModelKind::Anthropic => {
            let provider = AnthropicProvider::new(
                config.api_key.clone(),
                config.model.clone(),
                config.base_url.clone(),
            );
            Ok(Arc::new(provider) as Arc<dyn XyModel>)
        }
        ModelKind::Fake => {
            let steps = {
                let tool = FAKE_TOOL_CALL.with(|c| c.borrow_mut().take());
                let text = FAKE_TEXT.with(|c| c.borrow_mut().take());
                if let Some((tool_name, tool_args)) = tool {
                    let args: serde_json::Value =
                        serde_json::from_str(&tool_args).unwrap_or(serde_json::json!({}));
                    let tool_result = FAKE_TOOL_RESULT.with(|c| c.borrow_mut().take());
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
