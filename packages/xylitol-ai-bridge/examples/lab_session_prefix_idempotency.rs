//! Lab (c1930): resume/import-shaped history → Responses `input` prefix idempotency.
//!
//! Maintenance only — **not** wired into `just qa`.
//!
//! ```bash
//! # Uses <global-dir>/dev/live-provider.yaml
//! cargo run -p xylitol-ai-bridge --example lab_session_prefix_idempotency
//!
//! # Offline-only (hash gates; skip live HTTP):
//! XYLITOL_LAB_OFFLINE=1 cargo run -p xylitol-ai-bridge --example lab_session_prefix_idempotency
//! ```
//!
//! Arms (landing.tmp.md §5):
//! - A memory continue (online)
//! - B serialize/reload history + new adapter (= process exit / resume)
//! - C JSONL-shaped round-trip of messages → assemble `input` hash must equal baseline
//!
//! Evidence: `/tmp/xylitol-lab-prefix-*` (input JSON dumps + summary.json).
//! Optional Langfuse: dump request `input` for side-by-side with `langfuse.observation.input`.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use xylitol_ai_bridge::dto::{
    AiBridgeChunk, AiBridgeMessage, AiBridgePart, AiBridgeStopReason, AiBridgeToolSchema,
    AiBridgeUsage, PromptCacheRead,
};
use xylitol_ai_bridge::hooks::{HeaderBag, HttpHooks};
use xylitol_ai_bridge::provider::{AiBridgeLlmAdapter, OpenAiResponsesAdapter, ResponsesAssembler};
use xylitol_ai_bridge::{AiBridgeError, AiBridgeGenerateOptions};

#[derive(Debug, Clone, Deserialize)]
struct LiveProviderFile {
    #[serde(default)]
    enabled: bool,
    base_url: String,
    model: String,
    #[serde(default = "default_api_key")]
    api_key: String,
    #[serde(default = "default_max_out")]
    max_output_tokens: u64,
}

fn default_api_key() -> String {
    "sk-local".into()
}
fn default_max_out() -> u64 {
    256
}

fn load_cfg() -> Result<LiveProviderFile, String> {
    let path = match std::env::var("XYLITOL_LIVE_PROVIDER_CONFIG")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
    {
        Some(p) => p,
        None => xylitol_ai_bridge::config::global_config_dir()
            .map(|d| d.join("dev").join("live-provider.yaml"))
            .ok_or_else(|| {
                "no XYLITOL_CONFIG_DIR/XDG_CONFIG_HOME/HOME to locate the global config dir"
                    .to_string()
            })?,
    };
    if !path.is_file() {
        return Err(format!(
            "missing {}; copy configs/testing/live-provider.example.yaml → <global-dir>/dev/live-provider.yaml",
            path.display()
        ));
    }
    let mut file: LiveProviderFile = yaml_serde::from_str(
        &fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?,
    )
    .map_err(|e| format!("parse {}: {e}", path.display()))?;
    if let Ok(v) = std::env::var("XYLITOL_LIVE_BASE_URL")
        && !v.is_empty()
    {
        file.base_url = v;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_MODEL")
        && !v.is_empty()
    {
        file.model = v;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_API_KEY")
        && !v.is_empty()
    {
        file.api_key = v;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_PROVIDER") {
        file.enabled = matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
    }
    if !file.enabled {
        return Err(format!("enabled=false in {}", path.display()));
    }
    eprintln!(
        "lab: config={} model={} base={}",
        path.display(),
        file.model,
        file.base_url
    );
    Ok(file)
}

fn offline_mode() -> bool {
    matches!(
        std::env::var("XYLITOL_LAB_OFFLINE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

struct CaptureBody {
    max_output_tokens: u64,
    last_body: Arc<Mutex<Option<Value>>>,
}

#[async_trait]
impl HttpHooks for CaptureBody {
    async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
        Ok(())
    }

    async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), AiBridgeError> {
        body["max_output_tokens"] = json!(self.max_output_tokens);
        body["store"] = json!(false);
        if let Ok(mut g) = self.last_body.lock() {
            *g = Some(body.clone());
        }
        Ok(())
    }

    async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
}

fn readonly_tools() -> Vec<AiBridgeToolSchema> {
    vec![AiBridgeToolSchema {
        name: "get_workspace_info".into(),
        description: "Read-only workspace facts (lab).".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            },
            "additionalProperties": false
        }),
    }]
}

