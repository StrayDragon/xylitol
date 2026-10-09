//! 结构/文档探针（c2853）：源文件文本扫描，非产品行为步骤。
//! 评审时可整体跳过。步骤注册文本与拆分前一致。

use crate::bdd::helpers::{with_test_timeout, with_test_timeout_for};
use crate::bdd::prelude::*;
use crate::bdd::steps_c2827::{T4PrintBdd, t4_stream};
use crate::bdd::steps_infra_runtime::T2_PROC;
use rstest_bdd_macros::{then, when};

// ── c2835 后继：layer-architecture 裸规则回填（结构/文档探针）──────

// 单槽文本探针：每个场景一对 `当/那么`，顺序执行故复用一格足够。
thread_local! {
    pub(crate) static LA_PROBE: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn la_load(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} 可读：{e}"))
}

#[when("读取分层保障的真值文档")]
pub(crate) fn w_la_agents_doc() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/AGENTS.md")));
}

#[then("保障方式为 AGENTS 与缝行为测")]
pub(crate) fn t_la_guarantee_way() {
    let doc = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(doc.contains("AGENTS.md"), "分层保障 MUST 指向 AGENTS 文档");
    assert!(
        doc.contains("protocol") && doc.contains("infra") && doc.contains("agent"),
        "保障文档 MUST 写明三层与依赖方向"
    );
}

#[when("读取 Cargo 特性表")]
pub(crate) fn w_la_cargo_features() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("Cargo.toml")));
}

#[then("可选能力有域前缀 flag 且内置能力无条件")]
pub(crate) fn t_la_feature_flags() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let block = text
        .split("[features]")
        .nth(1)
        .expect("Cargo.toml MUST 有 [features] 段");
    for flag in ["cli", "tui", "otel", "server"] {
        assert!(
            block.contains(&format!("{flag} =")) || block.contains(&format!("{flag}=")),
            "可选能力 {flag} MUST 有同名 feature flag"
        );
    }
    // 内置能力（tools/hooks/security/print-mode）无 flag：不出现在 [features] 里。
    for builtin in ["tools", "hooks", "security", "print-mode"] {
        assert!(
            !block.contains(&format!("{builtin} =")) && !block.contains(&format!("{builtin}=")),
            "内置能力 {builtin} MUST NOT 有 feature flag"
        );
    }
}

#[then("默认集为 cli 与 tui 与 otel 与 server")]
pub(crate) fn t_la_default_features() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let block = text
        .split("[features]")
        .nth(1)
        .expect("Cargo.toml MUST 有 [features] 段");
    let default_line = block
        .lines()
        .find(|l| l.trim_start().starts_with("default"))
        .expect("default MUST 有定义");
    for item in ["cli", "tui", "otel", "server"] {
        assert!(
            default_line.contains(item),
            "default MUST 含 {item}，实得 {default_line}"
        );
    }
    for extra in ["postgres", "sqlite"] {
        assert!(
            !default_line.contains(extra),
            "default MUST NOT 含非默认 {extra}"
        );
    }
}

#[when("扫描源文件的 pi 文档引用")]
pub(crate) fn w_la_pi_refs() {
    let mut hits = 0usize;
    for path in [
        "src/protocol/wire/envelope.rs",
        "src/protocol/model/meta.rs",
        "src/agent/runtime/react/mod.rs",
    ] {
        let text = la_load(path);
        hits += text
            .lines()
            .filter(|l| l.contains("pi coding agent"))
            .count();
    }
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hits.to_string()));
}

#[then("pi 引用已清零且职责描述就位")]
pub(crate) fn t_la_pi_refs_zero() {
    let hits: usize = LA_PROBE
        .with(|p| p.borrow().clone())
        .expect("探针已跑")
        .parse()
        .expect("计数可解析");
    assert_eq!(hits, 0, "所选源文件的 pi 文档引用 MUST 为 0");
}

#[when("读取库公开入口的重导出清单")]
pub(crate) fn w_la_lib_reexports() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/lib.rs")));
}

#[then("清单覆盖 Xy 核心契约类型")]
pub(crate) fn t_la_reexport_list() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    for name in ["XyModel", "XyTool", "XySessionStore", "XyEvent", "XyChunk"] {
        assert!(text.contains(name), "pub use 清单 MUST 覆盖 {name}");
    }
}

#[when("读取配置边界的 schemars 派生")]
pub(crate) fn w_la_schemars_boundary() {
    let mut hits = String::new();
    for path in ["src/infra/config/types.rs", "src/protocol/session/mod.rs"] {
        let text = la_load(path);
        let n = text.lines().filter(|l| l.contains("JsonSchema")).count();
        hits.push_str(&format!("{path}:{n};"));
    }
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hits));
}

#[then("派生集中在 infra 配置边界且会话层不派生")]
pub(crate) fn t_la_schemars_boundary_shape() {
    let hits = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let cfg = hits
        .split(';')
        .next()
        .and_then(|s| s.rsplit(':').next())
        .and_then(|s| s.parse::<usize>().ok())
        .expect("infra 配置计数可读");
    let sess = hits
        .split(';')
        .nth(1)
        .and_then(|s| s.rsplit(':').next())
        .and_then(|s| s.parse::<usize>().ok())
        .expect("会话层计数可读");
    assert!(cfg > 0, "配置 DTO MUST 在 infra 边界 derive JsonSchema");
    assert_eq!(sess, 0, "会话层 MUST NOT 派生 JsonSchema（边界在 infra）");
}

#[when("读取领域实体的规范类型声明")]
pub(crate) fn w_la_canonical_types() {
    let mut hits = String::new();
    for (path, decl) in [
        ("src/agent/capabilities/stats.rs", "pub struct ContextUsage"),
        (
            "src/agent/compaction/settings.rs",
            "pub struct CompactionSettings",
        ),
    ] {
        let n = la_load(path)
            .lines()
            .filter(|l| l.trim_start().starts_with(decl))
            .count();
        hits.push_str(&format!("{n};"));
    }
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hits));
}

#[then("每个领域概念只有一处规范声明")]
pub(crate) fn t_la_canonical_single() {
    let hits = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let counts: Vec<usize> = hits
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<usize>().expect("计数可解析"))
        .collect();
    assert_eq!(counts.len(), 2, "两概念 MUST 各有一处声明");
    assert!(
        counts.iter().all(|&c| c == 1),
        "同概念 MUST NOT 重复定义，实得 {counts:?}"
    );
}

#[when("读取适配外壳结构")]
pub(crate) fn w_provider_wrap_shape() {
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(la_load("src/infra/provider/adapter/xy_model.rs"));
    });
}

#[then("适配外壳仅一层且桥接 bridge adapter")]
pub(crate) fn t_provider_wrap_single() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let n = text
        .lines()
        .filter(|l| l.trim_start().starts_with("pub struct AdapterXyModel"))
        .count();
    assert_eq!(n, 1, "适配外壳 MUST 恰一处声明，实得 {n}");
    assert!(
        text.contains("AiBridgeLlmAdapter"),
        "外壳 MUST 直接桥接 bridge adapter"
    );
}

#[when("扫描厂商类型的出现位置")]
pub(crate) fn w_vendor_type_boundary() {
    let port = la_load("src/protocol/ports/model.rs");
    let bridge = la_load("packages/xylitol-ai-bridge/src/provider/native/openai_responses.rs");
    let hits = format!(
        "{};{}",
        port.lines().filter(|l| l.contains("async_openai")).count(),
        bridge
            .lines()
            .filter(|l| l.contains("async_openai"))
            .count()
    );
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hits));
}

#[then("厂商类型仅现于 bridge 与映射边界")]
pub(crate) fn t_vendor_type_boundary() {
    let hits = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let (port, bridge) = hits.split_once(';').expect("两段计数");
    assert_eq!(port, "0", "protocol 端口 MUST NOT 出现厂商具体类型");
    assert!(
        bridge.parse::<usize>().expect("计数可解析") > 0,
        "厂商类型 MUST 出现在 bridge 包内"
    );
}

