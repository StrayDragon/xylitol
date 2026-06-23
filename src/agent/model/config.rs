/// LLM provider construction and mock model state for testing.
///
/// The [`ModelConfig`] data struct lives in [`crate::core::model`];
/// this module extends it with the [`build`](ModelConfigExt::build) method
/// that creates provider instances, plus thread-local helpers for BDD tests.
use std::cell::RefCell;
use std::sync::Arc;

use crate::core::model::{ModelConfig, ModelKind};
use crate::core::traits::XyModel;

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

// ── Extension trait: ModelConfig → provider ───────────────────────

/// Extension trait adding provider construction to [`ModelConfig`].
pub trait ModelConfigExt {
    fn build(&self) -> Result<Arc<dyn XyModel>, String>;
}

impl ModelConfigExt for ModelConfig {
    fn build(&self) -> Result<Arc<dyn XyModel>, String> {
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
            ModelKind::Fake => {
                use crate::agent::provider::{FakeProvider, ScenarioStep};
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
}
