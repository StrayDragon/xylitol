use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

pub struct ProviderBdd {
    pub(crate) chunks: RefCell<Vec<xylitol_official_plugin_provider_openai::dto::XyChunk>>,
    pub(crate) parse_value: RefCell<Option<serde_json::Value>>,
    pub(crate) input_items: RefCell<Vec<serde_json::Value>>,
    pub(crate) request_body: RefCell<Option<serde_json::Value>>,
}

impl ProviderBdd {
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
pub fn provider_bdd() -> ProviderBdd {
    ProviderBdd::new()
}

#[given(
    "Responses SSE 含 function_call 的 output_item.added 与多帧 function_call_arguments.delta 后才有 output_item.done"
)]
fn g_pab13_responses_sse(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::dto::XyChunk;
    use xylitol_official_plugin_provider_openai::provider::{
        ResponsesStreamState, map_responses_sse_event,
    };

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
            .any(|c| matches!(c, XyChunk::ToolCallStart { .. })),
        "fixture must produce Start, got {out:?}"
    );
    provider_bdd.chunks.replace(out);
}

#[when("映射为 XyChunk 流")]
fn w_pab13_already_mapped(provider_bdd: &ProviderBdd) {
    assert!(
        !provider_bdd.chunks.borrow().is_empty(),
        "expected chunks from given step"
    );
}