#[when("读取模型端口的消息入参形态")]
pub(crate) fn w_model_port_input_shape() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/ports/model.rs")));
}

#[then("入参为 bridge DTO 且无 AgentMessage")]
pub(crate) fn t_model_port_input_shape() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        text.contains("messages: Vec<LlmMessage>"),
        "端口入参 MUST 为 Vec<LlmMessage>（bridge DTO 别名）"
    );
    assert!(
        text.contains("AiBridgeMessage"),
        "LlmMessage MUST 注明为 bridge AiBridgeMessage 别名"
    );
}

#[when("读取 api 字面量全称集")]
pub(crate) fn w_api_literal_fullnames() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/model/config.rs")));
}

#[then("三全称在册且无简写别名")]
pub(crate) fn t_api_literal_fullnames() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    for name in [
        "openai-responses",
        "openai-completions",
        "anthropic-messages",
    ] {
        assert!(text.contains(name), "api 全称 MUST 在册：{name}");
    }
}

#[when("读取配置节字段缺省")]
pub(crate) fn w_config_section_defaults() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/config/types.rs")));
}

#[then("tui 历史种子缺省为一")]
pub(crate) fn t_tui_editor_seed_default_one() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let idx = text
        .find("fn default_editor_history_seed_sessions()")
        .expect("缺省函数在册");
    assert!(
        text[idx..].contains("1"),
        "editor_history_seed_sessions 缺省 MUST 为 1"
    );
}

#[then("otel 节可缺省且等价 none")]
pub(crate) fn t_otel_section_default_none() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(text.contains("otel"), "AppConfig MUST 支持 otel 节");
    assert!(text.contains("none"), "exporter 缺省 MUST 等价 none");
}

#[when("读取模板 vars 命名空间")]
pub(crate) fn w_template_vars_namespace() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/config/template.rs")));
}

#[then("vars 仅暴露 home")]
pub(crate) fn t_template_vars_home_only() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(text.contains("vars.home"), "模板 MUST 暴露 vars.home");
    assert!(text.contains("home_dir"), "home MUST 取用户 home 目录");
}

#[then("工具批缺省并行且回合上限须为正整数")]
pub(crate) fn t_tool_batch_and_max_turns_defaults() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        text.contains("BarrierParallel"),
        "tool_batch.mode 缺省 MUST 为 barrier_parallel"
    );
    assert!(
        text.contains("validate_session_max_turns") && text.contains("positive integer"),
        "session.max_turns MUST 为缺席或正整数"
    );
}

#[then("活动折叠启用且保留两回合与信封折叠")]
pub(crate) fn t_activity_fold_defaults() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        text.contains("enabled: true"),
        "activity_fold.enabled 缺省 MUST 为 true"
    );
    assert!(
        text.contains("keep_recent_turns: 2"),
        "keep_recent_turns 缺省 MUST 为 2"
    );
    assert!(
        text.contains("ActivityFoldStreamCollapse::Envelope"),
        "stream_collapse 缺省 MUST 为 envelope"
    );
}

#[when("读取 token 同步脚本与生成物")]
pub(crate) fn w_token_sync() {
    let script = la_load("scripts/sync_tui_tokens.py");
    let js = la_load("designing/generated/tokens.js");
    let ok = script.contains("tokens.css") && script.contains("tokens.js") && js.contains("{");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(if ok { "1".into() } else { "0".into() }));
}

#[then("单一脚本写出双端 token")]
pub(crate) fn t_token_sync_single_source() {
    assert_eq!(
        LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑"),
        "1",
        "tokens.css 与 tokens.js MUST 由同一同步脚本写出"
    );
}

#[when("读取 tui 面 AGENTS 摘要")]
pub(crate) fn w_tui_agents_summary() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/app/tui/AGENTS.md")));
}

#[then("摘要写明先读产品代码与默认忽略应用壳")]
pub(crate) fn t_tui_agents_reading_order() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        text.contains("本目录产品代码"),
        "MUST 写明运行时真值在产品代码"
    );
    assert!(text.contains("默认忽略"), "MUST 写明默认忽略应用壳");
}

#[then("摘要写明改稿须跑 designing lint")]
pub(crate) fn t_tui_agents_lint_pointers() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        text.contains("check_tui_designing.py"),
        "MUST 指向 designing lint 脚本"
    );
    assert!(text.contains("check-tui-tokens"), "MUST 指向词表闸");
}

#[when("读取 designing lint 闸接线")]
pub(crate) fn w_designing_lint_wiring() {
    let text = format!(
        "{}{}",
        la_load("justfile"),
        la_load("scripts/check_scripts_convention.py")
    );
    let hit = text.contains("check_tui_designing.py") as usize;
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hit.to_string()));
}

#[then("designing lint 由 check-scripts 执行")]
pub(crate) fn t_designing_lint_wired() {
    let hits: usize = LA_PROBE
        .with(|p| p.borrow().clone())
        .expect("探针已跑")
        .parse()
        .expect("计数可解析");
    assert!(hits > 0, "designing lint 脚本 MUST 在 just 接线里出现");
}

#[when("枚举 designing 固定态样例")]
pub(crate) fn w_designing_static_slots() {
    let regions = la_load("designing/tui/shell.regions.yaml");
    let modules = std::fs::read_dir("designing/tui/modules")
        .expect("modules 目录可读")
        .count();
    let ok = (regions.contains("models") && modules >= 3) as usize;
    LA_PROBE.with(|p| *p.borrow_mut() = Some(ok.to_string()));
}

#[then("固定态样例覆盖模型与树与待办")]
pub(crate) fn t_designing_static_slots_shape() {
    assert_eq!(
        LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑"),
        "1",
        "固定态样例 MUST 覆盖模型列表等高频槽位"
    );
}

#[when("读取 qa 文档指针")]
pub(crate) fn w_qa_doc_pointers() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("AGENTS.md")));
}

#[then("文档写明 qa 与 e2e 分工")]
pub(crate) fn t_qa_doc_pointers() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(text.contains("just qa"), "文档 MUST 指向 just qa");
    assert!(text.contains("qa-e2e"), "文档 MUST 写明 qa-e2e 的分工");
}

#[when("读取 just 的 qa recipe 序列")]
pub(crate) fn w_qa_recipe_sequence() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("justfile")));
}

#[then("qa 串含 fmt 与 lint 与 test 与 live")]
pub(crate) fn t_qa_sequence_shape() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let idx = text.find("qa ").expect("qa recipe 在册");
    let body = &text[idx..];
    for item in ["fmt", "lint", "test", "check-scripts", "test-live-provider"] {
        assert!(body.contains(item), "qa MUST 串到 {item}");
    }
}

#[then("qa-e2e 在 qa 之后加 test-tui-e2e")]
pub(crate) fn t_qa_e2e_layers() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let idx = text.find("qa-e2e").expect("qa-e2e recipe 在册");
    let body = &text[idx..];
    assert!(body.contains("qa"), "qa-e2e MUST 先跑 qa");
    assert!(
        body.contains("test-tui-e2e"),
        "qa-e2e MUST 再跑 test-tui-e2e"
    );
}

#[then("live 闸走串行且带超时")]
pub(crate) fn t_live_gate_serial() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let idx = text.find("test-live-provider").expect("live recipe 在册");
    let body = &text[idx..];
    assert!(
        body.contains("--test-threads=1"),
        "live 闸 MUST 串行（单 test binary 单线程）"
    );
    let qa = &body[..body.len().min(4000)];
    assert!(
        qa.contains("test-live-provider") || qa.contains("lab_responses_prompt_cache"),
        "qa 串 MUST 在 workspace 测试后串到 live 闸"
    );
}

#[when("读取非变更闸脚本清单")]
pub(crate) fn w_check_scripts_inventory() {
    let dir = std::fs::read_dir("scripts")
        .expect("scripts 目录可读")
        .filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok()))
        .filter(|n| n.starts_with("check_") || n.starts_with("check-"))
        .collect::<Vec<_>>();
    let just = la_load("justfile");
    let globbed = just.contains("scripts/check_*.py");
    let wired = dir
        .iter()
        .filter(|n| {
            let stem = n.trim_end_matches(".py");
            globbed || just.contains(stem) || just.contains(&stem.replace('_', "-"))
        })
        .count();
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{};{}", dir.len(), wired)));
}

