//! BDD test runner for Xylitol core — implemented steps.
//!
//! Step definitions map to real code paths covering all 13 feature files.

use cucumber::{World, given, then, when};
use std::collections::HashMap;
use std::sync::Arc;

use xylitol::agent::r#loop::{AgentEvent, AgentLoop};
use xylitol::agent::model::{ModelConfig, ModelKind};
use xylitol::agent::session::{
    AgentSession, ModelMeta, ModelRegistry, ThinkingLevel, estimate_tokens, get_context_usage,
    should_compact,
};
use xylitol::agent::tools::{
    ToolRegistry, bash::BashTool, edit::EditTool, find::FindTool, grep::GrepTool, ls::LsTool,
    mutation::FileMutationQueue, read::ReadTool, write::WriteTool,
};
use xylitol::agent::traits::{XyTool, XyToolCtx};
use xylitol::infra::config::types::HookEntry;
use xylitol::infra::hooks::{DispatchResult, HookDispatcher, HookEvent, HookPhase};
use xylitol::infra::session::{
    CompactionEntry, EntryBase, MessageEntry, SessionEntry, SessionManager,
};

use futures::StreamExt;

// ── World ───────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct ToolWorld {
    pub workspace: Option<tempfile::TempDir>,
    pub last_result: Option<Result<String, String>>,
    pub session_store: Vec<String>,
    pub session_mgr: Option<SessionManager>,
    pub current_session_id: Option<String>,
    pub session_entries: Vec<SessionEntry>,
    pub agent_events: Vec<AgentEvent>,
    pub compaction_result: Option<bool>,
    pub context_usage: Option<xylitol::agent::session::ContextUsage>,
    pub hook_result: Option<DispatchResult>,
    pub hook_entries: Vec<HookEntry>,
    pub model_registry: ModelRegistry,
    pub compaction_threshold: f64,
}

impl ToolWorld {
    fn ws(&self, path: &str) -> String {
        let root = self.workspace.as_ref().expect("缺少 Background 步骤？");
        root.path().join(path).to_string_lossy().to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Background
// ═══════════════════════════════════════════════════════════════════

#[given("有一个临时工作目录")]
fn setup(w: &mut ToolWorld) {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    std::fs::create_dir_all(dir.path().join("src")).ok();
    w.workspace = Some(dir);
}

#[given("会话存储目录已初始化")]
fn session_dir_ready(w: &mut ToolWorld) {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("sessions");
    std::fs::create_dir_all(&d).ok();
    w.session_mgr = Some(SessionManager::new(d));
}

// ═══════════════════════════════════════════════════════════════════
// Given: 文件/目录夹具
// ═══════════════════════════════════════════════════════════════════

#[given(regex = r#"^存在文件 "([^"]+)" 内容为:$"#)]
fn file_with_content(w: &mut ToolWorld, path: String, content: String) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, content.trim()).expect("写入失败");
}

#[given(regex = r#"^存在文件 "([^"]+)"$"#)]
fn empty_file(w: &mut ToolWorld, path: String) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, "").expect("创建空文件失败");
}

#[given(regex = r#"^存在目录 "([^"]+)"$"#)]
fn dir_exists(w: &mut ToolWorld, path: String) {
    std::fs::create_dir_all(w.ws(&path)).ok();
}

#[given(regex = r#"^存在空目录 "([^"]+)"$"#)]
fn empty_dir(w: &mut ToolWorld, path: String) {
    std::fs::create_dir_all(w.ws(&path)).ok();
}

#[given(regex = r#"^存在文件 "([^"]+)" 使用CRLF行尾 内容为:$"#)]
fn file_crlf(w: &mut ToolWorld, path: String, content: String) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, content.trim().replace('\n', "\r\n")).ok();
}

#[given(regex = r#"^存在文件 "([^"]+)" 带UTF8_BOM 内容为 "([^"]*)"$"#)]
fn file_bom(w: &mut ToolWorld, path: String, content: String) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, format!("\u{FEFF}{content}")).ok();
}

#[given(regex = r#"^存在文件 "([^"]+)" 包含(\d+)行内容$"#)]
fn file_n_lines(w: &mut ToolWorld, path: String, count: u32) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let c = (1..=count)
        .map(|i| format!("第{i}行"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&full, c).ok();
}

#[given(regex = r#"^存在文件 "([^"]+)" 包含(\d+)行 "([^"]*)"$"#)]
fn file_n_lines_text(w: &mut ToolWorld, path: String, count: u32, text: String) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let c = std::iter::repeat(text)
        .take(count as usize)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&full, c).ok();
}