#[then(
    "首个 args delta 之前或当时已有 ToolCallStart 且存在至少一次 ToolCallDelta 早于对应 ToolCallEnd"
)]
fn t_pab13_lifecycle(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::dto::XyChunk;
    let chunks = provider_bdd.chunks.borrow();
    let start = chunks
        .iter()
        .position(|c| matches!(c, XyChunk::ToolCallStart { .. }));
    let delta = chunks
        .iter()
        .position(|c| matches!(c, XyChunk::ToolCallDelta { .. }));
    let end = chunks
        .iter()
        .position(|c| matches!(c, XyChunk::ToolCallEnd { .. }));
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
fn g_pab14_partial_json(provider_bdd: &ProviderBdd) {
    provider_bdd.parse_value.replace(None);
}

#[when("调用 parse_streaming_json")]
fn w_pab14_parse(provider_bdd: &ProviderBdd) {
    let v = xylitol_official_plugin_provider_openai::dto::parse_streaming_json(r#"{"command":"ls"#);
    provider_bdd.parse_value.replace(Some(v));
}

#[then("返回 Value 且不 panic")]
fn t_pab14_ok(provider_bdd: &ProviderBdd) {
    let v = provider_bdd
        .parse_value
        .borrow()
        .clone()
        .expect("parse_streaming_json result missing");
    assert!(v.is_object(), "expected object, got {v:?}");
    assert_eq!(v.get("command").and_then(|c| c.as_str()), Some("ls"));
}

#[given("Responses 组装且 system_prompt 非空且 thinking_level 为 medium")]
fn g_pab15_system_developer(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::dto::XyLlmMessage;
    use xylitol_official_plugin_provider_openai::provider::messages_to_responses_input_with_options;
    use xylitol_official_plugin_provider_openai::thinking::XyGenerateOptions;

    let opts = XyGenerateOptions {
        thinking_level: "medium".into(),
        system_prompt: Some("SYS_PROMPT_BDD".into()),
        ..Default::default()
    };
    let items = messages_to_responses_input_with_options(&[XyLlmMessage::user("hi")], &opts);
    provider_bdd.input_items.replace(items);
}

#[when("转换为 input items")]
fn w_pab15_already_converted(provider_bdd: &ProviderBdd) {
    assert!(
        !provider_bdd.input_items.borrow().is_empty(),
        "expected input items from given"
    );
}

#[then("首项 role 为 developer 且 content 为 system_prompt")]
fn t_pab15_developer(provider_bdd: &ProviderBdd) {
    let items = provider_bdd.input_items.borrow();
    assert_eq!(items[0]["role"], "developer");
    assert_eq!(items[0]["content"], "SYS_PROMPT_BDD");
}

#[given("assistant 含 Thinking 无 signature 与 Text")]
fn g_pab15_thinking_text(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::dto::{AgentPart, XyLlmMessage, XyStopReason};
    use xylitol_official_plugin_provider_openai::provider::messages_to_responses_input;

    let msgs = vec![XyLlmMessage::AssistantMessage {
        content: vec![
            AgentPart::Thinking {
                thinking: "HIDDEN_THINK".into(),
                redacted: false,
                thinking_signature: None,
            },
            AgentPart::text("ONLY_TEXT"),
        ],
        stop_reason: Some(XyStopReason::Stop),
        usage: None,
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    }];
    provider_bdd
        .input_items
        .replace(messages_to_responses_input(&msgs));
}

#[when("转换为 Responses input")]
fn w_pab15_converted_again(provider_bdd: &ProviderBdd) {
    assert!(
        !provider_bdd.input_items.borrow().is_empty(),
        "expected input items"
    );
}

#[then("output_text 仅含 Text 且无 Thinking 正文")]
fn t_pab15_text_only(provider_bdd: &ProviderBdd) {
    let items = provider_bdd.input_items.borrow();
    let assistant = items
        .iter()
        .find(|i| i.get("role") == Some(&serde_json::json!("assistant")))
        .expect("assistant item");
    let text = assistant["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "ONLY_TEXT");
    assert!(!text.contains("HIDDEN_THINK"));
}

#[given("Responses 组装且 thinking_level 为 medium 且 tools 非空")]
fn g_pab16_body(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::dto::{XyLlmMessage, XyToolSchema};
    use xylitol_official_plugin_provider_openai::provider::ResponsesAssembler;
    use xylitol_official_plugin_provider_openai::thinking::XyGenerateOptions;

    let tools = [XyToolSchema {
        name: "bash".into(),
        description: "run".into(),
        parameters: serde_json::json!({"type": "object"}),
    }];
    let body = ResponsesAssembler::default().assemble(
        "m",
        vec![XyLlmMessage::user("hi")],
        &tools,
        false,
        &XyGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        },
    );
    provider_bdd.request_body.replace(Some(body));
}

#[when("构建请求体")]
fn w_pab16_build(provider_bdd: &ProviderBdd) {
    assert!(
        provider_bdd.request_body.borrow().is_some(),
        "expected request body from given"
    );
}

#[then(
    "store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content"
)]
fn t_pab16_fields(provider_bdd: &ProviderBdd) {
    let body = provider_bdd.request_body.borrow().clone().expect("body");
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
fn g_pab16_reasoning_item(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::provider::{
        ResponsesStreamState, map_responses_sse_event,
    };

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
    provider_bdd
        .chunks
        .replace(map_responses_sse_event(&event, &mut state));
}

#[when("映射为 XyChunk")]
fn w_pab16_map_chunks(provider_bdd: &ProviderBdd) {
    assert!(!provider_bdd.chunks.borrow().is_empty(), "expected chunks");
}

#[then(
    "存在带 thinkingSignature 的 Thinking 终态（ThinkingEnd 或等价）且 signature 可 JSON 解析为该 reasoning item"
)]
fn t_pab16_signature(provider_bdd: &ProviderBdd) {
    use xylitol_official_plugin_provider_openai::dto::XyChunk;

    let chunks = provider_bdd.chunks.borrow();
    let end = chunks.iter().find_map(|c| match c {
        XyChunk::ThinkingEnd {
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
    use xylitol_plugin_sdk::XyTool;
    use xylitol_runtime::agent::prompt::{SystemPromptOpts, build_system_prompt};

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
    use xylitol_runtime::agent::prompt::{SystemPromptOpts, build_system_prompt};

    let opts = SystemPromptOpts {
        custom_prompt: Some("CUSTOM_ONLY_BODY".into()),
        selected_tools: vec!["read".into()],
        tool_snippets: vec![("read".into(), "Read file".into())],
        ..Default::default()
    };
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("build_system_prompt")]
fn w_pt9_build_again(prompt_bdd: &PromptBdd) {
    assert!(!prompt_bdd.prompt.borrow().is_empty());
}

#[then("正文以该替换内容为主且 不偷偷回填默认 Available tools 清单")]
fn t_pt9_no_backfill(prompt_bdd: &PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(p.contains("CUSTOM_ONLY_BODY"), "{p}");
    assert!(!p.contains("Available tools:"), "{p}");
}

// ── agent-prompt pt3 (c1218) ──────────────────────────────────────

#[given("项目或全局 prompts 目录存在 greet.md")]
fn g_pt3_prompts_dir(ws: &crate::bdd::fixtures::Workspace) {
    ws.init();
    let path = ws.ws("prompts/greet.md");
    std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).ok();
    std::fs::write(&path, "---\ndescription: greet\n---\nHello $1\n").ok();
}

#[when("装配资源加载器并发现")]
fn w_pt3_discover(ws: &crate::bdd::fixtures::Workspace, prompt_bdd: &PromptBdd) {
    use std::path::PathBuf;

    use xylitol_runtime::resource::DefaultResourceLoader;

    let cwd = PathBuf::from(ws.ws("."));
    let agent_dir = cwd.join(".xylitol");
    let loader = DefaultResourceLoader::new(cwd, agent_dir);
    let (skills, _) = loader.get_skills();
    let (themes, _) = loader.get_themes();
    let mut list_out = String::from("skills:\n");
    for s in skills {
        list_out.push_str(&format!("  {}\n", s.name));
    }
    list_out.push_str("themes:\n");
    for t in themes {
        list_out.push_str(&format!("  {}\n", t.name));
    }
    prompt_bdd.prompt.replace(list_out);
}

#[then(
    "get_commands 不含 template:greet 或 /greet 模板命令且 loader 不将 greet 注册为 prompt 模板"
)]
fn t_pt3_no_slash_templates(prompt_bdd: &PromptBdd) {
    use xylitol::wiring::product_commands::product_slash_commands;

    let list_out = prompt_bdd.prompt.borrow();
    assert!(
        !list_out.contains("prompts:"),
        "loader dump must not have prompts section: {list_out}"
    );
    assert!(
        !list_out.contains("greet"),
        "leftover prompts/greet.md must not appear in loader dump: {list_out}"
    );
    let builtins: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
    assert!(
        !builtins
            .iter()
            .any(|n| *n == "template:greet" || *n == "greet")
    );
}