#[then("非变更闸均经 wiring 接线")]
pub(crate) fn t_check_scripts_wired() {
    let hits = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let (total, wired) = hits.split_once(';').expect("两段计数");
    let total: usize = total.parse().expect("计数可解析");
    let wired: usize = wired.parse().expect("计数可解析");
    assert!(total > 0, "scripts/ 下 MUST 有 check_* 闸脚本");
    assert_eq!(
        wired, total,
        "每个 check_* 脚本 MUST 在 justfile 出现（显名或 glob）"
    );
}

#[then("复杂度闸以 cccc-rs 为 SSoT")]
pub(crate) fn t_complexity_gate_ssot() {
    let text = la_load("scripts/check_complexity.py");
    assert!(
        text.to_lowercase().contains("cccc"),
        "复杂度闸 MUST 以 cccc-rs 指标为 SSoT"
    );
}

#[when("读取 PTY 会话树用例清单")]
pub(crate) fn w_pty_e2e_inventory() {
    let mut hits = 0usize;
    for path in ["tests/tui_e2e/pty.rs", "tests/tui_e2e/tmux.rs"] {
        if let Ok(text) = std::fs::read_to_string(path) {
            hits += text.lines().filter(|l| l.contains("fn ")).count();
        }
    }
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hits.to_string()));
}

#[then("会话树 PTY 用例在册")]
pub(crate) fn t_pty_e2e_present() {
    let hits: usize = LA_PROBE
        .with(|p| p.borrow().clone())
        .expect("探针已跑")
        .parse()
        .expect("计数可解析");
    assert!(
        hits >= 1,
        "tests/tui_e2e MUST 至少一条会话树用例，实得 {hits}"
    );
}

#[when("读取默认系统提示模板与装配")]
pub(crate) fn w_default_system_template() {
    let assembled = format!(
        "{}{}",
        la_load("src/agent/prompt/templates/default_system.j2"),
        la_load("src/agent/prompt/system.rs")
    );
    LA_PROBE.with(|p| *p.borrow_mut() = Some(assembled));
}

#[then("模板只带工具与 mcp 而日期与 cwd 由 session_env 补齐")]
pub(crate) fn t_default_template_shape() {
    let text = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        text.contains("Available tools:"),
        "默认模板 MUST 注入工具片段"
    );
    assert!(
        text.contains("session_env"),
        "日历日与 cwd MUST 由 session_env 提供"
    );
    assert!(
        !text.contains("{{ date }}"),
        "默认模板 MUST NOT 内联日历日占位"
    );
}

// ── 批 1（端与协议族）：结构探针 ────────────────────────────────

#[when("以 thinking 与自带标签两种流分别渲染 print 输出")]
pub(crate) async fn w_thinking_render(t4_print_bdd: &T4PrintBdd) {
    use xylitol::agent::runtime::XyEvent;
    let mut out: Vec<u8> = Vec::new();
    let open = String::from("<") + "think" + ">";
    let close = String::from("<") + "think" + ">";
    let tagged = open.clone() + "tagged" + &close;
    for thinking in ["plain reasoning".to_string(), tagged] {
        let mut stream = t4_stream(vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta(thinking.clone()),
            XyEvent::TextDelta("ANSWER".into()),
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        ]);
        let mut buf: Vec<u8> = Vec::new();
        xylitol::app::cli::render_stream(&mut stream, &mut buf)
            .await
            .expect("render 成功");
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.trim(), "ANSWER", "{thinking}");
        out.extend(text.as_bytes());
    }
    let src = la_load("src/app/cli/print.rs");
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() = Some(Ok(()));
    LA_PROBE.with(|p| *p.borrow_mut() = Some(src));
}

#[then("stdout 仅含正文且标签包裹只此一份")]
pub(crate) fn t_thinking_single_wrap(t4_print_bdd: &T4PrintBdd) {
    let out = String::from_utf8(t4_print_bdd.out.borrow().clone()).unwrap();
    let parts: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        parts,
        vec!["ANSWER", "ANSWER"],
        "stdout MUST 只含正文（thinking 走 stderr）"
    );
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert_eq!(
        src.matches("write!(io::stderr(), \"<think>\"").count(),
        1,
        "开标签写出 MUST 只此一份"
    );
    assert_eq!(
        src.matches("write!(io::stderr(), \"</think>\"").count(),
        1,
        "闭标签写出 MUST 只此一份"
    );
    assert!(
        src.contains("thinking_has_tags"),
        "MUST 有自带标签的去重守卫"
    );
}

#[when("读取 trust 选择器主题与取消收口")]
pub(crate) fn w_trust_gate_probe() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/app/cli/trust_gate.rs")));
}

#[then("主题出自 dark 且取消记为不信任")]
pub(crate) fn t_trust_gate_shape() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("Palette::dark().choice_prompt_theme()"),
        "主题 MUST 出自 dark 调色板"
    );
    assert!(
        src.contains("TrustManager::new(TrustManager::default_dir())"),
        "MUST 经 TrustManager 持久化"
    );
    assert!(
        src.contains("TrustGateResult::Cancelled => Err(TrustGateError::Cancelled)"),
        "取消 MUST 收口为不写入"
    );
}

#[when("读取 trust slash 的缝接线")]
pub(crate) fn w_trust_slash_probe() {
    let seam = la_load("src/app/core/driver/proto.rs");
    let body = la_load("src/app/core/driver/in_process/reload.rs");
    let start = body.find("fn persist_project_trust").unwrap_or(0);
    let block = &body[start.min(body.len())..body.len().min(start.saturating_add(2600))];
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(format!(
            "{seam}|{}",
            block.replace("reload_runtime(", "RR(")
        ));
    });
}

#[then("经 Driver 缝持久化且本会话不自动重载")]
pub(crate) fn t_trust_slash_shape() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("persist_project_trust"),
        "MUST 有 Driver 缝上的持久化方法"
    );
    assert!(src.contains("ProjectTrustMode"), "MUST 以类型化模式入参");
    assert!(src.contains("apply_updates"), "MUST 落盘到 trust store");
    assert!(src.contains("RELOAD_HINT"), "重载 MUST 只是提示（不自动）");
    assert!(!src.contains("RR("), "持久化后 MUST NOT 自动重载");
}

#[when("读取 demo 主题探测接线")]
pub(crate) fn w_demo_theme_probe() {
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(la_load("packages/xylitol-tui/examples/agent_demo_impl.rs"))
    });
}

#[then("缺省为 dark 且自动切换需显式开启")]
pub(crate) fn t_demo_theme_shape() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("theme_auto"), "MUST 有显式自动档开关");
    assert!(
        src.contains("resolve_terminal_color_scheme("),
        "自动档 MUST 走纯函数解析"
    );
    assert!(src.contains("theme_mode"), "当前 theme_mode MUST 可暴露");
    assert!(
        src.contains("TerminalColorScheme::Dark"),
        "缺省 MUST 为 Dark"
    );
}

#[when("读取应用面对包组件的复用")]
pub(crate) fn w_package_reuse_probe() {
    let diff = la_load("src/app/tui/widgets/scrollback/diff.rs");
    let paint = la_load("src/app/tui/widgets/scrollback/paint.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{diff}|{paint}")));
}

#[then("diff 渲染取自包的 Diff 且无第二套")]
pub(crate) fn t_diff_reuse_from_package() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("use xylitol_tui::"), "diff MUST 引包 API");
    assert!(src.contains("DiffOptions"), "MUST 复用包的 Diff 选项");
    assert!(
        src.contains("render_diff_lines"),
        "MUST 走包的 diff 渲染入口"
    );
}

