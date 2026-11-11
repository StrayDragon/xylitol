use std::cell::RefCell;
use std::sync::Arc;

use crate::provider::adapter::factory::build_adapter;
use crate::provider::{ScenarioStep, fake_xy_model};
use xylitol_plugin_sdk::XyModel;
use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind};

thread_local! {
    static FAKE_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static FAKE_TOOL_CALL: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    static FAKE_TOOL_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
        static FAKE_SLOW_STREAM: RefCell<Option<(usize, u64)>> = const { RefCell::new(None) };
    static FAKE_RETRY_ONCE: RefCell<Option<String>> = const { RefCell::new(None) };
    static FAKE_ERROR: RefCell<Option<(String, bool)>> = const { RefCell::new(None) };
}

#[allow(dead_code)]
pub fn reset_fake_state() {
    FAKE_TEXT.with(|c| c.replace(None));
    FAKE_TOOL_CALL.with(|c| c.replace(None));
    FAKE_TOOL_RESULT.with(|c| c.replace(None));
    FAKE_SLOW_STREAM.with(|c| c.replace(None));
    FAKE_RETRY_ONCE.with(|c| c.replace(None));
    FAKE_ERROR.with(|c| c.replace(None));
}

#[allow(dead_code)]
pub fn set_fake_text(text: &str) {
    FAKE_TEXT.with(|c| c.replace(Some(text.to_string())));
}

#[allow(dead_code)]
pub fn set_fake_slow_stream(chunk_count: usize, delay_ms: u64) {
    FAKE_SLOW_STREAM.with(|c| c.replace(Some((chunk_count, delay_ms))));
}

#[allow(dead_code)]
pub fn set_fake_error(message: &str, retryable: bool) {
    FAKE_ERROR.with(|c| c.replace(Some((message.to_string(), retryable))));
}

#[allow(dead_code)]
pub fn set_fake_retry_once(message: &str) {
    FAKE_RETRY_ONCE.with(|c| c.replace(Some(message.to_string())));
}

#[allow(dead_code)]
pub fn set_fake_tool_call(name: &str, args: &str) {
    FAKE_TOOL_CALL.with(|c| c.replace(Some((name.to_string(), args.to_string()))));
}

#[allow(dead_code)]
pub fn set_fake_tool_result(text: &str) {
    FAKE_TOOL_RESULT.with(|c| c.replace(Some(text.to_string())));
}

#[allow(dead_code)]
pub fn build_provider(config: &XyModelConfig) -> Arc<dyn XyModel> {
    build_provider_with_hooks(config, None)
}

pub fn build_provider_with_hooks(
    config: &XyModelConfig,
    hooks: Option<Arc<xylitol_official_plugin_hooks::HookDispatcher>>,
) -> Arc<dyn XyModel> {
    match config.kind {
        XyModelKind::OpenAi | XyModelKind::Anthropic => build_adapter(config, hooks),
        XyModelKind::Fake => {
            if let Some((message, retryable)) = FAKE_ERROR.with(|c| c.borrow_mut().take()) {
                return fake_xy_model("__fake__", vec![ScenarioStep::error(message, retryable)]);
            }
            let retry_once = FAKE_RETRY_ONCE.with(|c| c.borrow_mut().take());
            if let Some(message) = retry_once {
                let text = FAKE_TEXT
                    .with(|c| c.borrow_mut().take())
                    .or_else(|| std::env::var("XYLITOL_FAKE_TEXT").ok())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| "retry ok".into());
                return fake_xy_model(
                    "__fake__",
                    vec![
                        ScenarioStep::error(message, true),
                        ScenarioStep::text(text.clone()),
                        ScenarioStep::text(text),
                    ],
                );
            }
            let steps = {
                let mut slow = FAKE_SLOW_STREAM.with(|c| c.borrow_mut().take());
                let mut tool = FAKE_TOOL_CALL.with(|c| c.borrow_mut().take());
                let mut text = FAKE_TEXT.with(|c| c.borrow_mut().take());
                let mut env_chunk_chars: Option<usize> = None;
                if slow.is_none()
                    && let Ok(spec) = std::env::var("XYLITOL_FAKE_SLOW_STREAM")
                {
                    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
                    if parts.len() >= 2
                        && let (Ok(n), Ok(delay_ms)) =
                            (parts[0].parse::<usize>(), parts[1].parse::<u64>())
                    {
                        slow = Some((n.max(1), delay_ms));
                        env_chunk_chars = parts.get(2).and_then(|s| s.parse().ok());
                    }
                }
                if text.is_none()
                    && let Ok(t) = std::env::var("XYLITOL_FAKE_TEXT")
                    && !t.is_empty()
                {
                    text = Some(t);
                }
                if tool.is_none()
                    && let Ok(raw) = std::env::var("XYLITOL_FAKE_TOOL_CALL")
                    && let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw)
                    && let Some(name) = value.get("name").and_then(|v| v.as_str())
                {
                    let args = value
                        .get("args")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({}));
                    tool = Some((name.to_string(), args.to_string()));
                }
                if let Some((n, delay_ms)) = slow {
                    let chunk_chars = env_chunk_chars.unwrap_or(12).clamp(1, 200);
                    let pad = "字".repeat(chunk_chars);
                    let chunks: Vec<String> = (0..n).map(|i| format!("[{i}/{n}]{pad}")).collect();
                    vec![ScenarioStep::slow_stream(
                        chunks,
                        std::time::Duration::from_millis(delay_ms),
                    )]
                } else if let Some((tool_name, tool_args)) = tool {
                    let args: serde_json::Value =
                        serde_json::from_str(&tool_args).unwrap_or(serde_json::json!({}));
                    let tool_result = FAKE_TOOL_RESULT
                        .with(|c| c.borrow_mut().take())
                        .or_else(|| std::env::var("XYLITOL_FAKE_TOOL_RESULT").ok());
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
            fake_xy_model("__fake__", steps)
        }
    }
}