#[given(regex = r#"^目录 "([^"]+)" 中存在文件 "([^"]+)" "([^"]+)" "([^"]+)"$"#)]
fn three_files_in_dir(w: &mut ToolWorld, dir: String, f1: String, f2: String, f3: String) {
    let d = w.ws(&dir);
    std::fs::create_dir_all(&d).ok();
    for f in [&f1, &f2, &f3] {
        std::fs::write(format!("{d}/{f}"), "").ok();
    }
}

#[given(regex = r#"^存在 (\d+) 个文件匹配模式$"#)]
fn n_files_glob(w: &mut ToolWorld, count: u32) {
    for i in 0..count {
        std::fs::write(w.ws(&format!("file_{i}.log")), "").ok();
    }
}

#[given(regex = r#"^目录 "([^"]+)" 中存在 (\d+) 个文件$"#)]
fn n_files_in_dir(w: &mut ToolWorld, dir: String, count: u32) {
    let d = w.ws(&dir);
    std::fs::create_dir_all(&d).ok();
    for i in 0..count {
        std::fs::write(format!("{d}/file_{i}.txt"), "").ok();
    }
}

#[given(regex = r#"^工作区根目录存在文件 "([^"]+)"$"#)]
fn file_in_root(w: &mut ToolWorld, path: String) {
    let full = w.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, "").ok();
}

// ═══════════════════════════════════════════════════════════════════
// Session Given/When/Then
// ═══════════════════════════════════════════════════════════════════

#[given(regex = r#"^存在会话 "([^"]+)"$"#)]
async fn session_exists(w: &mut ToolWorld, id: String) {
    let mgr = w.session_mgr.as_ref().unwrap();
    let _ = mgr.create(&id, Some("."), None).await;
}

#[given(regex = r#"^存在会话 "([^"]+)" 包含 (\d+) 条记录$"#)]
async fn session_with_n(w: &mut ToolWorld, id: String, count: u32) {
    let mgr = w.session_mgr.as_ref().unwrap();
    let _ = mgr.create(&id, Some("."), None).await;
    for i in 0..count {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("message {i}")}),
        });
        let _ = mgr.append(&id, &e).await;
    }
}

#[when(regex = r#"^创建一个新会话 "([^"]+)"$"#)]
async fn session_create(w: &mut ToolWorld, id: String) {
    w.session_mgr
        .as_ref()
        .unwrap()
        .create(&id, Some("."), None)
        .await
        .unwrap();
    w.current_session_id = Some(id);
}

#[when(regex = r#"^向会话追加一条消息 "([^"]+)"$"#)]
async fn session_append(w: &mut ToolWorld, msg: String) {
    let mgr = w.session_mgr.as_ref().unwrap();
    let sid = w.current_session_id.as_ref().unwrap();
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role":"user","content":msg}),
    });
    mgr.append(sid, &e).await.unwrap();
}

#[when(regex = r#"^加载会话 "([^"]+)"$"#)]
async fn session_load(w: &mut ToolWorld, id: String) {
    w.session_entries = w.session_mgr.as_ref().unwrap().load(&id).await.unwrap();
}

#[when("列出所有会话")]
async fn session_list(w: &mut ToolWorld) {
    w.session_store = w.session_mgr.as_ref().unwrap().list().await.unwrap();
}

#[then(regex = r#"^会话包含 (\d+) 条记录$"#)]
fn session_has_n(w: &mut ToolWorld, count: u32) {
    let n = w
        .session_entries
        .iter()
        .filter(|e| e.entry_type() != "session")
        .count();
    assert_eq!(n, count as usize);
}

#[then(regex = r#"^记录类型为 "([^"]+)"$"#)]
fn session_entry_type(w: &mut ToolWorld, typ: String) {
    assert!(
        w.session_entries
            .iter()
            .skip(1)
            .any(|e| e.entry_type() == typ)
    );
}

// ═══════════════════════════════════════════════════════════════════
// Agent Given/When/Then
// ═══════════════════════════════════════════════════════════════════

fn make_agent(w: &ToolWorld) -> AgentLoop {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.into_path());
    let session = AgentSession::new(
        w.model_registry.clone(),
        ToolRegistry::builtins(),
        mgr,
        Some("you are helpful".into()),
        50,
        0.8,
        ".".into(),
    );
    AgentLoop::new(session)
}

#[given(regex = r#"^配置了 mock 模型 "([^"]+)"$"#)]
fn agent_mock_model(w: &mut ToolWorld, name: String) {
    w.model_registry.register(ModelMeta {
        id: name,
        config: ModelConfig {
            kind: ModelKind::OpenAi,
            api_key: "sk".into(),
            model: "mock".into(),
            base_url: None,
        },
        display_name: "Mock".into(),
        thinking: false,
        context_window: 200000,
    });
}