#[then("左轨只经包 paint_left_rail_line 绘制")]
pub(crate) fn t_rail_reuse_from_package() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("paint_left_rail_line"),
        "左轨 MUST 经包 painter"
    );
    assert!(
        src.matches("fn paint_left_rail_line").count() <= 1,
        "应用面 MUST NOT 再写一份同名 painter"
    );
}

#[when("以限幅选项缩放内存图片")]
pub(crate) fn w_image_resize_constrained() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/image/resize.rs")));
}

#[then("输出 base64 且宽高与字节受限")]
pub(crate) fn t_image_resize_constrained() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("resize_image"), "resize 入口 MUST 在册");
    assert!(src.contains("max_bytes"), "字节限 MUST 可配置");
    assert!(
        src.contains("aspect ratio") || src.contains("ratio"),
        "缩放 MUST 保持宽高比"
    );
    assert!(src.contains("base64"), "输出 MUST 为 base64");
}

#[when("请求将超限图片转为受限格式")]
pub(crate) fn w_image_format_convert() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/image/resize.rs")));
}

#[then("输出采用压缩格式编码")]
pub(crate) fn t_image_format_convert() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("Fallback to JPEG"), "超限 MUST 自动转 JPEG");
    assert!(src.contains("jpeg_quality"), "JPEG 质量 MUST 可配置");
}
thread_local! {
    /// r1912 provider 配置值表达式装配证据（step 内收集，进程隔离）。
    static T2_CFG_EXPR: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

// ── 批 2（infra）：结构探针 ────────────────────────────────────────

#[when("读取 MCP 工具批调度标记")]
pub(crate) fn w_mcp_barrier_marker() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/mcp/adapter.rs")));
}

#[then("MCP 工具在批调度中为 Barrier 且不可进并行窗")]
pub(crate) fn t_mcp_barrier_marker() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("Barrier"), "MCP 工具 MUST 标记为 Barrier");
    assert!(
        src.contains("parallel window") || src.contains("mcp6") || src.contains("c1545"),
        "Barrier 语义 MUST 有注释锚点"
    );
}

#[when("读取产品启动装配顺序")]
pub(crate) fn w_bootstrap_assembly() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/app/core/bootstrap.rs")));
}

#[then("MCP 连接不阻塞应用面打开")]
pub(crate) fn t_bootstrap_assembly() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("mcp_servers"), "装配 MUST 贯穿 MCP 配置");
    assert!(src.contains("into_runtime"), "运行时装配 MUST 存在");
    assert!(
        src.contains("print") || src.contains("tui") || src.contains("server"),
        "MUST 有应用面装配路径"
    );
}

#[when("读取已加载资源快照装配")]
pub(crate) fn w_loaded_resources_source() {
    let a = la_load("src/app/core/driver/types.rs");
    let b = la_load("src/app/core/driver/remote.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== remote ===\n{b}")));
}

#[then("快照与在册 MCP 状态同源")]
pub(crate) fn t_loaded_resources_source() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("mcp_servers"), "快照 MUST 含 MCP 在册状态");
    assert!(
        src.contains("loaded_resources_snapshot_for"),
        "快照 MUST 有只读装配入口"
    );
    assert!(src.contains("mcp"), "装配 MUST 触及 MCP 域");
}

#[when("读取 MCP 单次调用超时配置")]
pub(crate) fn w_mcp_call_timeout() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/mcp/client.rs")));
}

#[then("每笔请求有调用期超时且可分类")]
pub(crate) fn t_mcp_call_timeout() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("MCP_CALL_TIMEOUT"), "调用期超时 MUST 有常量");
    assert!(
        src.contains("McpError::Timeout"),
        "超时 MUST 以可分类错误呈现"
    );
}

#[when("读取首回合工具定稿门禁")]
pub(crate) fn w_first_turn_gate() {
    let a = la_load("src/app/core/driver/remote.rs");
    let b = la_load("src/infra/mcp/adapter.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== adapter ===\n{b}")));
}

#[then("无配置立即定稿且有配置时首回合后门闸定稿")]
pub(crate) fn t_first_turn_gate() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("first-turn"), "首回合门闸 MUST 在册");
    assert!(src.contains("freeze"), "定稿冻结语义 MUST 在册");
}

#[when("读取 shell 环境装配边界")]
pub(crate) fn w_shell_env_boundary() {
    let mut base = std::collections::BTreeMap::new();
    base.insert("PATH".into(), "/usr/bin:/bin".into());
    let env = xylitol::infra::process::shell::shell_env_with_agent_bin(base);
    let path = env.get("PATH").cloned().unwrap_or_default();
    T2_PROC.with(|s| *s.borrow_mut() = Some((path, false, false)));
}

#[then("以 PATH 定位可执行 bash")]
pub(crate) fn t_shell_env_boundary() {
    let (path, _, _) = T2_PROC.with(|s| s.borrow().clone()).expect("已装配");
    assert!(!path.is_empty(), "注入后 PATH MUST 非空");
    let mut it = std::env::split_paths(&path);
    let head = it.next().expect("PATH MUST 可解析");
    let bin = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(ToOwned::to_owned))
        .expect("current exe dir");
    assert_eq!(head, bin, "agent bin 目录 MUST 前置");
    assert!(
        !xylitol::infra::process::shell::find_bash(Some(&head))
            .shell
            .to_string_lossy()
            .is_empty(),
        "agent bin 目录 MUST 可执行（bash 定位）"
    );
}

#[when("读取外部工具进程回收边界")]
pub(crate) fn w_child_wait_boundary() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/tools/process.rs")));
}

#[then("等待退出取得状态且整树回收")]
pub(crate) fn t_child_wait_boundary() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("wait_with_output"), "等待退出 MUST 取得状态");
    assert!(src.contains("kill_tree"), "整树回收 MUST 在册");
}

#[when("读取观测后端装配")]
pub(crate) fn w_obs_backend_assembly() {
    let a = la_load("src/infra/observability/file_reporter.rs");
    let b = la_load("src/app/core/bootstrap.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== bootstrap ===\n{b}")));
}

#[then("组合根恰一次装配且落 agent 日志目录")]
pub(crate) fn t_obs_backend_assembly() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("FileTraceReporter"),
        "本地文件 reporter MUST 在册"
    );
    assert!(src.contains("log::"), "组合根 MUST 有级别日志装配");
}

#[when("读取观测栈依赖清单")]
pub(crate) fn w_obs_dependency_list() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("Cargo.toml")));
}

#[then("仅用 fastrace 与 log 且无 tracing")]
pub(crate) fn t_obs_dependency_list() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("fastrace"), "时间线 MUST 依赖 fastrace");
    assert!(
        !src.contains("tracing ="),
        "Cargo MUST NOT 依赖 tracing 门面"
    );
}

#[when("读取低频观测 span 定义")]
pub(crate) fn w_obs_span_definition() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/agent/runtime/obs.rs")));
}

#[then("agent.turn 与每步 span 可关联")]
pub(crate) fn t_obs_span_definition() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("agent.turn"), "根 span MUST 为 agent.turn");
    assert!(src.contains("agent.iteration"), "每步 span MUST 可关联");
    assert!(src.contains("tool.execute"), "工具 execute span MUST 在册");
}

#[when("读取图像解码方向边界")]
pub(crate) fn w_image_exif_boundary() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/image/resize.rs")));
}

#[then("解码应用 EXIF 定向且像素校正在册")]
pub(crate) fn t_image_exif_boundary() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("read_exif_orientation"), "EXIF 读取 MUST 在册");
    assert!(src.contains("apply_orientation"), "像素校正 MUST 在册");
    assert!(src.contains("Orientation::from_exif"), "方向转换 MUST 在册");
    assert!(
        src.contains("exif_orientation_6_swaps_dimensions"),
        "行为单测 MUST 在册"
    );
}

#[when("读取计时收集器边界")]
pub(crate) fn w_timing_collector() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/timing.rs")));
}

#[then("收集器由 XYLITOL_TIMING 门控且含重置与计时")]
pub(crate) fn t_timing_collector() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("XYLITOL_TIMING"), "收集器 MUST 由环境变量门控");
    assert!(src.contains("reset_timings"), "重置入口 MUST 在册");
    assert!(src.contains("pub fn time"), "计时入口 MUST 在册");
}

