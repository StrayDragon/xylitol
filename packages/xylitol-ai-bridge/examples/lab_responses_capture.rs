//! Lab: dump Responses request bodies (offline + optional live). Formerly
//! `evidence_responses_capture` (c1290); renamed under the unified `lab_` prefix.
//!
//! ```bash
//! # Offline: reconstruct bodies for eeee turn prefixes
//! cargo run -p xylitol-ai-bridge --example lab_responses_capture -- \
//!   offline /tmp/xylitol-c1290-evidence/eeee-history.json /tmp/xylitol-c1290-evidence
//!
//! # Live: one stream call via existing OpenAiResponsesAdapter + HttpHooks capture
//! OPENAI_API_KEY=sk-local cargo run -p xylitol-ai-bridge --example lab_responses_capture -- \
//!   live http://127.0.0.1:8000/v1 my-model /tmp/xylitol-c1290-evidence
//! ```

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value, json};
use xylitol_ai_bridge::dto::{AiBridgeMessage, AiBridgePart, AiBridgeToolSchema};
use xylitol_ai_bridge::hooks::{HeaderBag, HttpHooks};
use xylitol_ai_bridge::provider::{
    AiBridgeLlmAdapter, OpenAiResponsesAdapter, messages_to_responses_input_with_options,
};
use xylitol_ai_bridge::thinking::{
    AiBridgeGenerateOptions, AiBridgeThinkingAdapterKind, apply_thinking_openai_responses,
    resolve_from_options,
};

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!(
            "usage: offline <history.json> <out_dir> | live <base_url> <model> <out_dir> [--probe-pi-fields]"
        );
        std::process::exit(2);
    }
    let mode = args.remove(0);
    match mode.as_str() {
        "offline" => {
            let hist = PathBuf::from(args.first().expect("history.json"));
            let out = PathBuf::from(args.get(1).expect("out_dir"));
            offline_dump(&hist, &out);
        }
        "live" => {
            let base = args.first().expect("base_url").clone();
            let model = args.get(1).expect("model").clone();
            let out = PathBuf::from(args.get(2).expect("out_dir"));
            let probe = args.iter().any(|a| a == "--probe-pi-fields");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(live_capture(base, model, out, probe));
        }
        other => {
            eprintln!("unknown mode {other}");
            std::process::exit(2);
        }
    }
}

#[derive(Clone)]
struct CaptureHooks {
    bodies: Arc<Mutex<Vec<Value>>>,
    probe_pi_fields: bool,
}

#[async_trait]
impl HttpHooks for CaptureHooks {
    async fn before_headers(
        &self,
        _headers: &mut HeaderBag,
    ) -> Result<(), xylitol_ai_bridge::AiBridgeError> {
        Ok(())
    }

    async fn before_request(
        &self,
        _model: &str,
        body: &mut Value,
    ) -> Result<(), xylitol_ai_bridge::AiBridgeError> {
        if self.probe_pi_fields {
            // Experimental: patch toward pi shape before send (compat probe).
            body["store"] = json!(false);
            if let Some(arr) = body.get_mut("tools").and_then(|t| t.as_array_mut()) {
                for t in arr {
                    t["strict"] = json!(false);
                }
            }
            if body.get("reasoning").is_some() {
                body["reasoning"]["summary"] = json!("auto");
                body["include"] = json!(["reasoning.encrypted_content"]);
            }
        }
        self.bodies.lock().unwrap().push(body.clone());
        Ok(())
    }

    async fn after_response(&self, status: u16, _headers: &HeaderBag) {
        let _ = status;
    }
}

fn offline_dump(hist: &PathBuf, out: &PathBuf) {
    fs::create_dir_all(out).expect("mkdir");
    let raw: Value = serde_json::from_str(&fs::read_to_string(hist).expect("read hist")).unwrap();
    let system = raw["system_prompt"].as_str().unwrap_or("").to_string();
    let level = raw["thinking_level"]
        .as_str()
        .unwrap_or("medium")
        .to_string();
    let model = raw["model"].as_str().unwrap_or("model").to_string();
    let all = parse_messages(&raw["messages"]);
    let points: Vec<usize> = raw["call_after_index"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as usize))
        .collect();

    let tools = sample_tools();
    let opts = AiBridgeGenerateOptions {
        thinking_level: level.clone(),
        system_prompt: Some(system.clone()),
        ..Default::default()
    };

    let mut checklist = Vec::new();
    for (turn, &idx) in points.iter().enumerate() {
        let slice = &all[..=idx];
        let body = assemble_body(&model, slice, &tools, true, &opts);
        let path = out.join(format!("offline-turn-{turn:02}-after-{idx}.json"));
        fs::write(&path, serde_json::to_string_pretty(&body).unwrap()).unwrap();
        let c = analyze_body(&body, slice);
        checklist.push(json!({
            "turn": turn,
            "after_index": idx,
            "file": path.file_name().and_then(|s| s.to_str()),
            "last_role": match slice.last() {
                Some(AiBridgeMessage::UserMessage { .. }) => "user",
                Some(AiBridgeMessage::ToolResultMessage { .. }) => "toolResult",
                Some(AiBridgeMessage::AssistantMessage { .. }) => "assistant",
                None => "empty",
            },
            "checklist": c,
        }));
        println!(
            "offline turn {turn:>2} after={idx:>2} → {} | {}",
            path.display(),
            summarize_checklist(&c)
        );
    }
    let summary_path = out.join("offline-checklist.json");
    fs::write(
        &summary_path,
        serde_json::to_string_pretty(&checklist).unwrap(),
    )
    .unwrap();
    println!("wrote {}", summary_path.display());
}

