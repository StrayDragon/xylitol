use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

pub struct AiBridgeBdd {
    pub(crate) chunks: RefCell<Vec<xylitol_ai_bridge::dto::AiBridgeChunk>>,
    pub(crate) parse_value: RefCell<Option<serde_json::Value>>,
    pub(crate) input_items: RefCell<Vec<serde_json::Value>>,
    pub(crate) request_body: RefCell<Option<serde_json::Value>>,
}

impl AiBridgeBdd {
    fn new() -> Self {
        Self {
            chunks: RefCell::new(Vec::new()),
            parse_value: RefCell::new(None),
            input_items: RefCell::new(Vec::new()),
            request_body: RefCell::new(None),
        }
    }
}

#[fixture]
pub fn ai_bridge_bdd() -> AiBridgeBdd {
    AiBridgeBdd::new()
}

#[given(
    "Responses SSE 含 function_call 的 output_item.added 与多帧 function_call_arguments.delta 后才有 output_item.done"
)]
fn g_pab13_responses_sse(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeChunk;
    use xylitol_ai_bridge::provider::{ResponsesStreamState, map_responses_sse_event};

    let mut state = ResponsesStreamState::default();
    let mut out = Vec::new();
    let events = [
        serde_json::json!({
            "type": "response.output_item.added",
            "item": { "type": "function_call", "id": "fc_bdd", "name": "ls", "arguments": "" }
        }),
        serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_bdd",
            "delta": "{\"path\":"
        }),
        serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_bdd",
            "delta": "\"/tmp\"}"
        }),
        serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "function_call",
                "id": "fc_bdd",
                "name": "ls",
                "arguments": "{\"path\":\"/tmp\"}"
            }
        }),
    ];
    for ev in events {
        out.extend(map_responses_sse_event(&ev, &mut state));
    }
    assert!(
        out.iter()
            .any(|c| matches!(c, AiBridgeChunk::ToolCallStart { .. })),
        "fixture must produce Start, got {out:?}"
    );
    ai_bridge_bdd.chunks.replace(out);
}

#[when("映射为 AiBridgeChunk 流")]
fn w_pab13_already_mapped(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        !ai_bridge_bdd.chunks.borrow().is_empty(),
        "expected chunks from given step"
    );
}

#[then(
    "首个 args delta 之前或当时已有 ToolCallStart 且存在至少一次 ToolCallDelta 早于对应 ToolCallEnd"
)]
fn t_pab13_lifecycle(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeChunk;
    let chunks = ai_bridge_bdd.chunks.borrow();
    let start = chunks
        .iter()
        .position(|c| matches!(c, AiBridgeChunk::ToolCallStart { .. }));
    let delta = chunks
        .iter()
        .position(|c| matches!(c, AiBridgeChunk::ToolCallDelta { .. }));
    let end = chunks
        .iter()
        .position(|c| matches!(c, AiBridgeChunk::ToolCallEnd { .. }));
    assert!(start.is_some(), "missing ToolCallStart in {chunks:?}");
    assert!(delta.is_some(), "missing ToolCallDelta in {chunks:?}");
    assert!(end.is_some(), "missing ToolCallEnd in {chunks:?}");
    let (s, d, e) = (start.unwrap(), delta.unwrap(), end.unwrap());
    assert!(
        s <= d && d < e,
        "expected Start<=Delta<End, got Start={s} Delta={d} End={e} chunks={chunks:?}"
    );
}

#[given("输入残缺工具参数 JSON")]
fn g_pab14_partial_json(ai_bridge_bdd: &AiBridgeBdd) {
    ai_bridge_bdd.parse_value.replace(None);
}

#[when("调用 parse_streaming_json")]
fn w_pab14_parse(ai_bridge_bdd: &AiBridgeBdd) {
    let v = xylitol_ai_bridge::dto::parse_streaming_json(r#"{"command":"ls"#);
    ai_bridge_bdd.parse_value.replace(Some(v));
}

#[then("返回 Value 且不 panic")]
fn t_pab14_ok(ai_bridge_bdd: &AiBridgeBdd) {
    let v = ai_bridge_bdd
        .parse_value
        .borrow()
        .clone()
        .expect("parse_streaming_json result missing");
    assert!(v.is_object(), "expected object, got {v:?}");
    assert_eq!(v.get("command").and_then(|c| c.as_str()), Some("ls"));
}

#[given("Responses 组装且 system_prompt 非空且 thinking_level 为 medium")]
fn g_pab15_system_developer(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeMessage;
    use xylitol_ai_bridge::provider::messages_to_responses_input_with_options;
    use xylitol_ai_bridge::thinking::AiBridgeGenerateOptions;

    let opts = AiBridgeGenerateOptions {
        thinking_level: "medium".into(),
        system_prompt: Some("SYS_PROMPT_BDD".into()),
        ..Default::default()
    };
    let items = messages_to_responses_input_with_options(&[AiBridgeMessage::user("hi")], &opts);
    ai_bridge_bdd.input_items.replace(items);
}

#[when("转换为 input items")]
fn w_pab15_already_converted(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        !ai_bridge_bdd.input_items.borrow().is_empty(),
        "expected input items from given"
    );
}