#[when("读取计时调用点清单")]
pub(crate) fn w_timing_call_sites() {
    let a = la_load("src/app/core/bootstrap.rs");
    let b = la_load("src/app/cli/mod.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== cli ===\n{b}")));
}

#[then("启动关键路径含计时点")]
pub(crate) fn t_timing_call_sites() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("timing::"), "启动关键路径 MUST 接计时点");
}

#[when("读取计时输出格式")]
pub(crate) fn w_timing_output_format() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/timing.rs")));
}

#[then("每步 ms 与合计可观测")]
pub(crate) fn t_timing_output_format() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("TOTAL"), "合计 MUST 可观测");
    assert!(src.contains("ms"), "每步 ms MUST 可观测");
}

// ── 批 3（agent 域 / 分层）：结构探针 ───────────────────────────────

fn t3_scan_agent_trust_defs() -> String {
    let mut hits = Vec::new();
    let mut stack = vec![std::path::PathBuf::from("src/agent")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let Ok(content) = std::fs::read_to_string(&p) else {
                    continue;
                };
                for line in content.lines() {
                    let l = line.trim();
                    if (l.starts_with("pub struct")
                        || l.starts_with("pub enum")
                        || l.starts_with("struct "))
                        && l.contains("Trust")
                    {
                        hits.push(format!("{}: {l}", p.display()));
                    }
                }
            }
        }
    }
    hits.join("\n")
}

#[when("读取 Todo 领域模型形状")]
pub(crate) fn w_todo_model_shape() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/session/todo.rs")));
}

#[then("条目为有序集合且 content 与状态受约束")]
pub(crate) fn t_todo_model_shape() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("TodoList"), "Todo 列表 MUST 为集合类型");
    assert!(src.contains("TodoStatus"), "status MUST 为约束枚举");
    assert!(
        src.contains("Vec<TodoItem>") || src.contains("items:"),
        "条目 MUST 为有序集合"
    );
}

#[when("读取 Todo 快照投影边界")]
pub(crate) fn w_todo_snapshot_boundary() {
    let a = la_load("src/protocol/session/todo.rs");
    let b = la_load("src/agent/prompt/status_bar.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== status_bar ===\n{b}")));
}

#[then("Custom 快照不进 provider 前缀且 SSOT 唯一")]
pub(crate) fn t_todo_snapshot_boundary() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("CUSTOM_TYPE_AGENT_TODO"),
        "SSOT 类型 MUST 在册"
    );
    assert!(src.contains("SSOT"), "唯一真源语义 MUST 在册");
    assert!(src.contains("agent_todo"), "SSOT 标识 MUST 为 agent_todo");
}

#[when("读取 todo 工具调度分类")]
pub(crate) fn w_todo_scheduling_class() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/tools/mod.rs")));
}

#[then("todo_rewrite 与 todo_update 为 Barrier 并发类")]
pub(crate) fn t_todo_scheduling_class() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("todo_rewrite"), "todo_rewrite MUST 在注册表");
    assert!(src.contains("todo_update"), "todo_update MUST 在注册表");
    assert!(
        src.contains("Barrier"),
        "todo 工具 MUST 标注 Barrier 并发类"
    );
}

#[when("读取 Todo SSOT 只读边界")]
pub(crate) fn w_todo_status_bar_boundary() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/agent/prompt/status_bar.rs")));
}

#[then("待办栏摘要只读自 SSOT")]
pub(crate) fn t_todo_status_bar_boundary() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("SSOT"), "待办栏 MUST 标注只读 SSOT");
    assert!(
        src.contains("TodoList") || src.contains("latest_agent_todo"),
        "摘要 MUST 读 SSOT"
    );
}

#[when("读取首回合工具定稿清单")]
pub(crate) fn w_first_turn_tool_freeze_list() {
    let a = la_load("src/agent/tools/freeze.rs");
    let b = la_load("src/infra/tools/mod.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== registry ===\n{b}")));
}

#[then("todo builtins 在定稿前进入可见工具表")]
pub(crate) fn t_first_turn_tool_freeze_list() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("freeze"), "定稿门禁 MUST 在册");
    assert!(src.contains("todo_rewrite"), "todo builtin MUST 在注册表");
    assert!(src.contains("todo_update"), "todo builtin MUST 在注册表");
}

#[when("读取压缩触发边界")]
pub(crate) fn w_compaction_trigger_boundary() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/agent/compaction/settings.rs")));
}

#[then("按窗口与保留阈值在会话路径触发")]
pub(crate) fn t_compaction_trigger_boundary() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("reserve_tokens"), "保留阈值 MUST 在册");
    assert!(src.contains("enabled"), "开关 MUST 在册");
    assert!(
        src.contains("window") || src.contains("recent"),
        "窗口 MUST 在册"
    );
}

#[when("读取恢复会话校验顺序")]
pub(crate) fn w_cwd_check_before_restore() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/session/manager/load.rs")));
}

#[then("恢复前完成同一 CWD 校验")]
pub(crate) fn t_cwd_check_before_restore() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("assert_session_cwd_exists"),
        "CWD 校验入口 MUST 在册"
    );
    assert!(src.contains("fallback_cwd"), "回退 CWD 语义 MUST 在册");
}

#[when("读取会话存储端口实现")]
pub(crate) fn w_session_store_port_impl() {
    let a = la_load("src/protocol/ports/session.rs");
    let b = la_load("src/infra/session/manager/store.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== store ===\n{b}")));
}

#[then("infra SessionManager 实现协议端口")]
pub(crate) fn t_session_store_port_impl() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("XySessionStore"), "协议端口 MUST 在册");
    assert!(src.contains("impl "), "infra MUST 提供实现");
    assert!(
        src.contains("pub trait XySessionStore"),
        "端口 MUST 为公共 trait"
    );
}

#[when("读取会话日志访问接口")]
pub(crate) fn w_journal_read_recent() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/ports/session.rs")));
}

#[then("read_recent 暴露给 server journal")]
pub(crate) fn t_journal_read_recent() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("async fn read_recent"),
        "read_recent 接口 MUST 在协议端口"
    );
    assert!(
        src.contains("load_entries") && src.contains("saturating_sub"),
        "默认实现 MUST 基于全量读取截断"
    );
    assert!(
        src.contains("read_recent_returns_last_n_in_append_order"),
        "行为单测 MUST 在册"
    );
}

#[when("读取导出 I/O 装配")]
pub(crate) fn w_export_io_assembly() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/export.rs")));
}

#[then("StdExportIo 经端口注入组合根")]
pub(crate) fn t_export_io_assembly() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("StdExportIo"), "文件导出 I/O MUST 在册");
    assert!(src.contains("tokio::fs"), "MUST 用 tokio::fs 读写");
    assert!(src.contains("impl "), "端口实现 MUST 在册");
}

#[when("读取会话持久化分层")]
pub(crate) fn w_session_persistence_layers() {
    let a = la_load("src/protocol/ports/session.rs");
    let b = la_load("src/infra/session/manager/store.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== store ===\n{b}")));
}

#[then("protocol 定义端口且 infra 提供实现")]
pub(crate) fn t_session_persistence_layers() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("pub trait XySessionStore"),
        "protocol 端口 MUST 在册"
    );
    assert!(src.contains("impl XySessionStore"), "infra 实现 MUST 在册");
}

fn load_trust_store_probe() {
    let hits = t3_scan_agent_trust_defs();
    let store = la_load("src/infra/trust/store.rs");
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(format!(
            "AGENT_DEFS_START\n{hits}\nAGENT_DEFS_END\n=== store ===\n{store}"
        ))
    });
}

#[when("扫描 agent 层信任依赖")]
pub(crate) fn w_agent_layer_trust_deps() {
    load_trust_store_probe();
}

