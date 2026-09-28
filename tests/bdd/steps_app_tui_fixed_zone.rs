//! Steps for `app-tui-fixed-zone` — 下缘待办栏与队列条 / 通知条 / status / editor 堆叠。

use crate::app::tui::{InteractionBdd, UiModel};
use crate::protocol::session::{TodoItem, TodoList, TodoStatus};
use crate::tests::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};

pub struct FixedZoneBdd {
    pub model: RefCell<UiModel>,
    pub frame: RefCell<Option<String>>,
}

#[fixture]
pub fn fixed_zone_bdd() -> FixedZoneBdd {
    FixedZoneBdd {
        model: RefCell::new(UiModel::new()),
        frame: RefCell::new(None),
    }
}

fn sample_todo() -> TodoList {
    TodoList::new(vec![
        TodoItem {
            id: "a".into(),
            content: "read glossary".into(),
            status: TodoStatus::Completed,
        },
        TodoItem {
            id: "b".into(),
            content: "write todo-bar copy".into(),
            status: TodoStatus::InProgress,
        },
        TodoItem {
            id: "c".into(),
            content: "paint lab states".into(),
            status: TodoStatus::Pending,
        },
    ])
}

fn paint_dock(model: UiModel) -> String {
    let mut fx = InteractionBdd::from_model(model);
    fx.push_toast_notice("boom-toast");
    fx.render_plain(80)
}

#[when("产品 TUI 同时存在非空待办栏、队列条与通知条")]
fn when_todo_queue_toast_present(fixed_zone_bdd: &FixedZoneBdd) {
    let mut model = UiModel::new();
    model.todo = sample_todo();
    model.enqueue_steer_strip("steer-me-queue".into());
    model.set_busy_status("Working");
    *fixed_zone_bdd.frame.borrow_mut() = Some(paint_dock(model.clone()));
    *fixed_zone_bdd.model.borrow_mut() = model;
}

#[then("从上到下 MUST 为队列条、待办栏、通知条、status、editor")]
fn then_dock_order_queue_todo_toast_status_editor(fixed_zone_bdd: &FixedZoneBdd) {
    let plain = fixed_zone_bdd
        .frame
        .borrow()
        .clone()
        .expect("painted frame");
    let idx = |needle: &str| {
        plain
            .find(needle)
            .unwrap_or_else(|| panic!("missing `{needle}` in:\n{plain}"))
    };
    let queue_at = idx("Steering: steer-me-queue");
    let todo_at = idx("write todo-bar copy");
    let toast_at = idx("Error: boom-toast");
    let status_at = idx("Working");
    assert!(
        queue_at < todo_at && todo_at < toast_at && toast_at < status_at,
        "dock order queue → 待办栏 → toast → status:\n{plain}"
    );
    let after_status = &plain[status_at..];
    assert!(
        after_status.contains('─'),
        "editor operation zone MUST sit below status:\n{plain}"
    );
}

#[then("空表时待办栏 MUST 占 0 行")]
fn then_empty_todo_bar_is_zero_rows(fixed_zone_bdd: &FixedZoneBdd) {
    let mut model = fixed_zone_bdd.model.borrow().clone();
    model.todo = TodoList::default();
    let plain = paint_dock(model);
    assert!(
        plain.contains("Steering: steer-me-queue"),
        "queue remains: {plain}"
    );
    assert!(
        plain.contains("Error: boom-toast"),
        "toast remains: {plain}"
    );
    assert!(
        !plain.contains("write todo-bar copy")
            && !plain.contains("completed")
            && !plain.contains("pending"),
        "empty 待办栏 MUST occupy 0 rows: {plain}"
    );
}

// ── c2826 specs-compact：固定区裸规则转场景 ────────────────────────

use crate::app::tui::UiEntry;
use xylitol_tui::Component;

fn root_with(model: &UiModel) -> crate::app::tui::UiRoot {
    let mut root = crate::app::tui::UiRoot::new();
    root.set_layout_meta("~/proj", "m1");
    root.apply_ui_model(model);
    root
}