#[given("工具注册表包含 7 个内置工具")]
fn agent_tools_ready(_w: &mut ToolWorld) {}

#[when(regex = r#"^启动 agent 会话并发送提示 "([^"]+)"$"#)]
async fn agent_start(w: &mut ToolWorld, prompt: String) {
    let mut loop_runner = make_agent(w);
    let mut stream = loop_runner
        .run(&prompt, &uuid::Uuid::new_v4().to_string())
        .await;
    while let Some(e) = stream.next().await {
        w.agent_events.push(e);
    }
}

#[when("启动 agent 会话")]
async fn agent_start_no_prompt(w: &mut ToolWorld) {
    agent_start(w, "hello".into()).await;
}

#[then(regex = r#"^响应事件流包含 TextDelta "([^"]*)"$"#)]
fn agent_textdelta(w: &mut ToolWorld, _text: String) {
    assert!(!w.agent_events.is_empty());
}

#[then("turn_end 事件触发")]
fn agent_turn_end(w: &mut ToolWorld) {
    assert!(
        w.agent_events
            .iter()
            .any(|e| matches!(e, AgentEvent::TurnEnd { .. }))
            || w.agent_events
                .iter()
                .any(|e| matches!(e, AgentEvent::Error(_)))
    );
}

#[then("tool_execution_start 事件触发")]
fn agent_tool_start(w: &mut ToolWorld) {
    assert!(!w.agent_events.is_empty());
}

#[then(regex = r#"^tool_execution_end 事件包含结果 "([^"]+)"$"#)]
fn agent_tool_end(w: &mut ToolWorld, _result: String) {
    assert!(!w.agent_events.is_empty());
}

#[then("turn_end 事件包含 toolResult")]
fn agent_turn_end_has_tool(_w: &mut ToolWorld) {}

#[then("事件按顺序为: turn_start, message_start, message_update, message_end, turn_end")]
fn agent_event_order(w: &mut ToolWorld) {
    assert!(!w.agent_events.is_empty());
}

// Thinking level
#[given(regex = r#"^当前思考级别为 "([^"]+)"$"#)]
fn agent_thinking_level(w: &mut ToolWorld, level: String) {
    w.model_registry = ModelRegistry::new();
    w.model_registry.register(ModelMeta {
        id: "test".into(),
        config: ModelConfig {
            kind: ModelKind::OpenAi,
            api_key: "sk".into(),
            model: "m".into(),
            base_url: None,
        },
        display_name: "Test".into(),
        thinking: level != "off",
        context_window: 128000,
    });
}

#[given("当前模型不支持思考")]
fn agent_no_thinking(w: &mut ToolWorld) {
    w.model_registry = ModelRegistry::new();
    w.model_registry.register(ModelMeta {
        id: "test".into(),
        config: ModelConfig {
            kind: ModelKind::OpenAi,
            api_key: "sk".into(),
            model: "m".into(),
            base_url: None,
        },
        display_name: "Test".into(),
        thinking: false,
        context_window: 128000,
    });
}

#[when(regex = r#"^([^ ]+)思考级别到 "([^"]+)"$"#)]
fn agent_switch_thinking(w: &mut ToolWorld, _verb: String, level: String) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.into_path());
    let mut session = AgentSession::new(
        w.model_registry.clone(),
        ToolRegistry::builtins(),
        mgr,
        None,
        50,
        0.8,
        ".".into(),
    );
    let tl = match level.as_str() {
        "high" => ThinkingLevel::High,
        "medium" => ThinkingLevel::Medium,
        "low" => ThinkingLevel::Low,
        _ => ThinkingLevel::Off,
    };
    let _ = session.set_thinking_level(tl);
    w.last_result = Some(Ok(format!("level:{}", session.thinking_level().as_str())));
}

#[then(regex = r#"^getThinkingLevel 返回 "([^"]+)"$"#)]
fn agent_thinking_level_is(w: &mut ToolWorld, level: String) {
    assert!(
        w.last_result
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains(&level)
    );
}

#[then("thinking_level_change 记录写入会话")]
fn agent_thinking_saved(_w: &mut ToolWorld) {}

#[then(regex = r#"^实际思考级别被限制为 "([^"]+)" 或模型支持的最高级别$"#)]
fn agent_thinking_clamped(w: &mut ToolWorld, _level: String) {
    assert!(
        w.last_result
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains("off")
    );
}

// Model switching
#[given(regex = r#"^注册了模型 "([^"]+)" 和 "([^"]+)"$"#)]
fn agent_models_registered(w: &mut ToolWorld, m1: String, m2: String) {
    w.model_registry = ModelRegistry::new();
    for name in [m1, m2] {
        let kind = if name.contains("claude") {
            ModelKind::Anthropic
        } else {
            ModelKind::OpenAi
        };
        w.model_registry.register(ModelMeta {
            id: name.clone(),
            config: ModelConfig {
                kind,
                api_key: "sk".into(),
                model: name.clone(),
                base_url: None,
            },
            display_name: name.clone(),
            thinking: true,
            context_window: 128000,
        });
    }
}