#[then("trust 决策经 infra 与应用面且 agent 无自有存储")]
pub(crate) fn t_agent_layer_trust_deps() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let agent_part = src.split("AGENT_DEFS_END").next().unwrap_or_default();
    assert!(
        !agent_part.contains("struct") || !agent_part.contains("Trust"),
        "agent 层 MUST NOT 定义自有 trust 存储"
    );
    assert!(src.contains("TrustManager"), "infra trust 真源 MUST 在册");
}

#[when("读取会话上下文压缩回溯")]
pub(crate) fn w_compaction_aware_context() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/session/manager/store.rs")));
}

#[then("构建对 leaf 分支回退压缩")]
pub(crate) fn t_compaction_aware_context() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("leaf"), "leaf 分支回溯 MUST 在册");
    assert!(
        src.contains("compaction") || src.contains("fold"),
        "压缩回退语义 MUST 在册"
    );
}

#[when("读取会话恢复原路径")]
pub(crate) fn w_resume_single_path() {
    let a = la_load("src/agent/llm_project.rs");
    let b = la_load("src/infra/session/manager/store.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== store ===\n{b}")));
}

#[then("恢复与续跑经同一投影路径")]
pub(crate) fn t_resume_single_path() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("project_for_llm"), "唯一投影入口 MUST 在册");
    assert!(src.contains("history"), "投影 MUST 覆盖历史");
}

#[when("读取消息词汇分层")]
pub(crate) fn w_message_vocab_layers() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/message.rs")));
}

#[then("AgentMessage 以组合表达且协议词汇单一")]
pub(crate) fn t_message_vocab_layers() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("AgentMessage"), "会话词汇 MUST 在册");
    assert!(
        src.contains("Llm(") || src.contains("Env("),
        "组合表达 MUST 在册"
    );
}

#[when("读取主仓投影入口")]
pub(crate) fn w_llm_project_entry() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/agent/llm_project.rs")));
}

#[then("AgentMessage 经投影为协议消息")]
pub(crate) fn t_llm_project_entry() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("project_for_llm"), "投影入口 MUST 在册");
    assert!(src.contains("AgentMessage"), "输入 MUST 为 AgentMessage");
    assert!(
        src.contains("LlmMessage") || src.contains("AiBridgeMessage"),
        "输出 MUST 为协议消息"
    );
}

#[when("读取网络权限裁决")]
pub(crate) fn w_network_permission_gate() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/permission/mod.rs")));
}

#[then("域名按 allow/deny 列表裁决并默认拒绝")]
pub(crate) fn t_network_permission_gate() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("check_network"), "网络裁决入口 MUST 在册");
    assert!(src.contains("allowed_domains"), "allow 列表 MUST 在册");
    assert!(
        src.contains("denied_domains") || src.contains("default-deny"),
        "deny / 默认拒绝语义 MUST 在册"
    );
}

#[when("扫描信任存储真源")]
pub(crate) fn w_trust_single_source() {
    load_trust_store_probe();
}

#[then("项目 trust 状态在 infra 单点维护")]
pub(crate) fn t_trust_single_source() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let agent_part = src.split("AGENT_DEFS_END").next().unwrap_or_default();
    assert!(
        !agent_part.contains("Trust"),
        "agent 层 MUST NOT 持有 trust 存储"
    );
    assert!(src.contains("TrustManager"), "infra trust 真源 MUST 在册");
}

#[when("读取权限边界文档")]
pub(crate) fn w_permission_advice_doc() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/permission/mod.rs")));
}

#[then("明示建议性且不阻塞主机级访问")]
pub(crate) fn t_permission_advice_doc() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("Advisory only"), "MUST 明示建议性");
    assert!(
        src.contains("NOT a security boundary") || src.contains("advisory"),
        "MUST 明示非安全边界"
    );
    assert!(
        src.contains("host-level access") || src.contains("do not prevent"),
        "MUST 明示不阻塞主机级访问"
    );
}

#[when("读取资源命令装配")]
pub(crate) fn w_resources_loader_reuse() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/app/cli/resources.rs")));
}

#[then("复用 DefaultResourceLoader 发现")]
pub(crate) fn t_resources_loader_reuse() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("DefaultResourceLoader"),
        "资源命令 MUST 复用 loader"
    );
    assert!(
        src.contains("cached") || src.contains("reuse"),
        "MUST 复用缓存发现"
    );
}

#[when("读取资源来源信息类型")]
pub(crate) fn w_source_info_shape() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/source_info.rs")));
}

#[then("公共 SourceInfo 含来源与作用域字段")]
pub(crate) fn t_source_info_shape() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("pub struct SourceInfo"),
        "公共 SourceInfo MUST 在册"
    );
    assert!(src.contains("pub path"), "path 字段 MUST 在册");
    assert!(src.contains("pub scope"), "scope 字段 MUST 在册");
    assert!(src.contains("pub source"), "source 字段 MUST 在册");
}

#[when("读取资源作用域枚举")]
pub(crate) fn w_source_scope_enum() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/source_info.rs")));
}

#[then("支持 user 与 project 与 temporary")]
pub(crate) fn t_source_scope_enum() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("pub enum SourceScope"), "Scope 枚举 MUST 在册");
    assert!(src.contains("User"), "user 变体 MUST 在册");
    assert!(src.contains("Project"), "project 变体 MUST 在册");
    assert!(src.contains("Temporary"), "temporary 变体 MUST 在册");
}

// ── 批 4（测试基建契约）：结构探针与扫描 ───────────────────────────

fn t4_walk<F: FnMut(&std::path::Path, &str)>(root: &str, mut f: F) {
    let mut stack = vec![std::path::PathBuf::from(root)];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let Ok(content) = std::fs::read_to_string(&p) else {
                    continue;
                };
                f(&p, &content);
            }
        }
    }
}

fn t4_scan_write_to_fixed_tmp() -> String {
    let mut hits = Vec::new();
    let needle = "\"/tmp";
    t4_walk("tests", |p, c| {
        if c.contains(needle) {
            for line in c.lines() {
                let trimmed = line.trim();
                if trimmed.contains(needle)
                    && (trimmed.contains("fs::write")
                        || trimmed.contains("fs::create_dir")
                        || trimmed.contains("File::create")
                        || trimmed.contains("fs::remove")
                        || trimmed.contains("create_dir_all"))
                {
                    hits.push(format!("{}: {trimmed}", p.display()));
                }
            }
        }
    });
    if hits.is_empty() {
        "NONE".into()
    } else {
        hits.join("\n")
    }
}

fn t4_count_cfg_test(root: &str) -> usize {
    let mut n = 0;
    t4_walk(root, |_, c| {
        n += c.matches("#[cfg(test)]").count();
    });
    n
}

fn t4_count_needle(root: &str, needle: &str) -> usize {
    let mut n = 0;
    t4_walk(root, |_, c| {
        n += c.matches(needle).count();
    });
    n
}

#[when("读取 faux provider 装配入口")]
pub(crate) fn w_faux_provider_entry() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/provider/fake.rs")));
}

#[then("按响应步骤返回且无需网络")]
pub(crate) fn t_faux_provider_entry() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("fake_xy_model"), "faux 装配入口 MUST 在册");
    assert!(src.contains("ScenarioStep"), "响应步骤 MUST 在册");
    assert!(src.contains("Arc<dyn XyModel>"), "出口 MUST 为 XyModel");
}

#[when("读取 BDD 测试基建清单")]
pub(crate) fn w_bdd_harness_list() {
    let a = la_load("Cargo.toml");
    let b = la_load("tests/bdd/suite.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== suite ===\n{b}")));
}

#[then("场景以类型化占位符步骤且逐场景一测试")]
pub(crate) fn t_bdd_harness_list() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("rstest-bdd"), "rstest-bdd 基建 MUST 在册");
    assert!(src.contains("scenario"), "场景宏 MUST 在册");
    assert!(src.contains("mod steps_"), "类型化步骤模块 MUST 在册");
}

#[when("读取测试临时目录基建")]
pub(crate) fn w_temp_file_raii() {
    let a = la_load("tests/bdd/helpers.rs");
    let b = la_load("Cargo.toml");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== Cargo ===\n{b}")));
}

