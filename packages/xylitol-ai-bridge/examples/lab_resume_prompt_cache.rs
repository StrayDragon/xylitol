//! Lab: multi-turn Responses + thinking replay × prompt-cache across "process exit".
//!
//! Maintenance only — **not** wired into `just qa`.
//!
//! ```bash
//! # Uses configs/testing/live-provider.local.yaml (or env overrides)
//! cargo run -p xylitol-ai-bridge --example lab_resume_prompt_cache
//!
//! # Optional thinking level (default medium):
//! XYLITOL_LAB_THINKING=off cargo run -p xylitol-ai-bridge --example lab_resume_prompt_cache
//! ```
//!
//! Flow:
//! 1. Warm session A: chat + read-only tool + skill-flavored prompt
//! 2. Serialize history (JSONL-shaped messages)
//! 3. Drop adapter; new adapter instance = process exit / resume
//! 4. Continue with **full replay** (only strategy); print cached_tokens each round

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
use xylitol_ai_bridge::provider::{AiBridgeLlmAdapter, OpenAiResponsesAdapter};
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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn load_cfg() -> Result<LiveProviderFile, String> {
    let path = std::env::var("XYLITOL_LIVE_PROVIDER_CONFIG")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace_root()
                .join("configs")
                .join("testing")
                .join("live-provider.local.yaml")
        });
    if !path.is_file() {
        return Err(format!(
            "missing {}; copy from live-provider.example.yaml",
            path.display()
        ));
    }
    let mut file: LiveProviderFile = yaml_serde::from_str(
        &fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?,
    )
    .map_err(|e| format!("parse {}: {e}", path.display()))?;
    if let Ok(v) = std::env::var("XYLITOL_LIVE_BASE_URL") {
        if !v.is_empty() {
            file.base_url = v;
        }
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_MODEL") {
        if !v.is_empty() {
            file.model = v;
        }
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_API_KEY") {
        if !v.is_empty() {
            file.api_key = v;
        }
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

struct CapTokens {
    max_output_tokens: u64,
}

#[async_trait]
impl HttpHooks for CapTokens {
    async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
        Ok(())
    }

    async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), AiBridgeError> {
        body["max_output_tokens"] = json!(self.max_output_tokens);
        body["store"] = json!(false);
        Ok(())
    }

    async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
}

fn readonly_tools() -> Vec<AiBridgeToolSchema> {
    vec![
        AiBridgeToolSchema {
            name: "get_workspace_info".into(),
            description: "Read-only: return workspace name and lab skill hint (no writes).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "optional filter" }
                },
                "additionalProperties": false
            }),
        },
        AiBridgeToolSchema {
            name: "list_lab_skills".into(),
            description: "Read-only: list available lab skills (simulates skill discovery).".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    ]
}

fn run_readonly_tool(name: &str, _args: &Value) -> String {
    match name {
        "get_workspace_info" => {
            "workspace=xylitol-lab skill=lab-read-only status=ok writable=false".into()
        }
        "list_lab_skills" => "skills: [lab-read-only] — use get_workspace_info for details".into(),
        other => format!("unknown read-only tool: {other}"),
    }
}

struct RoundResult {
    usage: Option<AiBridgeUsage>,
    tool_rounds: usize,
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
    let mut tool_rounds = 0usize;
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

        log_usage(
            label,
            iter,
            &last_usage,
            thinking_signature.is_some(),
            tool_calls.len(),
        );

        if tool_calls.is_empty() {
            break;
        }
        tool_rounds += tool_calls.len();
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

    RoundResult {
        usage: last_usage,
        tool_rounds,
    }
}