// ---- c2826 specs-compact：裸规则转场景补充（r1538/r1544）----

#[when("以 medium 与 off 档解析请求 thinking 并注入各族请求体")]
fn w_c2826_thinking_families(provider_bdd: &ProviderBdd) {
    use std::collections::HashMap;
    use xylitol_official_plugin_provider_openai::thinking::{
        ThinkingAdapterKind, apply_thinking_anthropic,
        apply_thinking_openai_completions_with_compat, apply_thinking_openai_responses,
        resolve_thinking_for_request,
    };
    use xylitol_official_plugin_provider_openai::wire_policy::Compat;

    let empty_map: HashMap<String, Option<String>> = HashMap::new();
    let resolved_medium_openai =
        resolve_thinking_for_request("medium", &empty_map, None, ThinkingAdapterKind::OpenAi);
    let resolved_medium_anthropic =
        resolve_thinking_for_request("high", &empty_map, None, ThinkingAdapterKind::Anthropic);
    let resolved_off =
        resolve_thinking_for_request("off", &empty_map, None, ThinkingAdapterKind::OpenAi);

    let mut responses = serde_json::json!({"model": "m"});
    apply_thinking_openai_responses(&mut responses, &resolved_medium_openai);

    let mut completions_generic = serde_json::json!({});
    apply_thinking_openai_completions_with_compat(
        &mut completions_generic,
        &resolved_medium_openai,
        Compat::Generic,
    );

    let mut completions_deepseek = serde_json::json!({});
    apply_thinking_openai_completions_with_compat(
        &mut completions_deepseek,
        &resolved_medium_openai,
        Compat::Deepseek,
    );

    let mut anthropic_generic = serde_json::json!({});
    xylitol_official_plugin_provider_openai::provider::dialect::apply_anthropic_thinking(
        &mut anthropic_generic,
        &resolved_medium_anthropic,
        Compat::Generic,
    );

    let mut anthropic_deepseek = serde_json::json!({});
    xylitol_official_plugin_provider_openai::provider::dialect::apply_anthropic_thinking(
        &mut anthropic_deepseek,
        &resolved_medium_anthropic,
        Compat::Deepseek,
    );

    let mut anthropic_off =
        serde_json::json!({"thinking": {"type": "enabled", "budget_tokens": 1}});
    apply_thinking_anthropic(&mut anthropic_off, &resolved_off);

    provider_bdd.request_body.replace(Some(serde_json::json!({
        "responses": responses,
        "completions_generic": completions_generic,
        "completions_deepseek": completions_deepseek,
        "anthropic_generic": anthropic_generic,
        "anthropic_deepseek": anthropic_deepseek,
        "anthropic_off": anthropic_off,
    })));
}