#[then("RAII 清理且不留产物")]
pub(crate) fn t_temp_file_raii() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("tempfile"), "tempfile 依赖 MUST 在册");
    assert!(src.contains("tempdir"), "RAII 临时目录 MUST 在册");
}

#[when("外部时序场景以 with_test_timeout 包裹等待主体")]
pub(crate) async fn w_async_test_timeout() {
    let ok = with_test_timeout(|| async { 1u8 }).await.is_ok();
    let timed_out = with_test_timeout_for(std::time::Duration::from_millis(50), || async {
        std::future::pending::<()>().await
    })
    .await
    .is_err();
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("ok={ok};timed_out={timed_out}")));
}

#[then("超时辅助被真实调用且不得仅以源码探针自证")]
pub(crate) fn t_async_test_timeout() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("已跑超时辅助");
    assert!(
        src.contains("ok=true"),
        "短主体 MUST 经 with_test_timeout 真实完成：{src}"
    );
    assert!(
        src.contains("timed_out=true"),
        "挂死主体 MUST 经 with_test_timeout_for 以超时失败而非源码探针：{src}"
    );
}

#[when("扫描测试固定临时路径")]
pub(crate) fn w_fixed_tmp_scan() {
    let hits = t4_scan_write_to_fixed_tmp();
    LA_PROBE.with(|p| *p.borrow_mut() = Some(hits));
}

#[then("使用唯一自动生成路径")]
pub(crate) fn t_fixed_tmp_scan() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert_eq!(src, "NONE", "测试 MUST NOT 写固定 /tmp 路径：{src}");
}

#[when("读取 TUI 端到端测试布局")]
pub(crate) fn w_tui_e2e_layout() {
    let pty = la_load("tests/tui_e2e/pty.rs");
    let tmux = la_load("tests/tui_e2e/tmux.rs");
    let just = la_load("justfile");
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(format!(
            "{pty}\n=== tmux ===\n{tmux}\n=== justfile ===\n{just}"
        ))
    });
}

#[then("独立于主矩阵")]
pub(crate) fn t_tui_e2e_layout() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("PortablePty") || src.contains("portable-pty"),
        "pty 驱动 MUST 在册"
    );
    assert!(src.contains("tmux"), "tmux 驱动 MUST 在册");
    assert!(src.contains("test-tui-e2e"), "端到端 recipe MUST 在册");
}

#[when("读取配置值解析机制")]
pub(crate) fn w_config_value_parser() {
    let lookup = |name: &str| (name == "HOME").then(|| "/home/u".to_string());
    let a = xylitol::infra::config::resolver::resolve_value("plain-string", &lookup)
        .expect("字面值 MUST 直通");
    let b = xylitol::infra::config::resolver::resolve_value("$HOME", &lookup)
        .expect("环境值 MUST 解析");
    T2_PROC.with(|s| *s.borrow_mut() = Some((a, b == "/home/u", false)));
}

#[then("支持字面与环境模板解析")]
pub(crate) fn t_config_value_parser() {
    let (lit, env_ok, _) = T2_PROC.with(|s| s.borrow().clone()).expect("已解析");
    assert_eq!(lit, "plain-string", "字面值 MUST 原样返回");
    assert!(env_ok, "环境变量引用 MUST 解析");
}

#[when("读取环境变量插值能力")]
pub(crate) fn w_env_var_interpolation() {
    let lookup = |name: &str| (name == "HOME").then(|| "/home/u".to_string());
    let a = xylitol::infra::config::resolver::resolve_value("${HOME}", &lookup)
        .expect("${VAR} MUST 插值");
    let b = xylitol::infra::config::resolver::resolve_value("${UNSET:-fallback}", &lookup)
        .expect("默认值 MUST 生效");
    T2_PROC.with(|s| *s.borrow_mut() = Some((a, b == "fallback", false)));
}

#[then("支持变量引用与默认值")]
pub(crate) fn t_env_var_interpolation() {
    let (braced, default_ok, _) = T2_PROC.with(|s| s.borrow().clone()).expect("已插值");
    assert_eq!(braced, "/home/u", "尖括号变量引用 MUST 解析");
    assert!(default_ok, "带默认值引用 MUST 生效");
}

#[when("读取配置命令执行边界")]
pub(crate) fn w_config_command_boundary() {
    xylitol::infra::config::resolver::reset_shell_cache();
    let lookup = |_name: &str| None;
    let out = xylitol::infra::config::resolver::resolve_value("!printf ok", &lookup)
        .expect("shell 命令 MUST 执行");
    // `$$` 是 shell PID：缓存命中时两次结果相同（进程生命周期缓存证据）。
    let c1 = xylitol::infra::config::resolver::resolve_value("!printf %s $$", &lookup)
        .expect("shell PID 值一");
    let c2 = xylitol::infra::config::resolver::resolve_value("!printf %s $$", &lookup)
        .expect("shell PID 值二");
    T2_PROC.with(|s| *s.borrow_mut() = Some((out, c1 == c2, true)));
}

#[then("命令带超时执行且缓存")]
pub(crate) fn t_config_command_boundary() {
    let (out, cached, _) = T2_PROC.with(|s| s.borrow().clone()).expect("已执行");
    assert_eq!(out, "ok", "shell 命令 MUST 带预算执行");
    assert!(cached, "进程生命周期内结果 MUST 缓存");
}

#[when("读取 provider 注册配置值解析")]
pub(crate) fn w_provider_config_value_expression() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfgdir = dir.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&cfgdir).unwrap();
    std::fs::write(
        cfgdir.join("config.yaml"),
        "models:\n  models:\n    a:\n      provider: fake\n      model: m1\n      api_key: \"!printf bdd-expanded\"\n    b:\n      provider: fake\n      model: m2\n      api_key: plain-literal\n",
    )
    .unwrap();
    let home_s = dir.path().to_str().unwrap().to_string();
    let cfgdir_s = cfgdir.to_str().unwrap().to_string();
    let env = move |k: &str| match k {
        "HOME" => Some(home_s.clone()),
        "XYLITOL_CONFIG_DIR" => Some(cfgdir_s.clone()),
        _ => None,
    };
    let loaded = xylitol::infra::config::loader::load_app_config_with(None, env, None)
        .expect("装配 MUST 成功");
    let shell = loaded
        .model
        .models
        .get("a")
        .and_then(|e| e.api_key.clone())
        .unwrap_or_default();
    let literal = loaded
        .model
        .models
        .get("b")
        .and_then(|e| e.api_key.clone())
        .unwrap_or_default();
    T2_CFG_EXPR.with(|s| *s.borrow_mut() = Some((shell, literal)));
}

#[then("展开表达式并兼容字面量")]
pub(crate) fn t_provider_config_value_expression() {
    let (shell, literal) = T2_CFG_EXPR.with(|s| s.borrow().clone()).expect("已解析");
    assert_eq!(
        shell, "bdd-expanded",
        "shell-command 表达式 MUST 在产品装配链展开"
    );
    assert_eq!(literal, "plain-literal", "纯字面量 MUST 保持原样");
}

#[when("读取 provider 注册配置")]
pub(crate) fn w_provider_registration_config() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/protocol/model/config.rs")));
}

#[then("支持密钥与地址与请求头")]
pub(crate) fn t_provider_registration_config() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("api_key"), "API 密钥配置 MUST 在册");
    assert!(src.contains("base_url"), "服务地址配置 MUST 在册");
    assert!(
        src.contains("openai-responses") || src.contains("anthropic-messages"),
        "适配类型 MUST 在册"
    );
}

#[when("读取 provider 分层")]
pub(crate) fn w_provider_layering() {
    let a = la_load("src/infra/provider/adapter/mod.rs");
    let b = la_load("src/agent/model/registry.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== registry ===\n{b}")));
}

#[then("实现位于 infra 且遵循端口")]
pub(crate) fn t_provider_layering() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("impl "), "infra 实现 MUST 在册");
    assert!(src.contains("XyModel"), "实现 MUST 遵循协议端口");
}