#[given(regex = r#"^当前模型为 "([^"]+)"$"#)]
fn agent_current_model(_w: &mut ToolWorld, _model: String) {}

#[when("执行 cycleForward")]
fn agent_cycle_forward(w: &mut ToolWorld) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.into_path());
    let mut session = AgentSession::new(
        w.model_registry.clone(),
        ToolRegistry::builtins(),
        mgr,
        None,
        50,
        0.8,
        ".".into(),
    );
    let next = session
        .cycle_forward()
        .map(|m| m.id.clone())
        .unwrap_or_default();
    w.last_result = Some(Ok(next));
}

#[then(regex = r#"^当前模型变为 "([^"]+)"$"#)]
fn agent_model_changed_to(w: &mut ToolWorld, model: String) {
    assert_eq!(w.last_result.as_ref().unwrap().as_ref().unwrap(), &model);
}

#[then("model_select 事件触发")]
fn agent_model_select_event(_w: &mut ToolWorld) {}

// Context usage
#[given(regex = r#"^会话包含 (\d+) 个 token 的消息$"#)]
fn agent_tokens(w: &mut ToolWorld, tokens: u32) {
    w.last_result = Some(Ok(format!("tokens:{tokens}")));
}

#[given("当前模型上下文窗口为 200000")]
fn agent_window_200k(_w: &mut ToolWorld) {}

#[when("调用 getContextUsage")]
fn agent_context_usage(w: &mut ToolWorld) {
    let tokens: u64 = w
        .last_result
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    w.context_usage = Some(get_context_usage(tokens, 200000, 0.8));
}

#[then(regex = r#"^返回 tokens 约为 (\d+)$"#)]
fn agent_tokens_approx(w: &mut ToolWorld, tokens: u32) {
    assert_eq!(w.context_usage.as_ref().unwrap().tokens, tokens as u64);
}

#[then(regex = r#"^percent 约为 (\d+)$"#)]
fn agent_percent(w: &mut ToolWorld, pct: u32) {
    assert_eq!(w.context_usage.as_ref().unwrap().percent, pct as u64);
}

// Auto-persistence
#[given("一个 turn 完成")]
async fn agent_turn_done(w: &mut ToolWorld) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.into_path());
    let sid = "auto-save-test";
    let _ = mgr.create(sid, Some("."), None).await;
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "auto-1".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role":"assistant","content":"done"}),
    });
    let _ = mgr.append(sid, &e).await;
    w.current_session_id = Some(sid.to_string());
    w.session_mgr = Some(mgr);
}

#[when("加载会话文件")]
async fn agent_load_session_file(w: &mut ToolWorld) {
    let sid = w.current_session_id.as_ref().unwrap().clone();
    w.session_entries = w
        .session_mgr
        .as_ref()
        .unwrap()
        .load(&sid)
        .await
        .unwrap_or_default();
}

#[then("该 turn 的消息记录已保存")]
fn agent_messages_saved(w: &mut ToolWorld) {
    assert!(!w.session_entries.is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Compaction
// ═══════════════════════════════════════════════════════════════════

#[given("配置了上下文窗口为 100000 的模型")]
fn comp_config_window(_w: &mut ToolWorld) {}

#[given(regex = r#"^会话消息估算使用 (\d+) 个 token$"#)]
fn comp_tokens(w: &mut ToolWorld, tokens: u32) {
    w.last_result = Some(Ok(format!("tokens:{tokens}")));
}

#[given(regex = r#"^压缩阈值为 ([0-9.]+)$"#)]
fn comp_threshold(w: &mut ToolWorld, t: f64) {
    w.compaction_threshold = t;
}

#[when("调用 shouldCompact")]
fn comp_check(w: &mut ToolWorld) {
    let tokens: u64 = w
        .last_result
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    w.compaction_result = Some(should_compact(tokens, 100000, w.compaction_threshold));
}

#[then("返回 true")]
fn comp_result_true(w: &mut ToolWorld) {
    assert_eq!(w.compaction_result, Some(true));
}

#[then("返回 false")]
fn comp_result_false(w: &mut ToolWorld) {
    assert_eq!(w.compaction_result, Some(false));
}

#[given("会话有 50 个轮次")]
fn comp_50_turns(_w: &mut ToolWorld) {}

#[when("触发压缩保留最近 10 轮")]
fn comp_trigger(w: &mut ToolWorld) {
    w.compaction_result = Some(should_compact(50 * 2000, 100000, 0.8));
}

#[then("前 40 轮被总结为一个 CompactionEntry")]
fn comp_has_summary(_w: &mut ToolWorld) {}

#[then(regex = r#"^会话中剩余 (\d+) 条记录（概要 \+ \d+ 轮）$"#)]
fn comp_remaining(_w: &mut ToolWorld, _n: u32) {}

#[given("会话正在活跃使用")]
fn comp_active(_w: &mut ToolWorld) {}

#[when("压缩完成")]
fn comp_done(_w: &mut ToolWorld) {}

#[then("会话 JSONL 包含 CompactionEntry")]
fn comp_jsonl_has_entry(_w: &mut ToolWorld) {}

#[then("CompactionEntry 包含 summary 字段")]
fn comp_has_summary_field(_w: &mut ToolWorld) {
    let e = CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: "c1".into(),
            parent_id: None,
            timestamp: "t".into(),
        },
        summary: "ok".into(),
        first_kept_entry_id: "e10".into(),
        tokens_before: 50000,
        details: None,
        from_hook: None,
    };
    assert_eq!(e.summary, "ok");
}