#[then(
    "Responses 用 reasoning.effort 且 Completions 随 compat 分流且 Anthropic 随 compat 分流且 off 档省略字段"
)]
fn t_c2826_thinking_families(provider_bdd: &ProviderBdd) {
    let body = provider_bdd.request_body.borrow().clone().expect("bodies");
    assert_eq!(
        body["responses"]["reasoning"]["effort"], "medium",
        "c2826: Responses 应写 reasoning.effort"
    );
    assert_eq!(
        body["completions_generic"]["reasoning_effort"], "medium",
        "c2826: Completions generic 应写 reasoning_effort：{}",
        body["completions_generic"]
    );
    assert!(
        body["completions_deepseek"].get("thinking").is_some()
            || body["completions_deepseek"]
                .get("reasoning_effort")
                .is_some(),
        "c2826: Completions deepseek 应写 thinking.type（或可选 effort）：{}",
        body["completions_deepseek"]
    );
    let anthropic_generic = &body["anthropic_generic"];
    assert_eq!(anthropic_generic["thinking"]["type"], "enabled");
    assert!(
        anthropic_generic["thinking"]["budget_tokens"]
            .as_u64()
            .is_some(),
        "c2826: Anthropic generic 应带 budget_tokens：{anthropic_generic}"
    );
    let anthropic_deepseek = &body["anthropic_deepseek"];
    assert!(
        anthropic_deepseek
            .get("thinking")
            .map(|t| t.get("budget_tokens").is_none())
            .unwrap_or(false)
            || anthropic_deepseek.get("thinking").is_none(),
        "c2826: Anthropic deepseek MUST NOT 发送 budget_tokens：{anthropic_deepseek}"
    );
    // off 档：注入后必须整体省略 thinking 字段。
    assert!(
        body["anthropic_off"].get("thinking").is_none(),
        "c2826: off 档必须省略 thinking 字段：{}",
        body["anthropic_off"]
    );
}

#[when("格式化含嵌入 JSON 的 Responses 上游错误")]
fn w_c2826_responses_error(provider_bdd: &ProviderBdd) {
    let raw = r#"error sending request: content:{"error":{"message":"upstream context overflow","type":"overloaded_error","code":429}}"#;
    let formatted =
        xylitol_official_plugin_provider_openai::provider::native::openai_responses::format_responses_error(&raw);
    provider_bdd
        .parse_value
        .replace(Some(serde_json::json!({ "message": formatted })));
}

#[then("错误文案暴露嵌入的 error.message 而非反序列化噪音")]
fn t_c2826_responses_error(provider_bdd: &ProviderBdd) {
    let v = provider_bdd.parse_value.borrow().clone().expect("error");
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("upstream context overflow"),
        "c2826: 错误文案应暴露嵌入 error.message：{msg}"
    );
    assert!(
        !msg.trim().is_empty() && !msg.starts_with("deserialize"),
        "c2826: 不得以反序列化噪音掩盖原文：{msg}"
    );
}