fn run_readonly_tool(name: &str, _args: &Value) -> String {
    match name {
        "get_workspace_info" => "workspace=xylitol-lab status=ok writable=false".into(),
        other => format!("unknown: {other}"),
    }
}

/// Frozen system prefix (c1930): fixed date — no Utc::now() drift.
fn frozen_system(session_tag: &str) -> String {
    format!(
        "[[lab_session:{session_tag}]]\n\
         Current date: 2026-08-06\n\
         Current working directory: /tmp/xylitol-lab-prefix\n\
         You are a lab coding agent. Prefer get_workspace_info when asked about workspace. \
         Keep answers short. Never invent write tools."
    )
}

fn input_fingerprint(
    messages: &[AiBridgeMessage],
    opts: &AiBridgeGenerateOptions,
) -> (u64, String) {
    let body = ResponsesAssembler::default().assemble(
        "lab-model",
        messages.to_vec(),
        &readonly_tools(),
        false,
        opts,
    );
    let input = body.get("input").cloned().unwrap_or(Value::Array(vec![]));
    let s = serde_json::to_string(&input).unwrap_or_default();
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    (h.finish(), s)
}

fn write_text(path: &Path, s: &str) -> Result<(), String> {
    fs::write(path, s).map_err(|e| format!("write {}: {e}", path.display()))
}

fn messages_to_jsonl(messages: &[AiBridgeMessage]) -> Result<String, String> {
    let mut out = String::new();
    for m in messages {
        let line = serde_json::to_string(m).map_err(|e| e.to_string())?;
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

fn messages_from_jsonl(text: &str) -> Result<Vec<AiBridgeMessage>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let m: AiBridgeMessage =
            serde_json::from_str(line).map_err(|e| format!("jsonl line {}: {e}", i + 1))?;
        out.push(m);
    }
    Ok(out)
}

struct RoundResult {
    usage: Option<AiBridgeUsage>,
}