async fn live_capture(base: String, model: String, out: PathBuf, probe: bool) {
    fs::create_dir_all(&out).expect("mkdir");
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let hooks = Arc::new(CaptureHooks {
        bodies: bodies.clone(),
        probe_pi_fields: probe,
    });
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-local".into());
    let adapter = OpenAiResponsesAdapter::new(api_key, model.clone(), Some(base), Some(hooks));

    let prompt = "你能执行以下任务首先编写一个 a.py 写一个 quick sort 并且运行, 然后 sleep 3 后继续调用下 a.py 然后编辑 a.py 改为归并排序, 然后 运行 最后删除这个 a.py (我们测试下流程)";
    let opts = AiBridgeGenerateOptions {
        thinking_level: "medium".into(),
        system_prompt: Some(
            "You are an expert coding assistant.\n\nAvailable tools:\n- bash: Execute bash\n- write: Write files\n- edit: Edit files\n- read: Read files"
                .into(),
        ),
        ..Default::default()
    };
    let tools = sample_tools();
    let msgs = vec![AiBridgeMessage::user(prompt)];

    println!("live generate_stream model={model} probe_pi_fields={probe} …");
    match adapter.generate_stream(msgs, &tools, opts).await {
        Ok(mut stream) => {
            let mut n = 0usize;
            let mut variants = std::collections::BTreeMap::<String, usize>::new();
            while let Some(item) = stream.next().await {
                n += 1;
                match item {
                    Ok(c) => {
                        let k = format!("{c:?}");
                        let key = k.split('(').next().unwrap_or("?").to_string();
                        *variants.entry(key).or_default() += 1;
                    }
                    Err(e) => {
                        eprintln!("stream err: {e}");
                        break;
                    }
                }
                if n > 5000 {
                    break;
                }
            }
            println!("stream chunks={n} variants={variants:?}");
        }
        Err(e) => eprintln!("generate_stream failed: {e}"),
    }

    let captured = bodies.lock().unwrap().clone();
    for (i, body) in captured.iter().enumerate() {
        let path = out.join(format!(
            "live-req-{i:02}{}.json",
            if probe { "-probe-pi" } else { "" }
        ));
        // redact nothing sensitive in body (no api key in JSON body)
        fs::write(&path, serde_json::to_string_pretty(body).unwrap()).unwrap();
        let c = analyze_body(body, &[]);
        println!(
            "live req {i} → {} | {}",
            path.display(),
            summarize_checklist(&c)
        );
        fs::write(
            out.join(format!("live-req-{i:02}-checklist.json")),
            serde_json::to_string_pretty(&c).unwrap(),
        )
        .unwrap();
    }
    if captured.is_empty() {
        eprintln!("WARNING: no request bodies captured (hooks not invoked?)");
    }
}

fn assemble_body(
    model: &str,
    messages: &[AiBridgeMessage],
    tools: &[AiBridgeToolSchema],
    stream: bool,
    options: &AiBridgeGenerateOptions,
) -> Value {
    let input_items = messages_to_responses_input_with_options(messages, options);
    let mut body = json!({
        "model": model,
        "input": input_items,
        "stream": stream,
    });
    if !tools.is_empty() {
        let tool_defs: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            })
            .collect();
        body["tools"] = Value::Array(tool_defs);
    }
    let resolved = resolve_from_options(options, AiBridgeThinkingAdapterKind::OpenAi);
    apply_thinking_openai_responses(&mut body, &resolved);
    body
}