#[then("首项 role 为 developer 且 content 为 system_prompt")]
fn t_pab15_developer(ai_bridge_bdd: &AiBridgeBdd) {
    let items = ai_bridge_bdd.input_items.borrow();
    assert_eq!(items[0]["role"], "developer");
    assert_eq!(items[0]["content"], "SYS_PROMPT_BDD");
}

#[given("assistant 含 Thinking 无 signature 与 Text")]
fn g_pab15_thinking_text(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason};
    use xylitol_ai_bridge::provider::messages_to_responses_input;

    let msgs = vec![AiBridgeMessage::AssistantMessage {
        content: vec![
            AiBridgePart::Thinking {
                thinking: "HIDDEN_THINK".into(),
                redacted: false,
                thinking_signature: None,
            },
            AiBridgePart::text("ONLY_TEXT"),
        ],
        stop_reason: Some(AiBridgeStopReason::Stop),
        usage: None,
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    }];
    ai_bridge_bdd
        .input_items
        .replace(messages_to_responses_input(&msgs));
}

#[when("转换为 Responses input")]
fn w_pab15_converted_again(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        !ai_bridge_bdd.input_items.borrow().is_empty(),
        "expected input items"
    );
}

#[then("output_text 仅含 Text 且无 Thinking 正文")]
fn t_pab15_text_only(ai_bridge_bdd: &AiBridgeBdd) {
    let items = ai_bridge_bdd.input_items.borrow();
    let assistant = items
        .iter()
        .find(|i| i.get("role") == Some(&serde_json::json!("assistant")))
        .expect("assistant item");
    let text = assistant["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "ONLY_TEXT");
    assert!(!text.contains("HIDDEN_THINK"));
}

#[given("Responses 组装且 thinking_level 为 medium 且 tools 非空")]
fn g_pab16_body(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::{AiBridgeMessage, AiBridgeToolSchema};
    use xylitol_ai_bridge::provider::assemble_responses_body;
    use xylitol_ai_bridge::thinking::AiBridgeGenerateOptions;

    let tools = [AiBridgeToolSchema {
        name: "bash".into(),
        description: "run".into(),
        parameters: serde_json::json!({"type": "object"}),
    }];
    let body = assemble_responses_body(
        "m",
        vec![AiBridgeMessage::user("hi")],
        &tools,
        false,
        &AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        },
        &xylitol_ai_bridge::WirePolicy::default(),
    );
    ai_bridge_bdd.request_body.replace(Some(body));
}

#[when("构建请求体")]
fn w_pab16_build(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(
        ai_bridge_bdd.request_body.borrow().is_some(),
        "expected request body from given"
    );
}

#[then(
    "store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content"
)]
fn t_pab16_fields(ai_bridge_bdd: &AiBridgeBdd) {
    let body = ai_bridge_bdd.request_body.borrow().clone().expect("body");
    assert_eq!(body["store"], false);
    let tools = body["tools"].as_array().expect("tools");
    assert!(!tools.is_empty());
    for t in tools {
        assert_eq!(t["strict"], false, "tool strict: {t}");
    }
    assert_eq!(body["reasoning"]["summary"], "auto");
    let include = body["include"].as_array().expect("include");
    assert!(
        include
            .iter()
            .any(|v| v.as_str() == Some("reasoning.encrypted_content")),
        "include={include:?}"
    );
}

#[given("Responses 流或非流输出含完整 type=reasoning 的 output item")]
fn g_pab16_reasoning_item(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::provider::{ResponsesStreamState, map_responses_sse_event};

    let mut state = ResponsesStreamState::default();
    let event = serde_json::json!({
        "type": "response.output_item.done",
        "item": {
            "type": "reasoning",
            "id": "rs_bdd",
            "summary": [{"type": "summary_text", "text": "think"}],
            "encrypted_content": "blob"
        }
    });
    ai_bridge_bdd
        .chunks
        .replace(map_responses_sse_event(&event, &mut state));
}

#[when("映射为 AiBridgeChunk")]
fn w_pab16_map_chunks(ai_bridge_bdd: &AiBridgeBdd) {
    assert!(!ai_bridge_bdd.chunks.borrow().is_empty(), "expected chunks");
}

#[then(
    "存在带 thinkingSignature 的 Thinking 终态（ThinkingEnd 或等价）且 signature 可 JSON 解析为该 reasoning item"
)]
fn t_pab16_signature(ai_bridge_bdd: &AiBridgeBdd) {
    use xylitol_ai_bridge::dto::AiBridgeChunk;

    let chunks = ai_bridge_bdd.chunks.borrow();
    let end = chunks.iter().find_map(|c| match c {
        AiBridgeChunk::ThinkingEnd {
            thinking_signature: Some(sig),
            ..
        } => Some(sig.clone()),
        _ => None,
    });
    let sig = end.expect("ThinkingEnd with signature");
    let parsed: serde_json::Value = serde_json::from_str(&sig).expect("signature JSON");
    assert_eq!(parsed["type"], "reasoning");
    assert_eq!(parsed["id"], "rs_bdd");
    assert_eq!(parsed["encrypted_content"], "blob");
}