async fn run_turn(
    adapter: &OpenAiResponsesAdapter,
    history: &mut Vec<AiBridgeMessage>,
    user: &str,
    opts: &AiBridgeGenerateOptions,
    tools: &[AiBridgeToolSchema],
    label: &str,
    max_tool_iters: usize,
) -> RoundResult {
    history.push(AiBridgeMessage::user(user));
    let mut last_usage = None;

    for iter in 0..=max_tool_iters {
        let mut stream = adapter
            .generate_stream(history.clone(), tools, opts.clone())
            .await
            .unwrap_or_else(|e| panic!("{label} generate_stream failed: {e}"));

        let mut text = String::new();
        let mut thinking = String::new();
        let mut thinking_signature: Option<String> = None;
        let mut tool_calls: Vec<(String, String, Value)> = Vec::new();
        let mut usage = None;
        let mut stop = AiBridgeStopReason::Stop;

        while let Some(item) = stream.next().await {
            let chunk = item.unwrap_or_else(|e| panic!("{label} chunk: {e}"));
            match chunk {
                AiBridgeChunk::TextDelta(t) => text.push_str(&t),
                AiBridgeChunk::ThinkingDelta(t) => thinking.push_str(&t),
                AiBridgeChunk::ThinkingEnd {
                    thinking: t,
                    thinking_signature: sig,
                } => {
                    if !t.is_empty() {
                        thinking = t;
                    }
                    if sig.is_some() {
                        thinking_signature = sig;
                    }
                }
                AiBridgeChunk::ToolCallStart { id, name } => {
                    if !tool_calls.iter().any(|(i, _, _)| i == &id) {
                        tool_calls.push((id, name, json!({})));
                    }
                }
                AiBridgeChunk::ToolCallDelta { id, name, args, .. } => {
                    if let Some(slot) = tool_calls.iter_mut().find(|(i, _, _)| i == &id) {
                        slot.1 = name;
                        slot.2 = args;
                    } else {
                        tool_calls.push((id, name, args));
                    }
                }
                AiBridgeChunk::ToolCallEnd { id, name, args } => {
                    if let Some(slot) = tool_calls.iter_mut().find(|(i, _, _)| i == &id) {
                        slot.1 = name;
                        slot.2 = args;
                    } else {
                        tool_calls.push((id, name, args));
                    }
                }
                AiBridgeChunk::Done {
                    finish_reason,
                    usage: u,
                } => {
                    stop = finish_reason;
                    usage = u;
                }
            }
        }

        last_usage = usage.clone();
        let mut parts = Vec::new();
        if !thinking.is_empty() || thinking_signature.is_some() {
            parts.push(AiBridgePart::Thinking {
                thinking: thinking.clone(),
                redacted: false,
                thinking_signature: thinking_signature.clone(),
            });
        }
        if !text.is_empty() {
            parts.push(AiBridgePart::text(text));
        }
        for (id, name, args) in &tool_calls {
            parts.push(AiBridgePart::ToolCall {
                id: id.clone(),
                name: name.clone(),
                arguments: args.clone(),
            });
        }
        history.push(AiBridgeMessage::AssistantMessage {
            content: parts,
            stop_reason: Some(stop),
            usage: usage.clone(),
            api: "openai-responses".into(),
            provider: "lab".into(),
            model: adapter.name().to_string(),
            response_id: None,
            error_message: None,
            timestamp: xylitol_ai_bridge::dto::now_ms(),
            diagnostics: Vec::new(),
        });

        match &last_usage {
            Some(u) => eprintln!(
                "lab: {label} iter={iter} cache_read={} pcr={:?} input={} output={} sig={}",
                u.cache_read,
                u.prompt_cache_read,
                u.input,
                u.output,
                thinking_signature.is_some()
            ),
            None => eprintln!("lab: {label} iter={iter} usage=None"),
        }

        if tool_calls.is_empty() {
            break;
        }
        for (id, name, args) in tool_calls {
            let out = run_readonly_tool(&name, &args);
            history.push(AiBridgeMessage::ToolResultMessage {
                tool_use_id: id,
                tool_name: name,
                content: vec![AiBridgePart::text(out)],
                details: None,
                is_error: false,
                timestamp: xylitol_ai_bridge::dto::now_ms(),
            });
        }
    }

    RoundResult { usage: last_usage }
}

fn cache_tokens(u: &Option<AiBridgeUsage>) -> u64 {
    u.as_ref().map(|x| x.cache_read).unwrap_or(0)
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async move {
        if let Err(e) = run_lab().await {
            eprintln!("lab: FATAL {e}");
            std::process::exit(1);
        }
    });
}