fn analyze_body(body: &Value, messages: &[AiBridgeMessage]) -> Value {
    let input = body.get("input").and_then(|v| v.as_array());
    let first_role = input
        .and_then(|a| a.first())
        .and_then(|i| i.get("role"))
        .and_then(|r| r.as_str());
    let has_developer = input
        .map(|a| {
            a.iter()
                .any(|i| i.get("role").and_then(|r| r.as_str()) == Some("developer"))
        })
        .unwrap_or(false);
    let has_system_role = input
        .map(|a| {
            a.iter()
                .any(|i| i.get("role").and_then(|r| r.as_str()) == Some("system"))
        })
        .unwrap_or(false);
    let reasoning_items = input
        .map(|a| {
            a.iter()
                .filter(|i| i.get("type").and_then(|t| t.as_str()) == Some("reasoning"))
                .count()
        })
        .unwrap_or(0);
    let thinking_parts = messages
        .iter()
        .filter_map(|m| match m {
            AiBridgeMessage::AssistantMessage { content, .. } => Some(content),
            _ => None,
        })
        .flatten()
        .filter(|p| matches!(p, AiBridgePart::Thinking { .. }))
        .count();
    let thinking_with_sig = messages
        .iter()
        .filter_map(|m| match m {
            AiBridgeMessage::AssistantMessage { content, .. } => Some(content),
            _ => None,
        })
        .flatten()
        .filter(|p| {
            matches!(
                p,
                AiBridgePart::Thinking {
                    thinking_signature: Some(_),
                    ..
                }
            )
        })
        .count();
    let tools = body.get("tools").and_then(|t| t.as_array());
    let any_strict_false = tools
        .map(|a| a.iter().any(|t| t.get("strict") == Some(&json!(false))))
        .unwrap_or(false);
    let any_strict_field = tools
        .map(|a| a.iter().any(|t| t.get("strict").is_some()))
        .unwrap_or(false);

    json!({
        "first_input_role": first_role,
        "has_developer": has_developer,
        "has_system_role": has_system_role,
        "store": body.get("store"),
        "include": body.get("include"),
        "reasoning": body.get("reasoning"),
        "tools_count": tools.map(|a| a.len()).unwrap_or(0),
        "tools_any_strict_field": any_strict_field,
        "tools_any_strict_false": any_strict_false,
        "input_reasoning_items": reasoning_items,
        "history_thinking_parts": thinking_parts,
        "history_thinking_with_signature": thinking_with_sig,
        "pi_gap_store_false": body.get("store") != Some(&json!(false)),
        "pi_gap_include_encrypted": body.get("include").is_none(),
        "pi_gap_reasoning_summary": body.get("reasoning").and_then(|r| r.get("summary")).is_none(),
        "pi_gap_strict_false": !any_strict_false,
        "pi_gap_thinking_replay": thinking_parts > 0 && reasoning_items == 0 && thinking_with_sig == 0,
    })
}

fn summarize_checklist(c: &Value) -> String {
    format!(
        "dev={} store={:?} include={:?} reasoning={:?} strict_false={} think_parts={} replay_items={} gap_replay={}",
        c["has_developer"],
        c["store"],
        c["include"],
        c["reasoning"],
        c["tools_any_strict_false"],
        c["history_thinking_parts"],
        c["input_reasoning_items"],
        c["pi_gap_thinking_replay"],
    )
}

fn sample_tools() -> Vec<AiBridgeToolSchema> {
    vec![
        AiBridgeToolSchema {
            name: "bash".into(),
            description: "Execute bash".into(),
            parameters: json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}),
        },
        AiBridgeToolSchema {
            name: "write".into(),
            description: "Write file".into(),
            parameters: json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}),
        },
        AiBridgeToolSchema {
            name: "edit".into(),
            description: "Edit file".into(),
            parameters: json!({"type":"object","properties":{"path":{"type":"string"},"edits":{"type":"array"}},"required":["path","edits"]}),
        },
        AiBridgeToolSchema {
            name: "read".into(),
            description: "Read file".into(),
            parameters: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        },
    ]
}

fn parse_messages(v: &Value) -> Vec<AiBridgeMessage> {
    let mut out = Vec::new();
    for m in v.as_array().unwrap() {
        match m.get("role").and_then(|r| r.as_str()).unwrap_or("") {
            "user" => {
                let text = m["content"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("");
                out.push(AiBridgeMessage::user(text));
            }
            "assistant" => {
                let mut parts = Vec::new();
                for p in m["content"].as_array().unwrap() {
                    match p.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                        "Thinking" | "thinking" => {
                            parts.push(AiBridgePart::Thinking {
                                thinking: p["thinking"].as_str().unwrap_or("").into(),
                                redacted: false,
                                thinking_signature: p
                                    .get("thinkingSignature")
                                    .and_then(|s| s.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(str::to_string),
                            });
                        }
                        "Text" | "text" => {
                            parts.push(AiBridgePart::text(p["text"].as_str().unwrap_or("")));
                        }
                        "ToolCall" | "toolCall" => {
                            parts.push(AiBridgePart::ToolCall {
                                id: p["id"].as_str().unwrap_or("").into(),
                                name: p["name"].as_str().unwrap_or("").into(),
                                arguments: p.get("arguments").cloned().unwrap_or(json!({})),
                            });
                        }
                        _ => {}
                    }
                }
                out.push(AiBridgeMessage::AssistantMessage {
                    content: parts,
                    stop_reason: None,
                    usage: None,
                    api: "openai-responses".into(),
                    provider: "evidence".into(),
                    model: String::new(),
                    response_id: None,
                    error_message: None,
                    timestamp: 0,
                    diagnostics: Vec::new(),
                });
            }
            "toolResult" => {
                let text = m["content"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("");
                out.push(AiBridgeMessage::tool_result(
                    m["tool_use_id"].as_str().unwrap_or("unknown"),
                    m["tool_name"].as_str().unwrap_or(""),
                    vec![AiBridgePart::text(text)],
                    false,
                ));
            }
            _ => {}
        }
    }
    out
}
