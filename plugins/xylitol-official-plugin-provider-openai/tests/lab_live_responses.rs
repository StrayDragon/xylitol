use futures::StreamExt;
use serde_json::json;
use xylitol_official_plugin_provider_openai::dto::XyLlmMessage;
use xylitol_official_plugin_provider_openai::provider::AdapterKind;
use xylitol_official_plugin_provider_openai::provider::factory::build_adapter_with_wire_policy;
use xylitol_official_plugin_provider_openai::thinking::XyGenerateOptions;
use xylitol_official_plugin_provider_openai::wire_policy::WirePolicy;
use xylitol_plugin_sdk::{XyChunk, XyStructuredOutput};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

async fn run(label: &str, options: XyGenerateOptions) -> String {
    let adapter = build_adapter_with_wire_policy(
        env("XYLITOL_LAB_API_KEY"),
        env("XYLITOL_LAB_MODEL"),
        Some(env("XYLITOL_LAB_BASE_URL")),
        AdapterKind::OpenAiResponses,
        None,
        WirePolicy::default(),
    );
    let stream = adapter
        .generate_stream(
            vec![XyLlmMessage::user("Reply with the single word OK.")],
            &[],
            true,
            options,
        )
        .await
        .unwrap_or_else(|e| panic!("{label}: request failed: {e}"));
    let mut text = String::new();
    let mut usage = None;
    for chunk in stream.collect::<Vec<_>>().await {
        match chunk.unwrap_or_else(|e| panic!("{label}: stream error: {e}")) {
            XyChunk::TextDelta(t) => text.push_str(&t),
            XyChunk::Done { usage: u, .. } => usage = u,
            _ => {}
        }
    }
    println!("[{label}] text={text:?} usage={usage:?}");
    assert!(!text.is_empty(), "{label}: empty text");
    text
}

#[tokio::test]
#[ignore = "live network probe; set XYLITOL_LAB_* env vars"]
async fn lab_live_basic() {
    run("basic", XyGenerateOptions::default()).await;
}

#[tokio::test]
#[ignore = "live network probe; set XYLITOL_LAB_* env vars"]
async fn lab_live_structured_output() {
    let mut options = XyGenerateOptions::default();
    options.structured_output = Some(XyStructuredOutput {
        name: "answer".into(),
        schema: json!({
            "type": "object",
            "properties": {"answer": {"type": "string"}},
            "required": ["answer"],
            "additionalProperties": false
        }),
        strict: Some(true),
        description: None,
    });
    let text = run("structured", options).await;
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(parsed) if parsed.get("answer").is_some() => println!("[structured] schema honored"),
        Ok(parsed) => println!("[structured] JSON but missing answer key: {parsed}"),
        Err(e) => println!("[structured] schema not honored by endpoint ({e}): {text:?}"),
    }
}

#[tokio::test]
#[ignore = "live network probe; set XYLITOL_LAB_* env vars"]
async fn lab_live_max_output_tokens() {
    let mut options = XyGenerateOptions::default();
    options.max_output_tokens = Some(32);
    run("max-tokens", options).await;
}

#[tokio::test]
#[ignore = "live network probe; set XYLITOL_LAB_* env vars"]
async fn lab_live_prompt_cache_key() {
    let mut options = XyGenerateOptions::default();
    options.obs_session.session_id = Some(format!("lab-probe-{}", std::process::id()));
    run("prompt-cache-key", options).await;
}

#[tokio::test]
#[ignore = "live network probe; set XYLITOL_LAB_* env vars"]
async fn lab_live_thinking() {
    let mut options = XyGenerateOptions::default();
    options.thinking_level = "high".into();
    let adapter = build_adapter_with_wire_policy(
        env("XYLITOL_LAB_API_KEY"),
        env("XYLITOL_LAB_MODEL"),
        Some(env("XYLITOL_LAB_BASE_URL")),
        AdapterKind::OpenAiResponses,
        None,
        WirePolicy::default(),
    );
    let stream = adapter
        .generate_stream(
            vec![XyLlmMessage::user("What is 2+2? Think briefly.")],
            &[],
            true,
            options,
        )
        .await
        .expect("thinking request failed");
    let mut text = String::new();
    let mut thinking = String::new();
    let mut signature = None;
    for chunk in stream.collect::<Vec<_>>().await {
        match chunk.expect("stream error") {
            XyChunk::TextDelta(t) => text.push_str(&t),
            XyChunk::ThinkingDelta(t) => thinking.push_str(&t),
            XyChunk::ThinkingEnd {
                thinking: t,
                thinking_signature,
            } => {
                thinking = t;
                signature = thinking_signature;
            }
            _ => {}
        }
    }
    println!(
        "[thinking] text={text:?} thinking_len={} signature={signature:?}",
        thinking.len()
    );
}