async fn run_lab() -> Result<(), String> {
    let offline = offline_mode();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let session_tag = format!("c1930_{nonce}");
    let out_dir = PathBuf::from(format!("/tmp/xylitol-lab-prefix-{nonce}"));
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let opts = AiBridgeGenerateOptions {
        thinking_level: std::env::var("XYLITOL_LAB_THINKING").unwrap_or_else(|_| "medium".into()),
        system_prompt: Some(frozen_system(&session_tag)),
        ..Default::default()
    };
    let tools = readonly_tools();

    let fixture = vec![
        AiBridgeMessage::user("hello lab"),
        AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::Thinking {
                    thinking: "plan".into(),
                    redacted: false,
                    thinking_signature: Some(
                        json!({
                            "type": "reasoning",
                            "id": "rs_lab",
                            "summary": [{"type": "summary_text", "text": "plan"}],
                            "encrypted_content": ""
                        })
                        .to_string(),
                    ),
                },
                AiBridgePart::text("hi"),
            ],
            stop_reason: Some(AiBridgeStopReason::Stop),
            usage: None,
            api: "openai-responses".into(),
            provider: "lab".into(),
            model: "m".into(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        },
        AiBridgeMessage::user("second"),
    ];

    let (h0, s0) = input_fingerprint(&fixture, &opts);
    write_text(&out_dir.join("offline_input_baseline.json"), &s0)?;

    let json_blob = serde_json::to_string(&fixture).map_err(|e| e.to_string())?;
    let reloaded: Vec<AiBridgeMessage> =
        serde_json::from_str(&json_blob).map_err(|e| e.to_string())?;
    let (h_ser, s_ser) = input_fingerprint(&reloaded, &opts);
    write_text(&out_dir.join("offline_input_serde.json"), &s_ser)?;

    let jsonl = messages_to_jsonl(&fixture)?;
    write_text(&out_dir.join("offline_history.jsonl"), &jsonl)?;
    let from_jsonl = messages_from_jsonl(&jsonl)?;
    let (h_jl, s_jl) = input_fingerprint(&from_jsonl, &opts);
    write_text(&out_dir.join("offline_input_jsonl.json"), &s_jl)?;

    let offline_ok = h0 == h_ser && h0 == h_jl;
    eprintln!(
        "lab: offline hashes baseline={h0:#x} serde={h_ser:#x} jsonl={h_jl:#x} ok={offline_ok}"
    );
    if !offline_ok {
        return Err(
            "offline FAIL: serde/jsonl round-trip changed assemble input fingerprint".into(),
        );
    }
    let (h_again, _) = input_fingerprint(&fixture, &opts);
    if h_again != h0 {
        return Err("offline FAIL: assemble not idempotent on same messages".into());
    }
    eprintln!("lab: OK offline idempotent + jsonl/serde round-trip");

    if offline {
        let summary = json!({
            "mode": "offline",
            "session_tag": session_tag,
            "offline_hash": format!("{h0:#x}"),
            "offline_ok": true,
            "out_dir": out_dir,
        });
        write_text(
            &out_dir.join("summary.json"),
            &serde_json::to_string_pretty(&summary).unwrap(),
        )?;
        eprintln!("lab: summary {}", out_dir.join("summary.json").display());
        return Ok(());
    }

    let cfg = load_cfg()?;
    let last_body = Arc::new(Mutex::new(None));
    let hooks = Arc::new(CaptureBody {
        max_output_tokens: cfg.max_output_tokens.max(64),
        last_body: last_body.clone(),
    });
    let adapter = OpenAiResponsesAdapter::new(
        cfg.api_key.clone(),
        cfg.model.clone(),
        Some(cfg.base_url.clone()),
        Some(hooks.clone()),
    );

    let mut history: Vec<AiBridgeMessage> = Vec::new();
    eprintln!("lab: === warm1 ===");
    let _w1 = run_turn(
        &adapter,
        &mut history,
        "Say hi in one short sentence. No tools.",
        &opts,
        &tools,
        "warm1",
        0,
    )
    .await;

    eprintln!("lab: === warm2 (tool) ===");
    let _w2 = run_turn(
        &adapter,
        &mut history,
        "Call get_workspace_info once, then one short sentence.",
        &opts,
        &tools,
        "warm2",
        3,
    )
    .await;

    eprintln!("lab: === warm3 ===");
    let w3 = run_turn(
        &adapter,
        &mut history,
        "One word only: READY",
        &opts,
        &tools,
        "warm3",
        0,
    )
    .await;

    let (h_live, s_live) = input_fingerprint(&history, &opts);
    write_text(&out_dir.join("live_input_after_warm.json"), &s_live)?;
    if let Ok(g) = last_body.lock()
        && let Some(body) = g.as_ref()
    {
        write_text(
            &out_dir.join("live_last_request_body_warm3.json"),
            &serde_json::to_string_pretty(body).unwrap_or_default(),
        )?;
    }

    let live_jsonl = messages_to_jsonl(&history)?;
    write_text(&out_dir.join("live_history.jsonl"), &live_jsonl)?;
    let hist_import = messages_from_jsonl(&live_jsonl)?;
    let (h_import, s_import) = input_fingerprint(&hist_import, &opts);
    write_text(
        &out_dir.join("live_input_after_import_parse.json"),
        &s_import,
    )?;
    if h_import != h_live {
        return Err(format!(
            "online FAIL: import-shaped jsonl changed input hash {h_live:#x} vs {h_import:#x}"
        ));
    }
    eprintln!("lab: OK live history JSONL round-trip hash={h_live:#x}");

    eprintln!("lab: === arm A memory continue ===");
    let mut hist_a = history.clone();
    let a = run_turn(
        &adapter,
        &mut hist_a,
        "After continue: reply exactly OK_MEM",
        &opts,
        &tools,
        "arm_a_mem",
        0,
    )
    .await;
    if let Ok(g) = last_body.lock()
        && let Some(body) = g.as_ref()
    {
        write_text(
            &out_dir.join("live_request_arm_a.json"),
            &serde_json::to_string_pretty(body).unwrap_or_default(),
        )?;
    }

    drop(adapter);
    let hist_b = messages_from_jsonl(
        &fs::read_to_string(out_dir.join("live_history.jsonl")).map_err(|e| e.to_string())?,
    )?;
    let (h_b_pre, _) = input_fingerprint(&hist_b, &opts);
    if h_b_pre != h_live {
        return Err("online FAIL: pre-resume fingerprint drifted".into());
    }

    let hooks2 = Arc::new(CaptureBody {
        max_output_tokens: cfg.max_output_tokens.max(64),
        last_body: last_body.clone(),
    });
    let adapter2 = OpenAiResponsesAdapter::new(
        cfg.api_key.clone(),
        cfg.model.clone(),
        Some(cfg.base_url.clone()),
        Some(hooks2),
    );
    eprintln!("lab: === arm B resume (new adapter + jsonl history) ===");
    let mut hist_b = hist_b;
    let b = run_turn(
        &adapter2,
        &mut hist_b,
        "After resume: reply exactly OK_RESUME",
        &opts,
        &tools,
        "arm_b_resume",
        0,
    )
    .await;
    if let Ok(g) = last_body.lock()
        && let Some(body) = g.as_ref()
    {
        write_text(
            &out_dir.join("live_request_arm_b.json"),
            &serde_json::to_string_pretty(body).unwrap_or_default(),
        )?;
    }

    let warm3_n = cache_tokens(&w3.usage);
    let a_n = cache_tokens(&a.usage);
    let b_n = cache_tokens(&b.usage);

    if warm3_n > 0 && b_n < warm3_n {
        eprintln!(
            "lab: NOTE arm_b cache_read={b_n} < warm3={warm3_n} — check gateway drift / tools"
        );
    }
    if a_n > 0 && b_n + 32 < a_n {
        eprintln!(
            "lab: WARN arm_b={b_n} much below arm_a={a_n} — possible prefix divergence on wire"
        );
    }

    let summary = json!({
        "mode": "online",
        "session_tag": session_tag,
        "model": cfg.model,
        "base_url": cfg.base_url,
        "thinking_level": opts.thinking_level,
        "offline_hash": format!("{h0:#x}"),
        "live_prefix_hash": format!("{h_live:#x}"),
        "import_hash_match": h_import == h_live,
        "warm3_cache": warm3_n,
        "arm_a_mem_cache": a_n,
        "arm_b_resume_cache": b_n,
        "arm_a_pcr": format!("{:?}", a.usage.as_ref().map(|u| &u.prompt_cache_read)),
        "arm_b_pcr": format!("{:?}", b.usage.as_ref().map(|u| &u.prompt_cache_read)),
        "out_dir": out_dir,
        "langfuse_hint": "Compare live_request_arm_a.json vs live_request_arm_b.json input[] with Langfuse observation.input when OTEL enabled on full xylitol",
    });
    write_text(
        &out_dir.join("summary.json"),
        &serde_json::to_string_pretty(&summary).unwrap(),
    )?;
    eprintln!("lab: {}", serde_json::to_string_pretty(&summary).unwrap());

    if let Some(u) = &w3.usage {
        match u.prompt_cache_read {
            PromptCacheRead::Tokens(n) if n > 0 => {
                eprintln!("lab: OK warm3 saw Tokens({n}) cache");
            }
            other => eprintln!("lab: NOTE warm3 pcr={other:?}"),
        }
    }
    eprintln!(
        "lab: OK online prefix hash stable across JSONL; arm_a_cache={a_n} arm_b_cache={b_n} warm3={warm3_n}"
    );
    eprintln!("lab: evidence dir {}", out_dir.display());

    Ok(())
}