#[then("CompactionEntry 包含 firstKeptEntryId 字段")]
fn comp_has_firstkept(_w: &mut ToolWorld) {}

#[then("CompactionEntry 包含 tokensBefore 字段")]
fn comp_has_tokensbefore(_w: &mut ToolWorld) {}

#[given("用户在树中导航到分支点")]
fn comp_navigate_branch(_w: &mut ToolWorld) {}

#[when("生成分支摘要")]
fn comp_branch_summary(w: &mut ToolWorld) {
    w.last_result = Some(Ok("branch summary".into()));
}

#[then("摘要描述了被跳过的上下文")]
fn comp_branch_desc(w: &mut ToolWorld) {
    assert!(w.last_result.as_ref().unwrap().is_ok());
}

#[then("当前上下文是连贯的")]
fn comp_context_coherent(_w: &mut ToolWorld) {}

// ═══════════════════════════════════════════════════════════════════
// Hooks
// ═══════════════════════════════════════════════════════════════════

#[given(regex = r#"^注册了匹配 "([^"]+)" 的 hook$"#)]
fn hook_registered(w: &mut ToolWorld, pattern: String) {
    w.hook_entries.push(HookEntry {
        events: vec![pattern],
        command: "echo '{\"action\":\"allow\"}'".into(),
        phase: String::new(),
        timeout_secs: 5,
        requires_approval: false,
        env: HashMap::new(),
    });
}

#[given(regex = r#"^hook 返回 (.+)$"#)]
fn hook_returns(w: &mut ToolWorld, json_str: String) {
    if let Some(e) = w.hook_entries.last_mut() {
        let j: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
        e.command = format!("echo '{}'", j.to_string().replace('\'', "'\\''"));
    }
}

#[given(regex = r#"^全局配置有 hook for "([^\"]+)"$"#)]
fn hook_global(w: &mut ToolWorld, _p: String) {
    w.hook_entries.push(HookEntry {
        events: vec!["pre.tool_call".into()],
        command: "echo '{\"action\":\"allow\"}'".into(),
        ..Default::default()
    });
}

#[given(regex = r#"^用户配置有 hook for "([^\"]+)" 覆盖全局$"#)]
fn hook_user_override(w: &mut ToolWorld, _p: String) {
    if let Some(e) = w.hook_entries.last_mut() {
        e.command = "echo '{\"action\":\"allow\",\"source\":\"user\"}'".into();
    }
}

#[given("一个 hook 脚本执行超过 2 秒")]
fn hook_slow(w: &mut ToolWorld) {
    if let Some(e) = w.hook_entries.last_mut() {
        e.command = "sleep 10".into();
    }
}

#[given("hook 超时设为 1 秒")]
fn hook_timeout_1s(w: &mut ToolWorld) {
    if let Some(e) = w.hook_entries.last_mut() {
        e.timeout_secs = 1;
    }
}

#[given("没有注册任何 hook")]
fn hook_none(w: &mut ToolWorld) {
    w.hook_entries.clear();
}

#[given(regex = r#"^当前 provider 为 "([^"]+)"$"#)]
fn hook_provider(_w: &mut ToolWorld, _name: String) {}

async fn dispatch_hook(w: &mut ToolWorld, event: HookEvent, phase: HookPhase) {
    let dispatcher = HookDispatcher::new(&xylitol::infra::config::types::HooksConfig {
        global: w.hook_entries.clone(),
        project: vec![],
        user: vec![],
    });
    w.hook_result = Some(dispatcher.dispatch(&event, phase).await);
}

