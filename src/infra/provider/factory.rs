//! Provider factory — constructs a concrete provider (`Arc<dyn XyModel>`) from
//! a [`XyModelConfig`]. This is the only place that names concrete provider
//! types; the agent holds the result as a trait object.
//!
//! Also hosts the thread-local fake-model state used by BDD tests to script
//! `FakeProvider` scenarios without touching the construction pipeline.

use std::cell::RefCell;
use std::sync::Arc;

use crate::domain::model::{XyModelConfig, XyModelKind};
use crate::infra::provider::adapter::{AdapterXyModel, factory::build_adapter};
use crate::infra::provider::{FakeProvider, ScenarioStep};
use crate::runtime_protocol::XyModel;

thread_local! {
    static FAKE_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static FAKE_TOOL_CALL: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    static FAKE_TOOL_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
    /// `(chunk_count, delay_ms)` — slow multi-delta stream for abort BDD.
    static FAKE_SLOW_STREAM: RefCell<Option<(usize, u64)>> = const { RefCell::new(None) };
}

/// Reset all mock state (call in test setup when needed).
pub fn reset_fake_state() {
    FAKE_TEXT.with(|c| c.replace(None));
    FAKE_TOOL_CALL.with(|c| c.replace(None));
    FAKE_TOOL_RESULT.with(|c| c.replace(None));
    FAKE_SLOW_STREAM.with(|c| c.replace(None));
}

/// Set the text the fake model should return.
pub fn set_fake_text(text: &str) {
    FAKE_TEXT.with(|c| c.replace(Some(text.to_string())));
}

/// Script a slow multi-chunk text stream (`chunk_count` deltas, `delay_ms` apart).
pub fn set_fake_slow_stream(chunk_count: usize, delay_ms: u64) {
    FAKE_SLOW_STREAM.with(|c| c.replace(Some((chunk_count, delay_ms))));
}

/// Set the tool call the fake model should return.
pub fn set_fake_tool_call(name: &str, args: &str) {
    FAKE_TOOL_CALL.with(|c| c.replace(Some((name.to_string(), args.to_string()))));
}

/// Set the result a tool execution should return.
pub fn set_fake_tool_result(text: &str) {
    FAKE_TOOL_RESULT.with(|c| c.replace(Some(text.to_string())));
}

/// Build a provider instance from a model config.
pub fn build_provider(config: &XyModelConfig) -> Result<Arc<dyn XyModel>, String> {
    build_provider_with_hooks(config, None)
}

/// Build a provider with optional script hooks wired into HTTP adapters.
pub fn build_provider_with_hooks(
    config: &XyModelConfig,
    hooks: Option<Arc<crate::infra::hooks::HookDispatcher>>,
) -> Result<Arc<dyn XyModel>, String> {
    match config.kind {
        XyModelKind::OpenAi | XyModelKind::Anthropic => {
            let adapter = build_adapter(config, hooks)?;
            Ok(Arc::new(AdapterXyModel::new(adapter)) as Arc<dyn XyModel>)
        }
        XyModelKind::Fake => {
            let steps = {
                let slow = FAKE_SLOW_STREAM.with(|c| c.borrow_mut().take());
                let tool = FAKE_TOOL_CALL.with(|c| c.borrow_mut().take());
                let text = FAKE_TEXT.with(|c| c.borrow_mut().take());
                if let Some((n, delay_ms)) = slow {
                    let chunks: Vec<String> = (0..n).map(|i| format!("chunk-{i}")).collect();
                    vec![ScenarioStep::slow_stream(
                        chunks,
                        std::time::Duration::from_millis(delay_ms),
                    )]
                } else if let Some((tool_name, tool_args)) = tool {
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