// ── agent-prompt pt9 (c1290) ──────────────────────────────────────

pub struct PromptBdd {
    pub(crate) prompt: RefCell<String>,
}

impl PromptBdd {
    fn new() -> Self {
        Self {
            prompt: RefCell::new(String::new()),
        }
    }
}

#[fixture]
pub fn prompt_bdd() -> PromptBdd {
    PromptBdd::new()
}

#[given("工具集含 bash 且其 prompt_guidelines 非空")]
fn g_pt9_bash_guidelines(prompt_bdd: &PromptBdd) {
    use xylitol::agent::prompt::{SystemPromptOpts, build_system_prompt};
    use xylitol::protocol::ports::XyTool;

    let bash = BashTool::default();
    assert!(
        !bash.prompt_guidelines().is_empty(),
        "bash guidelines must be non-empty"
    );
    let opts = SystemPromptOpts {
        prompt_guidelines: bash
            .prompt_guidelines()
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        cwd: ".".into(),
        ..Default::default()
    };
    // Store guidelines for when step; also prebuild for convenience
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("set_tools 或等价装配后 build_system_prompt")]
fn w_pt9_build(prompt_bdd: &PromptBdd) {
    assert!(
        !prompt_bdd.prompt.borrow().is_empty(),
        "expected prompt from given"
    );
}

#[then("输出含 Guidelines 段且含该工具 guideline 短句")]
fn t_pt9_guidelines(prompt_bdd: &PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(p.contains("Guidelines:"), "{p}");
    assert!(
        p.contains("Prefer specialized read/edit/write tools") || p.contains("bash"),
        "expected bash guideline in {p}"
    );
}

#[given("custom_prompt 或 SYSTEM.md 整段替换默认正文且未附 Available tools")]
fn g_pt9_custom(prompt_bdd: &PromptBdd) {
    use xylitol::agent::prompt::{SystemPromptOpts, build_system_prompt};

    let opts = SystemPromptOpts {
        custom_prompt: Some("CUSTOM_ONLY_BODY".into()),
        selected_tools: vec!["read".into()],
        tool_snippets: vec![("read".into(), "Read file".into())],
        cwd: ".".into(),
        ..Default::default()
    };
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("build_system_prompt")]
fn w_pt9_build_again(prompt_bdd: &PromptBdd) {
    assert!(!prompt_bdd.prompt.borrow().is_empty());
}

#[then("正文以该替换内容为主且 MUST NOT 偷偷回填默认 Available tools 清单")]
fn t_pt9_no_backfill(prompt_bdd: &PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(p.contains("CUSTOM_ONLY_BODY"), "{p}");
    assert!(!p.contains("Available tools:"), "{p}");
}

// ── agent-prompt pt3 (c1218) ──────────────────────────────────────

#[given("项目或全局 prompts 目录存在 greet.md")]
fn g_pt3_prompts_dir(ws: &crate::fixtures::Workspace) {
    ws.init();
    let path = ws.ws("prompts/greet.md");
    std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).ok();
    std::fs::write(&path, "---\ndescription: greet\n---\nHello $1\n").ok();
}

#[when("装配 AgentSession 或 ResourceLoader 发现")]
fn w_pt3_discover(ws: &crate::fixtures::Workspace, prompt_bdd: &PromptBdd) {
    use std::path::PathBuf;

    use xylitol::app::cli::resources::{ResourcesAction, run_with_dirs};

    let cwd = PathBuf::from(ws.ws("."));
    let agent_dir = cwd.join(".xylitol");
    let (code, list_out) = run_with_dirs(ResourcesAction::List, &cwd, &agent_dir);
    assert_eq!(code, std::process::ExitCode::SUCCESS);
    prompt_bdd.prompt.replace(list_out);
}

#[then(
    "get_commands MUST NOT 含 template:greet 或 /greet 模板命令且 loader MUST NOT 将 greet 注册为 prompt 模板"
)]
fn t_pt3_no_slash_templates(prompt_bdd: &PromptBdd) {
    use xylitol::agent::prompt::product_commands::product_slash_commands;

    let list_out = prompt_bdd.prompt.borrow();
    assert!(
        !list_out.contains("prompts:"),
        "resources list must not have prompts section: {list_out}"
    );
    assert!(
        !list_out.contains("greet"),
        "leftover prompts/greet.md must not appear in resources list: {list_out}"
    );
    let builtins: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
    assert!(
        !builtins
            .iter()
            .any(|n| *n == "template:greet" || *n == "greet")
    );
}