fn log_usage(
    label: &str,
    iter: usize,
    usage: &Option<AiBridgeUsage>,
    has_sig: bool,
    n_tools: usize,
) {
    match usage {
        Some(u) => eprintln!(
            "lab: {label} iter={iter} cache_read={} pcr={:?} input={} output={} sig={has_sig} tools={n_tools}",
            u.cache_read, u.prompt_cache_read, u.input, u.output
        ),
        None => eprintln!("lab: {label} iter={iter} usage=None sig={has_sig} tools={n_tools}"),
    }
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
    let cfg = load_cfg()?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let session_tag = format!("c1925_{nonce}");
    let out_dir = PathBuf::from(format!("/tmp/xylitol-lab-resume-cache-{nonce}"));
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let hooks = Arc::new(CapTokens {
        max_output_tokens: cfg.max_output_tokens.max(128),
    });
    let adapter = OpenAiResponsesAdapter::new(
        cfg.api_key.clone(),
        cfg.model.clone(),
        Some(cfg.base_url.clone()),
        Some(hooks.clone()),
    );

    // Session tag lives inside the single system/developer prefix (llama.cpp
    // chat templates reject a second system message mid-input).
    let system = format!(
        "[[lab_session:{session_tag}]]\n\
         You are a careful coding agent in lab mode. \
         Skills available: lab-read-only (read-only workspace facts). \
         Prefer tools get_workspace_info / list_lab_skills when asked about workspace or skills. \
         Never invent write/edit/bash tools. Keep answers short."
    );
    let opts = AiBridgeGenerateOptions {
        thinking_level: std::env::var("XYLITOL_LAB_THINKING").unwrap_or_else(|_| "medium".into()),
        system_prompt: Some(system),
        ..Default::default()
    };
    let tools = readonly_tools();
    let mut history: Vec<AiBridgeMessage> = Vec::new();

    eprintln!("lab: === warm turn 1 (chat) ===");
    let w1 = run_turn(
        &adapter,
        &mut history,
        "Say hi in one short sentence. Do not call tools.",
        &opts,
        &tools,
        "warm1",
        0,
    )
    .await;

    eprintln!("lab: === warm turn 2 (skill + tool) ===");
    let w2 = run_turn(
        &adapter,
        &mut history,
        "Using the lab-read-only skill idea, call list_lab_skills then get_workspace_info, then summarize in one sentence.",
        &opts,
        &tools,
        "warm2",
        4,
    )
    .await;

    eprintln!("lab: === warm turn 3 (chat, same prefix) ===");
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

    let hist_path = out_dir.join("history.json");
    fs::write(
        &hist_path,
        serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    eprintln!("lab: wrote {}", hist_path.display());

    // Simulate process exit: drop adapter, reload history from disk, new client.
    drop(adapter);
    let reloaded: Vec<AiBridgeMessage> =
        serde_json::from_str(&fs::read_to_string(&hist_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("reload history: {e}"))?;
    let sig_count = reloaded
        .iter()
        .filter(|m| {
            m.content().iter().any(|p| {
                matches!(
                    p,
                    AiBridgePart::Thinking {
                        thinking_signature: Some(_),
                        ..
                    }
                )
            })
        })
        .count();
    eprintln!(
        "lab: reloaded {} messages, assistants_with_sig={sig_count}",
        reloaded.len()
    );

    let adapter2 = OpenAiResponsesAdapter::new(
        cfg.api_key.clone(),
        cfg.model.clone(),
        Some(cfg.base_url.clone()),
        Some(hooks),
    );

    let mut hist_replay = reloaded.clone();
    eprintln!("lab: === resume full-replay (new adapter) ===");
    let r_resume = run_turn(
        &adapter2,
        &mut hist_replay,
        "After resume: reply with exactly OK_RESUME",
        &opts,
        &tools,
        "resume_full",
        0,
    )
    .await;

    let summary = json!({
        "session_tag": session_tag,
        "model": cfg.model,
        "base_url": cfg.base_url,
        "thinking_level": opts.thinking_level,
        "replay_strategy": "full_only",
        "warm1_cache": cache_tokens(&w1.usage),
        "warm2_cache": cache_tokens(&w2.usage),
        "warm2_tool_rounds": w2.tool_rounds,
        "warm3_cache": cache_tokens(&w3.usage),
        "resume_full_cache": cache_tokens(&r_resume.usage),
        "resume_full_pcr": format!("{:?}", r_resume.usage.as_ref().map(|u| &u.prompt_cache_read)),
        "assistants_with_sig": sig_count,
    });

    let sum_path = out_dir.join("summary.json");
    fs::write(&sum_path, serde_json::to_string_pretty(&summary).unwrap())
        .map_err(|e| e.to_string())?;
    eprintln!("lab: summary {}", sum_path.display());
    eprintln!("lab: {}", serde_json::to_string_pretty(&summary).unwrap());

    // Soft expectations (lab, not CI assert): warm3 should show some cache if endpoint supports it.
    let warm3_n = cache_tokens(&w3.usage);
    let resume_n = cache_tokens(&r_resume.usage);
    if let Some(u) = &w3.usage {
        match u.prompt_cache_read {
            PromptCacheRead::Tokens(n) if n > 0 => {
                eprintln!("lab: OK warm3 saw Tokens({n}) cache");
            }
            other => eprintln!("lab: NOTE warm3 pcr={other:?} (endpoint may lack prompt cache)"),
        }
    }
    // c1925 gate: full-replay resume must not regress vs same-run warm3 (prefix break).
    // Absolute floors from research §5.2 (medium≥761 / off≥747) may drift with gateway;
    // primary check is resume >= warm3. Print floors for human compare.
    let floor = match opts.thinking_level.as_str() {
        "off" => 747u64,
        _ => 761u64,
    };
    if resume_n < warm3_n {
        return Err(format!(
            "lab FAIL: resume_full cache_read={resume_n} < warm3={warm3_n} (full-replay prefix broken?)"
        ));
    }
    eprintln!(
        "lab: OK resume_full={resume_n} >= warm3={warm3_n} (delta {}); research floor≈{floor} (informational)",
        resume_n.saturating_sub(warm3_n)
    );
    if resume_n < floor {
        eprintln!(
            "lab: NOTE resume_full={resume_n} below historical floor {floor} — check gateway drift vs same-run warm3 before calling regression"
        );
    }

    Ok(())
}