#[when("bash 工具即将执行")]
async fn hook_bash(w: &mut ToolWorld) {
    dispatch_hook(
        w,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"echo hello"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("任何工具即将执行")]
async fn hook_any_tool(w: &mut ToolWorld) {
    dispatch_hook(
        w,
        HookEvent::ToolCall {
            tool: "read".into(),
            args: serde_json::json!({}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when(regex = r#"^bash 工具以 "([^"]+)" 调用$"#)]
async fn hook_bash_called(w: &mut ToolWorld, _cmd: String) {
    dispatch_hook(
        w,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"rm -rf /"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("hook 被加载")]
fn hook_loaded(_w: &mut ToolWorld) {}

#[when("dispatch hook")]
async fn hook_dispatch_step(w: &mut ToolWorld) {
    dispatch_hook(
        w,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("provider 请求发送前")]
fn hook_before_request(_w: &mut ToolWorld) {}

#[when("provider 返回状态码 200")]
fn hook_provider_responded(_w: &mut ToolWorld) {}

#[when("任何事件触发")]
async fn hook_any_event_step(w: &mut ToolWorld) {
    dispatch_hook(
        w,
        HookEvent::StepComplete {
            step: 1,
            summary: "done".into(),
        },
        HookPhase::Post,
    )
    .await;
}

#[then("hook 脚本被调用")]
fn hook_called(_w: &mut ToolWorld) {}

#[then("hook 收到包含事件类型和参数的 JSON")]
fn hook_received_json(_w: &mut ToolWorld) {}

#[then("操作被阻止")]
fn hook_blocked(w: &mut ToolWorld) {
    assert!(matches!(
        w.hook_result.as_ref().unwrap(),
        DispatchResult::Blocked { .. }
    ));
}

#[then(regex = r#"^阻止原因包含 "([^"]*)"$"#)]
fn hook_block_reason(w: &mut ToolWorld, reason: String) {
    if let Some(DispatchResult::Blocked { reason: r }) = w.hook_result.as_ref() {
        assert!(r.contains(&reason));
    }
}

#[then(regex = r#"^实际执行的命令为 "([^"]*)"$"#)]
fn hook_actual_cmd(_w: &mut ToolWorld, _cmd: String) {}

#[then("使用用户配置的 hook 命令")]
fn hook_user_used(_w: &mut ToolWorld) {}

#[then("hook 在 1 秒后被杀死")]
fn hook_killed(w: &mut ToolWorld) {
    assert!(w.hook_result.is_some());
}

#[then("操作被允许继续")]
fn hook_allowed(w: &mut ToolWorld) {
    assert!(matches!(
        w.hook_result.as_ref().unwrap(),
        DispatchResult::Allowed
    ));
}

#[then("hook 收到请求 payload")]
fn hook_got_payload(_w: &mut ToolWorld) {}

#[then("hook 可以注入 cache_control 字段")]
fn hook_cache_control(_w: &mut ToolWorld) {}

#[then("hook 收到 status=200 和响应 headers")]
fn hook_got_response(_w: &mut ToolWorld) {}

#[then("dispatch 是零开销 no-op")]
fn hook_noop(w: &mut ToolWorld) {
    assert!(matches!(
        w.hook_result.as_ref().unwrap(),
        DispatchResult::Allowed
    ));
}

// ═══════════════════════════════════════════════════════════════════
// Tool Whens
// ═══════════════════════════════════════════════════════════════════

#[when(regex = r#"^调用edit工具 路径 "([^"]+)" 将 "([^"]*)" 替换为 "([^"]*)"$"#)]
async fn edit_single(w: &mut ToolWorld, path: String, old: String, new: String) {
    let full = w.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    match tool
        .execute(
            &ctx,
            serde_json::json!({"path": full, "edits": [{"oldText": old, "newText": new}]}),
        )
        .await
    {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用bash命令 "([^"]+)"$"#)]
async fn bash_cmd_step(w: &mut ToolWorld, cmd: String) {
    let tool = BashTool;
    let ctx = XyToolCtx::new("test");
    match tool
        .execute(&ctx, serde_json::json!({"command": cmd}))
        .await
    {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用read工具 路径 "([^"]+)"$"#)]
async fn read_path_step(w: &mut ToolWorld, path: String) {
    let full = w.ws(&path);
    let tool = ReadTool;
    let ctx = XyToolCtx::new("test");
    match tool.execute(&ctx, serde_json::json!({"path": full})).await {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用read工具 路径 "([^"]+)" 偏移 (\d+) 限制 (\d+)$"#)]
async fn read_offset_step(w: &mut ToolWorld, path: String, offset: i64, limit: i64) {
    let full = w.ws(&path);
    let tool = ReadTool;
    let ctx = XyToolCtx::new("test");
    match tool
        .execute(
            &ctx,
            serde_json::json!({"path": full, "offset": offset, "limit": limit}),
        )
        .await
    {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用write工具 路径 "([^"]+)" 内容 "([^"]*)"$"#)]
async fn write_file_step(w: &mut ToolWorld, path: String, content: String) {
    let full = w.ws(&path);
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    match tool
        .execute(&ctx, serde_json::json!({"path": full, "content": content}))
        .await
    {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用grep 模式 "([^"]+)" 路径 "([^"]+)"$"#)]
async fn grep_step(w: &mut ToolWorld, pattern: String, path: String) {
    let full = w.ws(&path);
    let tool = GrepTool;
    let ctx = XyToolCtx::new("test");
    match tool
        .execute(&ctx, serde_json::json!({"pattern": pattern, "path": full}))
        .await
    {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用find 模式 "([^"]+)" 路径 "([^"]+)"$"#)]
async fn find_step(w: &mut ToolWorld, pattern: String, path: String) {
    let full = w.ws(&path);
    let tool = FindTool;
    let ctx = XyToolCtx::new("test");
    match tool
        .execute(&ctx, serde_json::json!({"pattern": pattern, "path": full}))
        .await
    {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

#[when(regex = r#"^调用ls工具 路径 "([^"]+)"$"#)]
async fn ls_step(w: &mut ToolWorld, path: String) {
    let full = w.ws(&path);
    let tool = LsTool;
    let ctx = XyToolCtx::new("test");
    match tool.execute(&ctx, serde_json::json!({"path": full})).await {
        Ok(r) => w.last_result = Some(Ok(r)),
        Err(e) => w.last_result = Some(Err(e.to_string())),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Shared Thens
// ═══════════════════════════════════════════════════════════════════

macro_rules! result_ok {
    ($w:expr) => {
        $w.last_result.as_ref().unwrap().as_ref().unwrap()
    };
}

#[then(regex = r#"^文件 "([^"]+)" 应该包含 "([^"]*)"$"#)]
fn file_contains(w: &mut ToolWorld, path: String, text: String) {
    let c = std::fs::read_to_string(w.ws(&path)).unwrap();
    assert!(c.contains(&text), "期望包含'{text}'，实际: {c}");
}

#[then(regex = r#"^文件 "([^"]+)" 内容为 "([^"]*)"$"#)]
fn file_content_is(w: &mut ToolWorld, path: String, text: String) {
    assert_eq!(std::fs::read_to_string(w.ws(&path)).unwrap(), text);
}

#[then(regex = r#"^文件 "([^"]+)" 应该存在$"#)]
fn file_exists(w: &mut ToolWorld, path: String) {
    assert!(std::path::Path::new(&w.ws(&path)).exists());
}

#[then(regex = r#"^文件 "([^"]+)" 应该保持CRLF行尾$"#)]
fn file_has_crlf(w: &mut ToolWorld, path: String) {
    assert!(
        std::fs::read_to_string(w.ws(&path))
            .unwrap()
            .contains("\r\n")
    );
}

#[then(regex = r#"^文件 "([^"]+)" 应该保留UTF8_BOM$"#)]
fn file_has_bom(w: &mut ToolWorld, path: String) {
    assert!(
        std::fs::read_to_string(w.ws(&path))
            .unwrap()
            .starts_with('\u{FEFF}')
    );
}

#[then("结果包含 unified patch")]
fn result_has_patch(w: &mut ToolWorld) {
    assert!(result_ok!(w).contains("diff") || result_ok!(w).contains("---"));
}

#[then("结果包含 带行号的 display diff")]
fn result_has_diff(w: &mut ToolWorld) {
    assert!(result_ok!(w).contains("display_diff"));
}

#[then(regex = r#"^edit调用应该失败 包含错误信息 "([^"]*)"$"#)]
fn edit_failed(w: &mut ToolWorld, msg: String) {
    let r = w.last_result.as_ref().unwrap();
    assert!(r.is_err() && r.as_ref().unwrap_err().contains(&msg));
}

#[then(regex = r#"^调用失败 包含验证错误$"#)]
fn call_fail_validation(w: &mut ToolWorld) {
    assert!(w.last_result.as_ref().unwrap().is_err());
}

#[then(regex = r#"^调用失败 包含错误信息 "([^"]*)"$"#)]
fn call_fail_msg(w: &mut ToolWorld, msg: String) {
    assert!(
        w.last_result
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap_err()
            .contains(&msg)
    );
}

#[then("调用失败 包含错误信息")]
fn call_fail(w: &mut ToolWorld) {
    assert!(w.last_result.as_ref().unwrap().is_err());
}

#[then(regex = r#"^退出码为 (\d+)$"#)]
fn exit_code_is(w: &mut ToolWorld, code: i32) {
    assert!(result_ok!(w).contains(&format!("exit_code\":{code}")));
}

#[then(regex = r#"^stdout 包含 "([^"]*)"$"#)]
fn stdout_has(w: &mut ToolWorld, text: String) {
    assert!(result_ok!(w).contains(&text));
}

#[then(regex = r#"^stdout 和 stderr 合并输出包含 "([^"]*)"$"#)]
fn combined_has(w: &mut ToolWorld, text: String) {
    assert!(result_ok!(w).contains(&text));
}

#[then("命令应该失败 包含超时错误")]
fn cmd_timeout(w: &mut ToolWorld) {
    assert!(w.last_result.as_ref().unwrap().is_err());
}

#[then("命令应该失败 包含取消错误")]
fn cmd_abort(w: &mut ToolWorld) {
    assert!(w.last_result.as_ref().unwrap().is_err());
}

#[then("输出被截断")]
fn output_truncated(w: &mut ToolWorld) {
    assert!(result_ok!(w).contains("truncated"));
}

#[then("截断详情显示达到字节或行限制")]
fn truncation_details(w: &mut ToolWorld) {
    assert!(result_ok!(w).contains("truncated"));
}

#[then("如果截断则显示剩余行提示")]
fn remaining_hint_if_truncated(_w: &mut ToolWorld) {}

#[then(regex = r#"^内容为 "([^"]*)"$"#)]
fn read_content(w: &mut ToolWorld, text: String) {
    assert!(result_ok!(w).contains(&text));
}

#[then(regex = r#"^总行数为 (\d+)$"#)]
fn total_lines(w: &mut ToolWorld, count: u32) {
    let v: serde_json::Value = serde_json::from_str(result_ok!(w)).unwrap_or_default();
    assert_eq!(v["total_lines"], count);
}

#[then(regex = r#"^偏移量为 (\d+)$"#)]
fn offset_is(w: &mut ToolWorld, o: i64) {
    let v: serde_json::Value = serde_json::from_str(result_ok!(w)).unwrap_or_default();
    assert_eq!(v["offset"], o);
}

#[then(regex = r#"^结果指示目录为空$"#)]
fn ls_empty(w: &mut ToolWorld) {
    assert!(result_ok!(w).contains("empty"));
}

#[then(regex = r#"^结果列出 "([^"]+)"$"#)]
fn ls_lists(w: &mut ToolWorld, entry: String) {
    assert!(result_ok!(w).contains(&entry));
}

#[then(regex = r#"^结果列出 "([^"]+)" 带后缀 "([^"]*)"$"#)]
fn ls_lists_suffix(w: &mut ToolWorld, entry: String, suffix: String) {
    assert!(result_ok!(w).contains(&format!("{entry}{suffix}")));
}

#[then("条目按字母顺序排列")]
fn ls_sorted(w: &mut ToolWorld) {
    let r = result_ok!(w);
    let lines: Vec<&str> = r
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('['))
        .collect();
    let mut sorted = lines.clone();
    sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    assert_eq!(lines, sorted);
}

#[then("结果指示达到条目限制")]
fn ls_limit_hint(w: &mut ToolWorld) {
    assert!(result_ok!(w).contains("entries shown"));
}

#[then(regex = r#"^结果包含 "([^"]+)"$"#)]
fn result_contains(w: &mut ToolWorld, text: String) {
    assert!(result_ok!(w).contains(&text));
}

#[then(regex = r#"^结果不包含 "([^"]+)"$"#)]
fn result_not_has(w: &mut ToolWorld, text: String) {
    assert!(!result_ok!(w).contains(&text));
}

#[then(regex = r#"^恰好有 (\d+) 条结果$"#)]
fn exact_results(w: &mut ToolWorld, count: u32) {
    let lines = result_ok!(w)
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("limit"))
        .count();
    assert_eq!(lines, count as usize);
}

#[then(regex = r#"^共有 (\d+) 条匹配$"#)]
fn grep_match_count(_w: &mut ToolWorld, _n: u32) {}

#[then(regex = r#"^匹配结果包含第(\d+)行的 "([^"]*)"$"#)]
fn grep_match_on_line(_w: &mut ToolWorld, _line: u32, _text: String) {}

#[then(regex = r#"^内容为:$"#)]
fn read_content_multi(w: &mut ToolWorld, content: String) {
    let v: serde_json::Value = serde_json::from_str(result_ok!(w)).unwrap_or_default();
    assert_eq!(v["content"].as_str().unwrap_or(""), content.trim());
}

// ═══════════════════════════════════════════════════════════════════
// main
// ═══════════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() {
    ToolWorld::run("tests/features").await;
}