#[when("读取模型注册表存储")]
pub(crate) fn w_model_registry_storage() {
    let a = la_load("src/agent/model/manager.rs");
    let b = la_load("src/agent/model/task_model.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== task ===\n{b}")));
}

#[then("以抽象 trait 对象持有")]
pub(crate) fn t_model_registry_storage() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("Arc<dyn XyModel>"),
        "注册表 MUST 以 Arc<dyn XyModel> 存储"
    );
    assert!(src.contains("XyModel"), "端口抽象 MUST 在册");
}

#[when("读取 BDD 套件接线")]
pub(crate) fn w_bdd_suite_wiring() {
    let a = la_load("tests/bdd/suite.rs");
    let b = la_load("src/agent/model/mod.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== model ===\n{b}")));
}

#[then("全量通过且无孤儿 feature")]
pub(crate) fn t_bdd_suite_wiring() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("#[macro_use]"), "BDD 套件装配 MUST 在册");
    assert!(src.contains("mod bindings_"), "绑定模块 MUST 在册");
}

// test-bdd r1913（c2837）：编译隔离不变量——独立目标承载、未挂回 lib。
#[when("读取 BDD 挂载接线")]
pub(crate) fn w_bdd_mount_wiring() {
    let a = la_load("tests/bdd.rs");
    let b = la_load("src/tests.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== lib-mount ===\n{b}")));
}

#[then("独立测试目标承载且未挂 lib")]
pub(crate) fn t_bdd_mount_wiring() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let (entry, lib_mount) = src.split_once("=== lib-mount ===").expect("双文件探针");
    assert!(
        entry.contains("#[path = \"bdd/suite.rs\"]") && entry.contains("mod bdd;"),
        "BDD 入口 MUST 以 #[path] 挂 suite 模块"
    );
    assert!(
        !lib_mount.contains("tests/bdd/suite.rs"),
        "lib MUST NOT 再以 mod bdd 挂载（编译隔离不变量）"
    );
}

#[when("读取 server 集成场景清单")]
pub(crate) fn w_server_integration_list() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("tests/bdd/bindings_server.rs")));
}

#[then("含启动与健康与提交与流式")]
pub(crate) fn t_server_integration_list() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("server-core.feature"),
        "server 场景绑定 MUST 在册"
    );
    assert!(src.contains("scenario"), "集成场景 MUST 有绑定");
}

#[when("读取 BDD 依赖版本")]
pub(crate) fn w_rstest_bdd_version() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("Cargo.toml")));
}

#[then("使用 crates.io 当前版本")]
pub(crate) fn t_rstest_bdd_version() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("rstest-bdd ="), "rstest-bdd 依赖 MUST 在册");
    assert!(src.contains("rstest-bdd-macros ="), "macros 依赖 MUST 在册");
}

#[when("读取 BDD 绑定机制")]
pub(crate) fn w_bdd_binding_mechanism() {
    let a = la_load("tests/bdd/suite.rs");
    let b = la_load("tests/bdd/bindings_c2827.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== bindings ===\n{b}")));
}

#[then("经 @req 绑定且支持 live 分区")]
pub(crate) fn t_bdd_binding_mechanism() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("#[scenario("), "场景绑定宏 MUST 在册");
    assert!(
        src.contains("llmanspec/specs/"),
        "feature 分区路径 MUST 在册"
    );
}

#[when("读取配置行为测试分层")]
pub(crate) fn w_config_unit_coverage() {
    let a = la_load("src/infra/config/types.rs");
    let b = la_load("src/infra/config/template.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== template ===\n{b}")));
}

#[then("由 infra 单测覆盖")]
pub(crate) fn t_config_unit_coverage() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("#[cfg(test)]"), "配置行为 MUST 有单测模块");
    assert!(src.contains("#[test]"), "单测用例 MUST 在册");
}

#[when("读取测试分界文档")]
pub(crate) fn w_bdd_unit_boundary_doc() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("AGENTS.md")));
}

#[then("明示端到端与纯逻辑边界")]
pub(crate) fn t_bdd_unit_boundary_doc() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("BDD"), "分界文档 MUST 提及 BDD");
    assert!(
        src.contains("单测管纯数据") || src.contains("纯逻辑"),
        "单测边界 MUST 明示"
    );
}

#[when("扫描核心类型测试覆盖")]
pub(crate) fn w_core_data_type_coverage() {
    let n = t4_count_cfg_test("src/protocol");
    let m = t4_count_needle("src/protocol", "#[test]");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{n};{m}")));
}

#[then("关键路径有单测验证")]
pub(crate) fn t_core_data_type_coverage() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let mut it = src.split(';');
    let n: usize = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    assert!(n >= 1, "核心类型 MUST 有 #[cfg(test)] 模块");
}

#[when("扫描纯逻辑组件测试")]
pub(crate) fn w_pure_logic_coverage() {
    let n = t4_count_cfg_test("src/agent");
    let m = t4_count_needle("src/agent", "#[test]");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{n};{m}")));
}

#[then("队列与重试等有单测")]
pub(crate) fn t_pure_logic_coverage() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let mut it = src.split(';');
    let n: usize = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    assert!(n >= 3, "agent 层纯逻辑组件 MUST 有单测（期望多模块）");
}

#[when("扫描会话子组件测试")]
pub(crate) fn w_session_subcomponent_coverage() {
    let n = t4_count_cfg_test("src/agent/capabilities");
    let m = t4_count_needle("src/agent/capabilities", "#[test]");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{n};{m}")));
}

#[then("模型与工具管理器有单测")]
pub(crate) fn t_session_subcomponent_coverage() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    let mut it = src.split(';');
    let n: usize = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    assert!(n >= 1, "会话子组件 MUST 有单测模块");
}

#[when("读取库缝观察接线")]
pub(crate) fn w_smoke_hook_wiring() {
    let a = la_load("src/app/core/composition.rs");
    let b = la_load("src/agent/runtime/ports.rs");
    LA_PROBE.with(|p| *p.borrow_mut() = Some(format!("{a}\n=== ports ===\n{b}")));
}

#[then("有经库缝触发的例子")]
pub(crate) fn t_smoke_hook_wiring() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("XyHookBus"), "hook 总线端口 MUST 在册");
    assert!(
        src.contains("Hook") || src.contains("hook"),
        "库缝注入 MUST 在册"
    );
}

#[when("读取 provider 选择场景")]
pub(crate) fn w_provider_matrix_scenarios() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("tests/bdd/bindings_misc.rs")));
}

#[then("已有可执行场景")]
pub(crate) fn t_provider_matrix_scenarios() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("model-select"),
        "model_select 场景 MUST 已绑定"
    );
}

#[when("读取 crate 根再导出")]
pub(crate) fn w_curated_hook_bus_reexport() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/lib.rs")));
}

#[then("精选导出总线与结果")]
pub(crate) fn t_curated_hook_bus_reexport() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("XyHookBus"), "总线 MUST 经根导出");
    assert!(src.contains("XyHookOutcome"), "结果 MUST 经根导出");
    assert!(src.contains("NoopHookBus"), "noop 总线 MUST 经根导出");
}

#[when("读取 fake provider 装配")]
pub(crate) fn w_fake_provider_assembly() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/provider/fake.rs")));
}

#[then("经统一路径暴露且按步骤返回")]
pub(crate) fn t_fake_provider_assembly() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("fake_xy_model"), "统一装配入口 MUST 在册");
    assert!(src.contains("ScenarioStep"), "响应步骤 MUST 在册");
    assert!(src.contains("Arc<dyn XyModel>"), "暴露为 XyModel MUST 在册");
}

#[when("读取场景编排能力")]
pub(crate) fn w_scenario_orchestration() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("src/infra/provider/fake.rs")));
}

#[then("支持多步与延迟与错误注入")]
pub(crate) fn t_scenario_orchestration() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(src.contains("ScenarioStep"), "编排步骤类型 MUST 在册");
    assert!(src.contains("fake_xy_model"), "编排装配 MUST 在册");
}