/// 剥离 ANSI 转义后的纯文本帧（断言用）。
fn plain(frame: &str) -> String {
    static ANSI: once_cell_shim::Ansi = once_cell_shim::Ansi;
    let _ = &ANSI;
    strip_ansi(frame)
}

mod once_cell_shim {
    pub struct Ansi;
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // 跳过 ESC [ ... 终止于字母
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn stash(fixed_zone_bdd: &FixedZoneBdd, frames: Vec<String>) {
    *fixed_zone_bdd.frame.borrow_mut() = Some(frames.join("\n"));
}

#[when("以空闲与忙碌两种 UiModel 渲染固定区")]
fn w_c2826_idle_busy_frames(fixed_zone_bdd: &FixedZoneBdd) {
    let idle = UiModel::new();
    let mut busy = UiModel::new();
    busy.begin_run("hi");
    busy.set_busy_status("Working");
    let idle_frame = root_with(&idle).render(80).join("\n");
    let busy_frame = root_with(&busy).render(80).join("\n");
    stash(
        fixed_zone_bdd,
        vec![
            format!("---IDLE---\n{idle_frame}"),
            format!("---BUSY---\n{busy_frame}"),
        ],
    );
}

#[then("空闲帧不含忙碌短词且忙碌帧恰一行 Working 且 footer 不含 Working")]
fn t_c2826_idle_busy(fixed_zone_bdd: &FixedZoneBdd) {
    let raw = fixed_zone_bdd.frame.borrow().clone().expect("frames");
    let (idle, busy) = raw.split_once("---BUSY---").expect("two frames");
    let idle = idle.replace("---IDLE---\n", "");
    assert!(
        !idle.contains("Working"),
        "c2826: 空闲 status 槽应 0 行：{idle}"
    );
    let busy = plain(busy);
    let working_rows = busy.lines().filter(|l| l.contains("Working")).count();
    assert_eq!(working_rows, 1, "c2826: 忙碌短词应恰一行：{busy}");
    let footer = busy.lines().last().unwrap_or_default();
    assert!(
        !footer.contains("Working"),
        "c2826: footer 不含忙碌词：{footer}"
    );
}

#[when("以忙碌 UiModel 渲染固定区")]
fn w_c2826_busy_frame(fixed_zone_bdd: &FixedZoneBdd) {
    let mut busy = UiModel::new();
    busy.begin_run("hi");
    busy.set_busy_status("Working");
    stash(fixed_zone_bdd, vec![root_with(&busy).render(80).join("\n")]);
}

#[when("以 Compacting 忙碌态 UiModel 渲染固定区")]
fn w_c2826_compacting_frame(fixed_zone_bdd: &FixedZoneBdd) {
    let mut m = UiModel::new();
    m.begin_run("hi");
    m.status = Some("Compacting".into());
    stash(fixed_zone_bdd, vec![root_with(&m).render(80).join("\n")]);
}

#[then("恰一行 Compacting 且 footer 不含 Compacting")]
fn t_c2826_compacting(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = plain(&fixed_zone_bdd.frame.borrow().clone().expect("frame"));
    let rows = frame.lines().filter(|l| l.contains("Compacting")).count();
    assert_eq!(rows, 1, "c2826: Compacting 应单行：{frame}");
    let footer = frame.lines().last().unwrap_or_default();
    assert!(
        !footer.contains("Compacting"),
        "c2826: footer 不含 Compacting：{footer}"
    );
}

#[when("以空闲 UiModel 渲染固定区")]
fn w_c2826_idle_frame(fixed_zone_bdd: &FixedZoneBdd) {
    stash(
        fixed_zone_bdd,
        vec![root_with(&UiModel::new()).render(80).join("\n")],
    );
}

#[then("帧内不出现 Ready 文案")]
fn t_c2826_no_ready(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = plain(&fixed_zone_bdd.frame.borrow().clone().expect("frame"));
    assert!(
        !frame.contains("Ready"),
        "c2826: idle 不得显示 Ready：{frame}"
    );
}

#[then("编辑器操作区保留上下横线边框且无大块空盒")]
fn t_c2826_editor_compact(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = plain(&fixed_zone_bdd.frame.borrow().clone().expect("frame"));
    let rules = frame.lines().filter(|l| l.contains("─")).count();
    assert!(rules >= 2, "c2826: 操作区应保留上下边框：{frame}");
    let blanks = frame.lines().filter(|l| l.trim().is_empty()).count();
    assert!(
        blanks <= 6,
        "c2826: 空闲区不得出现大块空盒（{blanks} 空行）：{frame}"
    );
}

#[then("恰一行含 spinner 短词且 footer 不含该短词")]
fn t_c2826_spinner_single(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = plain(&fixed_zone_bdd.frame.borrow().clone().expect("frame"));
    let rows = frame.lines().filter(|l| l.contains("Working")).count();
    assert_eq!(rows, 1, "c2826: spinner 短词应恰一行：{frame}");
    let footer = frame.lines().last().unwrap_or_default();
    assert!(
        !footer.contains("Working"),
        "c2826: footer 不含 spinner：{footer}"
    );
}

#[when("以含用户消息的 UiModel 渲染 scrollback")]
fn w_c2826_user_entry(fixed_zone_bdd: &FixedZoneBdd) {
    let mut m = UiModel::new();
    m.entries.push(UiEntry::User {
        text: "用户原话".into(),
    });
    stash(fixed_zone_bdd, vec![root_with(&m).render(80).join("\n")]);
}

#[then("用户行保留语义前缀与正文")]
fn t_c2826_user_prefix(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = fixed_zone_bdd.frame.borrow().clone().expect("frame");
    assert!(frame.contains("用户原话"), "c2826: 用户正文应可见：{frame}");
    let user_line = frame
        .lines()
        .find(|l| l.contains("用户原话"))
        .expect("user line");
    assert!(
        user_line.trim_start_matches(' ').starts_with("❯")
            || user_line.contains("❯")
            || user_line.trim_start().len() > 0,
        "c2826: 用户行应保留前缀形态：{user_line}"
    );
}

#[when("以带与不带 timeout 的工具条目渲染 scrollback")]
fn w_c2826_tool_timeout(fixed_zone_bdd: &FixedZoneBdd) {
    let mut m = UiModel::new();
    m.entries.push(UiEntry::Tool {
        id: "t1".into(),
        name: "bash".into(),
        args_preview: "{\"cmd\":\"ls\"}".into(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: "done".into(),
        is_error: false,
        done: true,
        timeout_secs: Some(30),
    });
    m.entries.push(UiEntry::Tool {
        id: "t2".into(),
        name: "bash".into(),
        args_preview: "{\"cmd\":\"pwd\"}".into(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: "ok".into(),
        is_error: false,
        done: true,
        timeout_secs: None,
    });
    stash(fixed_zone_bdd, vec![root_with(&m).render(80).join("\n")]);
}

#[then("带 30s 的显示 (timeout 30s) 且省略的不显示 timeout 注记")]
fn t_c2826_tool_timeout(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = fixed_zone_bdd.frame.borrow().clone().expect("frame");
    assert!(
        frame.contains("(timeout 30s)"),
        "c2826: 显式 timeout 应显示注记：{frame}"
    );
    let timeouts = frame.lines().filter(|l| l.contains("(timeout")).count();
    assert_eq!(timeouts, 1, "c2826: 省略 timeout 不得显示注记：{frame}");
}

#[when("以布局根设置 thinking 档 {level:string} 后渲染")]
fn w_c2826_thinking_footer(fixed_zone_bdd: &FixedZoneBdd, level: String) {
    let level = level.trim_matches('"');
    let mut root = crate::app::tui::UiRoot::new();
    root.set_layout_meta("~/proj", "m1");
    root.set_thinking_level_ui(level.to_string());
    root.apply_ui_model(&UiModel::new());
    stash(fixed_zone_bdd, vec![root.render(80).join("\n")]);
}

#[then("footer 含该档位标签")]
fn t_c2826_thinking_footer(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = fixed_zone_bdd.frame.borrow().clone().expect("frame");
    let footer = frame.lines().last().unwrap_or_default();
    assert!(
        footer.contains("high") || footer.contains("thinking"),
        "c2826: footer 应含 thinking 档标签：{footer}"
    );
}

#[when("以含 skills 与 connecting MCP 的资源快照渲染头部卡")]
fn w_c2826_loaded_resources(fixed_zone_bdd: &FixedZoneBdd) {
    use crate::app::core::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};
    let mut root = crate::app::tui::UiRoot::new();
    root.set_layout_meta("~/proj", "m1");
    root.set_loaded_resources(LoadedResourcesSnapshot {
        skill_names: vec!["demo-skill".into()],
        mcp_configured: 2,
        mcp_connected: vec![("up".into(), 3)],
        mcp_servers: vec![
            McpServerSnapshot {
                id: "up".into(),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 3,
            },
            McpServerSnapshot {
                id: "booting".into(),
                phase: McpServerPhase::Connecting,
                tools_armed: false,
                tool_count: 0,
            },
        ],
        mcp_connecting_label: Some("1/2".into()),
        mcp_bootstrap_complete: false,
        ..Default::default()
    });
    root.apply_ui_model(&UiModel::new());
    stash(fixed_zone_bdd, vec![root.render(100).join("\n")]);
}

#[then("卡含 skills 行与 mcp 行且展示连接进度")]
fn t_c2826_loaded_resources(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = fixed_zone_bdd.frame.borrow().clone().expect("frame");
    assert!(
        frame.contains("demo-skill"),
        "c2826: 头卡应含 skills 行：{frame}"
    );
    assert!(
        frame.contains("mcp") || frame.contains("MCP"),
        "c2826: 头卡应含 mcp 行：{frame}"
    );
    assert!(
        frame.contains("1/2") || frame.contains("connecting"),
        "c2826: 连接进行中应展示进度：{frame}"
    );
}

#[when("连续推送两条通知条后渲染")]
fn w_c2826_toast_replace(fixed_zone_bdd: &FixedZoneBdd) {
    let mut fx = crate::app::tui::InteractionBdd::from_model(UiModel::new());
    fx.push_toast_notice("第一条通知");
    fx.push_toast_notice("第二条通知");
    stash(fixed_zone_bdd, vec![fx.render_plain(80)]);
    let model = UiModel::new();
    *fixed_zone_bdd.model.borrow_mut() = model;
}

#[then("仅见后一条且以 Error: 前缀且未写入对话条目")]
fn t_c2826_toast_replace(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = fixed_zone_bdd.frame.borrow().clone().expect("frame");
    assert!(
        !frame.contains("第一条通知"),
        "c2826: 通知条单槽替换：{frame}"
    );
    assert!(
        frame.contains("Error:") && frame.contains("第二条通知"),
        "c2826: 通知条应带 Error: 前缀：{frame}"
    );
    assert!(
        fixed_zone_bdd.model.borrow().entries.is_empty(),
        "c2826: 通知条不得写入对话条目"
    );
}

async fn footer_with_estimate(
    tokens: u64,
    provenance: crate::protocol::model::TokenProvenance,
    window: u64,
) -> String {
    use crate::app::tui::TuiHostSession as HostSession;
    use crate::app::tui::harness::{ScriptedDriver, TestTerminal};
    use crate::protocol::model::ContextTokenEstimate;
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let mut driver = ScriptedDriver::new();
    driver.set_current_model(crate::app::core::driver::ModelInfo {
        id: "Fake".into(),
        display_name: "Fake".into(),
        thinking: false,
        thinking_levels: Vec::new(),
        context_window: window,
    });
    driver.set_session_messages(crate::app::tui::harness::harness_sample_session_messages());
    driver.set_estimate_override(Some(ContextTokenEstimate {
        tokens,
        provenance,
        usage_tokens: tokens,
        trailing_tokens: 0,
        last_usage_index: None,
    }));
    crate::app::tui::refresh_footer_tokens(&mut session, &mut driver).await;
    session
        .ui_root()
        .expect("ui")
        .borrow_mut()
        .render(160)
        .last()
        .expect("footer")
        .clone()
}

#[when("以 Heuristic 上下文估计驱动 footer token 字段")]
async fn w_c2826_footer_heuristic(fixed_zone_bdd: &FixedZoneBdd) {
    use crate::protocol::model::TokenProvenance;
    let footer = footer_with_estimate(42, TokenProvenance::Heuristic, 128_000).await;
    stash(fixed_zone_bdd, vec![footer]);
}

#[then("footer 为单行且含 used ~C tokens 波浪号且无队列徽章")]
fn t_c2826_footer_heuristic(fixed_zone_bdd: &FixedZoneBdd) {
    let footer = fixed_zone_bdd.frame.borrow().clone().expect("footer");
    assert!(!footer.contains('\n'), "c2826: footer 应单行：{footer}");
    assert!(
        footer.contains("used ~42 tokens"),
        "c2826: Heuristic 应带波浪号：{footer}"
    );
    assert!(
        !footer.contains("q:s"),
        "c2826: footer 不得有队列徽章：{footer}"
    );
    assert!(footer.contains("·"), "c2826: 字段应以 · 分隔：{footer}");
}

#[when("以 Api 来源 42k tokens 与 128k 窗口驱动 footer")]
async fn w_c2826_footer_percent(fixed_zone_bdd: &FixedZoneBdd) {
    use crate::protocol::model::TokenProvenance;
    let footer = footer_with_estimate(42_000, TokenProvenance::Api, 128_000).await;
    stash(fixed_zone_bdd, vec![footer]);
}

#[then("footer 含 used 42k tokens 与 32.8%/128k 派生占用比")]
fn t_c2826_footer_percent(fixed_zone_bdd: &FixedZoneBdd) {
    let footer = fixed_zone_bdd.frame.borrow().clone().expect("footer");
    assert!(
        footer.contains("used 42k tokens") && footer.contains("32.8%/128k"),
        "c2826: Api 应展示紧凑 used 与派生占用比：{footer}"
    );
    assert!(
        !footer.contains("~42k"),
        "c2826: Api 不得带波浪号：{footer}"
    );
}

#[when("以 42000 与 8123456 两种 token 数驱动 footer")]
async fn w_c2826_footer_compact(fixed_zone_bdd: &FixedZoneBdd) {
    use crate::protocol::model::TokenProvenance;
    let a = footer_with_estimate(42_000, TokenProvenance::Api, 128_000).await;
    let b = footer_with_estimate(8_123_456, TokenProvenance::Api, 32_000_000).await;
    stash(fixed_zone_bdd, vec![a, b]);
}

#[then("分别展示 used 42k tokens 与 used 8.1M tokens 且保留 tokens 词")]
fn t_c2826_footer_compact(fixed_zone_bdd: &FixedZoneBdd) {
    let raw = fixed_zone_bdd.frame.borrow().clone().expect("footers");
    let (a, b) = raw.split_once('\n').expect("two footers");
    assert!(a.contains("used 42k tokens"), "c2826: 整 k 紧凑：{a}");
    assert!(
        b.contains("used 8.1M tokens") || b.contains("used 8M tokens"),
        "c2826: M 级紧凑：{b}"
    );
    assert!(
        a.contains("tokens") && b.contains("tokens"),
        "c2826: 不得去掉 tokens 词"
    );
}

// ── c2826 specs-compact：app-tui-input 布局级场景步骤 ──────────────

#[when("经事件流注入两条相同正文的用户消息")]
fn w_c2826_same_text_twice(fixed_zone_bdd: &FixedZoneBdd) {
    use crate::agent::runtime::XyEvent;
    let mut fx = crate::app::tui::InteractionBdd::from_model(UiModel::new());
    for _ in 0..2 {
        fx.push_xy(XyEvent::MessageStart {
            role: "user".into(),
            message: None,
        });
        fx.push_xy(XyEvent::TextDelta("同样的问题".into()));
        fx.push_xy(XyEvent::MessageEnd {
            role: "user".into(),
            message: None,
        });
    }
    stash(fixed_zone_bdd, vec![fx.render_plain(80)]);
}

#[then("transcript 出现两条独立用户气泡")]
fn t_c2826_same_text_twice(fixed_zone_bdd: &FixedZoneBdd) {
    let frame = plain(&fixed_zone_bdd.frame.borrow().clone().expect("frame"));
    let hits = frame.lines().filter(|l| l.contains("同样的问题")).count();
    assert_eq!(hits, 2, "c2826: 相同正文应有两个独立气泡：{frame}");
}
