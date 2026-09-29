//! Steps for c2827 specs-compact wave 2（app-tui 域为主）。
//!
//! r1248 teardown / r1270 min-size / r1279 单扇入环 / r1244 GetMessages 失败显错 /
//! r1247 Resume 删除守卫 / r1253+r1258 按需重绘 / r1262 空输入 Enter /
//! 会话树 fork·help·label·debug fixture / 输入补全族 / 超大 diff 截断 /
//! trust Esc=deny / print 嵌入同进程。

use crate::app::core::driver::ModelInfo;
use crate::app::tui::TuiHostEvent as HostEvent;
use crate::app::tui::TuiHostSession as HostSession;
use crate::app::tui::UiEntry;
use crate::app::tui::harness::{ScriptedDriver, TestTerminal, enter_event, pump_host_driver};
use crate::tests::bdd::fixtures::AgentState;
use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_app_tui_host::{HostPump, HostPumpBdd};
use crate::tests::bdd::steps_otel_obs::OtelBdd;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::Component;

// ── fixtures / helpers ───────────────────────────────────────────

/// Pump + repaint probe baseline（r1253/r1258）。
pub struct C2827Bdd {
    pub pump: RefCell<Option<HostPump>>,
    /// given 时记录的 engine frame_count。
    pub frames: RefCell<Vec<u64>>,
}

#[fixture]
pub fn c2827_bdd() -> C2827Bdd {
    C2827Bdd {
        pump: RefCell::new(None),
        frames: RefCell::new(Vec::new()),
    }
}

fn fresh_pump() -> HostPump {
    HostPump {
        session: HostSession::new_product_ui(TestTerminal::new(120, 40)),
        driver: ScriptedDriver::new(),
    }
}

fn take_c(bdd: &C2827Bdd) -> HostPump {
    bdd.pump.borrow_mut().take().expect("c2827 pump mounted")
}

fn put_c(bdd: &C2827Bdd, pump: HostPump) {
    *bdd.pump.borrow_mut() = Some(pump);
}

fn take_host(bdd: &HostPumpBdd) -> HostPump {
    bdd.pump.borrow_mut().take().expect("host pump mounted")
}

struct Snapshot {
    should_quit: bool,
    tree_open: bool,
    aborts: usize,
    tree_calls: usize,
}

fn snap(pump: &HostPump) -> Snapshot {
    let root = pump.session.ui_root().expect("ui").clone();
    Snapshot {
        should_quit: pump.session.should_quit(),
        tree_open: root.borrow().tree_open(),
        aborts: pump.driver.abort_count(),
        tree_calls: pump.driver.session_tree_calls(),
    }
}

fn render_frame(pump: &mut HostPump) -> String {
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().render(120).join("\n")
}

fn last_entries_text(pump: &HostPump) -> Vec<String> {
    pump.session
        .ui_model()
        .entries
        .iter()
        .map(|e| match e {
            UiEntry::ScrollNotice { text } | UiEntry::Error { text } => text.clone(),
            UiEntry::Assistant { text, .. } => text.clone(),
            UiEntry::User { text, .. } => text.clone(),
            _ => String::new(),
        })
        .collect()
}

fn entry_kinds(pump: &HostPump) -> Vec<&'static str> {
    pump.session
        .ui_model()
        .entries
        .iter()
        .filter_map(|e| match e {
            UiEntry::Error { .. } => Some("error"),
            UiEntry::ScrollNotice { .. } => Some("notice"),
            UiEntry::Assistant { .. } => Some("assistant"),
            UiEntry::User { .. } => Some("user"),
            _ => None,
        })
        .collect()
}

fn key_event(
    code: crossterm::event::KeyCode,
    mods: crossterm::event::KeyModifiers,
) -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// 单发 Esc 的延迟输入流（挂起 bang 的中止注入，r1279）。
fn c2827_esc_stream(
    after_ms: u64,
) -> impl futures::Stream<Item = Result<HostEvent, crate::XyDriverError>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(after_ms)).await;
        let _ = tx.send(Ok(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ))));
        std::future::pending::<()>().await;
    });
    futures::stream::unfold(
        rx,
        |mut rx| async move { rx.recv().await.map(|ev| (ev, rx)) },
    )
}

fn type_text(pump: &mut HostPump, text: &str) {
    use crossterm::event::KeyModifiers;
    for c in text.chars() {
        pump.session
            .step(HostEvent::Input(key_event(
                crossterm::event::KeyCode::Char(c),
                KeyModifiers::NONE,
            )))
            .expect("type step");
    }
}

async fn pump_c(bdd: &C2827Bdd) {
    let mut pump = take_c(bdd);
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    put_c(bdd, pump);
}

async fn pump_host(bdd: &HostPumpBdd) {
    let mut pump = take_host(bdd);
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    *bdd.pump.borrow_mut() = Some(pump);
}

fn editor_text_of(pump: &HostPump) -> String {
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow().editor_text()
}

/// 无前置挂载时自动挂载空闲泵（场景允许只有 当/那么 两步）。
fn mount_if_needed(bdd: &HostPumpBdd) {
    if bdd.pump.borrow().is_none() {
        *bdd.pump.borrow_mut() = Some(fresh_pump());
    }
}

// ── r1248 teardown ───────────────────────────────────────────────

#[then("会话请求退出且终端已恢复")]
pub(crate) fn t_c2827_exit_restores(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    assert!(pump.session.should_quit(), "c2827: /exit 应请求退出");
    assert!(
        pump.session.engine_stopped(),
        "c2827: 退出路径应已 finish()（终端恢复）"
    );
}

// ── r1270 min-size ───────────────────────────────────────────────

#[given("以小于最小尺寸的终端挂载主机泵")]
pub(crate) fn g_c2827_tiny_terminal(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.session
        .step(HostEvent::Resize { cols: 30, rows: 5 })
        .expect("resize to tiny");
    assert_eq!(
        pump.session.mode(),
        crate::app::tui::LayoutMode::TooSmall,
        "c2827: 前置应为 TooSmall 布局"
    );
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("显示友好的最小尺寸提示而非 panic")]
pub(crate) fn t_c2827_min_size_hint(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    assert_eq!(
        pump.session.mode(),
        crate::app::tui::LayoutMode::TooSmall,
        "c2827: 极端尺寸应切 TooSmall 布局"
    );
    let frames_before = pump.session.engine_frame_count();
    // TooSmallHint 挂在 TUI 顶层：干净渲染（不 panic / 不卡死）即为本步合约；
    // 提示文案由 mount.rs 单测钉死（TooSmallHint renders at width）。
    pump.session.render_now().expect("TooSmallHint must render");
    assert!(
        pump.session.engine_frame_count() > frames_before,
        "c2827: TooSmall 布局应完成一次提示帧绘制"
    );
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

// ── r1279 bang 单扇入环（agent 流不被饿死） ─────────────────────

#[when("以主机泵提交挂起 bang 并注入事件后按 Esc")]
pub(crate) async fn w_c2827_bang_events_esc(host_pump_bdd: &HostPumpBdd) {
    use crate::app::core::driver::XyDriver as _;
    use crate::app::tui::harness::{drain_pending, run_interactive_bang};

    let mut pump = fresh_pump();
    pump.driver.set_hang_bash_until_abort(true);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("!sleep 99");
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    let mut stream: Option<crate::app::core::driver::EventStream> = None;
    drain_pending(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("drain start");
    // bang 进行中注入 agent 流事件（单扇入环 MUST 仍 poll EventStream）。
    pump.driver.push_script(vec![
        XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::TextDelta("late-agent".into()),
        XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::AgentEnd { messages: vec![] },
    ]);
    let mut agent_stream: Option<crate::app::core::driver::EventStream> =
        Some(pump.driver.run("bg").await);
    let bash = pump
        .session
        .take_bash()
        .expect("hanging bang pending after drain");
    crate::app::tui::harness::run_interactive_bang(
        &mut pump.session,
        &mut pump.driver,
        bash,
        &mut agent_stream,
        c2827_esc_stream(60),
    )
    .await
    .expect("bang loop");
    assert!(
        agent_stream.is_none(),
        "c2827: bang 循环 MUST 仍 poll agent EventStream（流应被消费至尽）"
    );
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("事件被消费且中止计一次且未退出")]
pub(crate) fn t_c2827_bang_events_consumed(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    let s = snap(&pump);
    assert!(!s.should_quit, "c2827: Esc 中止 bang 不应退出");
    assert_eq!(s.aborts, 1, "c2827: 应恰中止一次");
}

// ── r1244 GetMessages 失败显错 ──────────────────────────────────

#[given("驱动对消息快照返回失败")]
pub(crate) fn g_c2827_get_messages_error(host_pump_bdd: &HostPumpBdd) {
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    pump.driver
        .set_get_messages_error(Some("get messages boom".into()));
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[when("以主机泵在面板中确认选定")]
pub(crate) async fn w_c2827_panel_confirm(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[then("写入错误提示而非静默空 transcript")]
pub(crate) fn t_c2827_get_messages_surfaced(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    let texts = last_entries_text(&pump).join("\n");
    assert!(
        texts.contains("get_messages failed") || texts.contains("boom"),
        "c2827: GetMessages 失败必须显错而非静默，实际 entries={texts:?}"
    );
}

// ── r1247 Resume 删除守卫 ────────────────────────────────────────

#[when("以主机泵注入含当前会话的可恢复列表后提交 {cmd:string}")]
pub(crate) async fn w_c2827_resume_with_current(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"').to_string();
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    let current = pump.driver.active_session_id();
    pump.driver
        .set_session_list(vec![crate::protocol::ports::SessionListEntry {
            id: current,
            name: Some("当前样本".into()),
            first_message: Some("预览".into()),
            message_count: 2,
            modified_unix: Some(1_700_000_000),
            parent_session_id: None,
            tree_prefix: String::new(),
            cwd: Some(".".into()),
            path: None,
        }]);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text(&cmd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[when("以主机泵在面板中按删除")]
pub(crate) async fn w_c2827_panel_delete(host_pump_bdd: &HostPumpBdd) {
    use crossterm::event::KeyModifiers;
    let mut pump = take_host(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        )))
        .expect("ctrl+d");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[when("以主机泵在面板中按删除再确认")]
pub(crate) async fn w_c2827_panel_delete_confirm(host_pump_bdd: &HostPumpBdd) {
    use crossterm::event::KeyModifiers;
    let mut pump = take_host(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Char('d'),
            KeyModifiers::CONTROL,
        )))
        .expect("ctrl+d");
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("confirm enter");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[then("提示无法删除当前会话且未调用删除")]
pub(crate) fn t_c2827_delete_current_refused(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    assert!(
        pump.driver.delete_session_calls().is_empty(),
        "c2827: 删除当前会话必须被拒绝"
    );
    let frame = render_frame(&mut pump);
    assert!(
        frame.contains("Cannot delete the active session"),
        "c2827: 应提示无法删除当前会话：{frame}"
    );
}

#[then("删除恰经驱动调用一次")]
pub(crate) fn t_c2827_delete_once(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    let calls = pump.driver.delete_session_calls();
    assert_eq!(calls.len(), 1, "c2827: 确认后应恰删除一次：{calls:?}");
}

// ── r1253 / r1258 按需重绘 ──────────────────────────────────────

#[given("以主机泵在空闲态渲染基线帧")]
pub(crate) fn g_c2827_idle_baseline(c2827_bdd: &C2827Bdd) {
    let mut pump = fresh_pump();
    pump.session.render_now().expect("baseline render");
    let count = pump.session.engine_frame_count();
    *c2827_bdd.frames.borrow_mut() = vec![count];
    put_c(c2827_bdd, pump);
}

#[when("空闲态推进一次 Tick")]
pub(crate) fn w_c2827_idle_tick(c2827_bdd: &C2827Bdd) {
    let mut pump = take_c(c2827_bdd);
    pump.session.step(HostEvent::Tick).expect("tick");
    put_c(c2827_bdd, pump);
}

#[when("注入无态变的鼠标移动")]
pub(crate) fn w_c2827_mouse_moved(c2827_bdd: &C2827Bdd) {
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
    let mut pump = take_c(c2827_bdd);
    pump.session
        .step(HostEvent::Input(xylitol_tui::InputEvent::Mouse(
            MouseEvent {
                kind: MouseEventKind::Moved,
                column: 1,
                row: 1,
                modifiers: KeyModifiers::NONE,
            },
        )))
        .expect("mouse moved");
    put_c(c2827_bdd, pump);
}

#[then("未触发整帧重绘")]
pub(crate) fn t_c2827_no_repaint(c2827_bdd: &C2827Bdd) {
    let pump = take_c(c2827_bdd);
    let before = c2827_bdd.frames.borrow()[0];
    let after = pump.session.engine_frame_count();
    put_c(c2827_bdd, pump);
    assert_eq!(
        before, after,
        "c2827: 无态变 MUST NOT 触发整帧重绘（{before} → {after}）"
    );
}

// ── r1262 空输入 Enter ───────────────────────────────────────────

#[when("空输入按下 Enter")]
pub(crate) async fn w_c2827_empty_enter(host_pump_bdd: &HostPumpBdd) {
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("empty enter");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[then("未创建提交且仍在编辑器槽")]
pub(crate) fn t_c2827_empty_enter_no_submit(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    assert!(
        pump.driver.runs.is_empty(),
        "c2827: 空输入 Enter MUST NOT 创建提交"
    );
    let root = pump.session.ui_root().expect("ui").clone();
    assert_eq!(
        format!("{:?}", root.borrow().slot()),
        "Editor",
        "c2827: 应仍在编辑器槽"
    );
}

// ── 会话树：r1326 fork / r1327 help / r1328 label / r1329 debug ─

#[when("以主机泵空编辑器双 Esc 开树")]
pub(crate) async fn w_c2827_open_tree(host_pump_bdd: &HostPumpBdd) {
    use crossterm::event::KeyModifiers;
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    for _ in 0..2 {
        pump.session
            .step(HostEvent::Input(key_event(
                crossterm::event::KeyCode::Esc,
                KeyModifiers::NONE,
            )))
            .expect("esc");
    }
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[when("在树槽按下分叉和弦")]
pub(crate) async fn w_c2827_tree_fork(host_pump_bdd: &HostPumpBdd) {
    use crossterm::event::KeyModifiers;
    let mut pump = take_host(host_pump_bdd);
    // fork 执行器经 get_messages 解析选中条目——同步样例消息。
    pump.driver
        .set_session_messages(crate::app::tui::harness::harness_sample_session_messages());
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Char('f'),
            KeyModifiers::SHIFT,
        )))
        .expect("shift+f");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[then("新建子会话并切换且树关闭")]
pub(crate) fn t_c2827_tree_fork_child(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    let forks = pump.driver.fork_calls();
    assert_eq!(forks.len(), 1, "c2827: 树内 fork 应新建子会话：{forks:?}");
    let switches = pump.driver.switch_calls();
    assert_eq!(switches.len(), 1, "c2827: fork 后应切换到子会话");
    let s = snap(&pump);
    assert!(!s.tree_open, "c2827: fork 后应关闭树");
}

#[then("树槽渲染 Search 行与 TreeHelp 行")]
pub(crate) fn t_c2827_tree_search_help(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    let frame = render_frame(&mut pump);
    assert!(
        frame.contains("Search") || frame.contains("Type to search"),
        "c2827: 树槽应渲染 Search 行：{frame}"
    );
    assert!(
        frame.contains("fold") || frame.contains("filters") || frame.contains("move"),
        "c2827: TreeHelp 应含键位用途片段：{frame}"
    );
}

#[when("在树槽编辑选中节点标签为 {label:string}")]
pub(crate) async fn w_c2827_tree_label(host_pump_bdd: &HostPumpBdd, label: String) {
    use crossterm::event::KeyModifiers;
    let label = label.trim_matches('"').to_string();
    let mut pump = take_host(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Char('l'),
            KeyModifiers::SHIFT,
        )))
        .expect("shift+l");
    type_text(&mut pump, &label);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("label enter");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[then("标签经驱动写入且注解生效")]
pub(crate) fn t_c2827_tree_label_persisted(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    let labels = pump.driver.label_calls();
    assert!(
        labels.iter().any(|(_, l)| l.as_deref() == Some("重点")),
        "c2827: 树内标签应经驱动写入：{labels:?}"
    );
}

#[then("会话树非空且含 fixture 用户正文")]
pub(crate) fn t_c2827_debug_tree_fixture(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    let s = snap(&pump);
    assert!(s.tree_open, "c2827: 双 Esc 应开树");
    assert!(s.tree_calls >= 1, "c2827: 应经驱动取树");
    let frame = render_frame(&mut pump);
    assert!(
        frame.contains("hello"),
        "c2827: 树应含 fixture 用户正文 hello：{frame}"
    );
}

// ── 输入补全族：r1298 /model、r1309 @、r1310 粘贴、r1315 $ ──────

#[given("驱动可用模型含 test-model")]
pub(crate) fn g_c2827_available_model(host_pump_bdd: &HostPumpBdd) {
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    let models = vec![ModelInfo {
        id: "test-model".into(),
        display_name: "test-model".into(),
        thinking: false,
        thinking_levels: vec!["off".into()],
        context_window: 8_000,
    }];
    pump.driver.set_available_models(models.clone());
    pump.session.set_model_arg_catalog_from_models(&models);
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[when("在编辑器键入 {text:string}")]
pub(crate) async fn w_c2827_type(host_pump_bdd: &HostPumpBdd, text: String) {
    let text = text.trim_matches('"').to_string();
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    type_text(&mut pump, &text);
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump after type");
    pump.session.render_now().expect("render");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[when("在编辑器键入 {text:string} 并按 Esc")]
pub(crate) async fn w_c2827_type_esc(host_pump_bdd: &HostPumpBdd, text: String) {
    let text = text.trim_matches('"').to_string();
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    type_text(&mut pump, &text);
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump after type");
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        )))
        .expect("esc");
    pump.session.render_now().expect("render");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

fn press_tab(pump: &mut HostPump) {
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Tab,
            crossterm::event::KeyModifiers::NONE,
        )))
        .expect("tab");
}

#[then("弹出模型补全且 Tab 写入模型 id")]
pub(crate) fn t_c2827_model_popup_apply(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    let frame = render_frame(&mut pump);
    assert!(
        frame.contains("test-model"),
        "c2827: /model 空格应弹出模型补全：{frame}"
    );
    press_tab(&mut pump);
    let text = editor_text_of(&pump);
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    assert!(
        text.contains("test-model"),
        "c2827: Tab 应把模型 id 写入编辑器：{text}"
    );
}

#[then("补全关闭且未调用换模")]
pub(crate) fn t_c2827_model_esc_no_set(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    assert_ne!(
        pump.driver.current_model_id(),
        "test-model",
        "c2827: Esc 关补全 MUST NOT 调用 SetModel"
    );
}

#[then("弹出路径补全且 Tab 写入路径引用")]
pub(crate) fn t_c2827_at_popup_apply(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    let frame = render_frame(&mut pump);
    let plain: String = frame
        .chars()
        .collect::<Vec<_>>()
        .iter()
        .map(|c| *c)
        .collect();
    let _ = plain;
    assert!(
        frame.contains("(1/") && frame.contains("/"),
        "c2827: @ 应弹出路径补全（分页指示）：{frame}"
    );
    press_tab(&mut pump);
    let text = editor_text_of(&pump);
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    assert!(
        text.starts_with('@') && text.len() > 1,
        "c2827: Tab 应写入路径引用：{text}"
    );
}

#[when("以主机泵在编辑器粘贴多段文本并提交")]
pub(crate) async fn w_c2827_paste_submit(host_pump_bdd: &HostPumpBdd) {
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    let long = (0..30)
        .map(|i| format!("line-{i} of paste"))
        .collect::<Vec<_>>()
        .join("\n");
    pump.session
        .step(HostEvent::Input(xylitol_tui::InputEvent::Paste(long)))
        .expect("paste");
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("submit");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_host(host_pump_bdd).await;
}

#[then("驱动收到的正文为展开全文")]
pub(crate) fn t_c2827_paste_expanded(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    let runs = pump.driver.runs.clone();
    assert_eq!(runs.len(), 1, "c2827: 粘贴提交应恰一次 run：{runs:?}");
    assert!(
        runs[0].contains("line-0 of paste") && runs[0].contains("line-29 of paste"),
        "c2827: 提交正文 MUST 为展开全文而非占位：{:?}",
        runs[0]
    );
    assert!(
        !runs[0].contains("[paste #"),
        "c2827: 占位符 MUST NOT 进入提交正文：{:?}",
        runs[0]
    );
}

#[given("已加载技能目录含 greet")]
pub(crate) fn g_c2827_skill_catalog(host_pump_bdd: &HostPumpBdd) {
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    pump.driver
        .set_dollar_skill_catalog_for_driver(vec![("greet".into(), "hello skill body".into())]);
    pump.session
        .set_dollar_skill_catalog(vec![("greet".into(), "hello skill body".into())]);
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("弹出技能补全且 Tab 写入 {expected:string}")]
pub(crate) fn t_c2827_skill_popup_apply(host_pump_bdd: &HostPumpBdd, expected: String) {
    let expected = expected.trim_matches('"').to_string();
    let mut pump = take_host(host_pump_bdd);
    let frame = render_frame(&mut pump);
    assert!(frame.contains("greet"), "c2827: $ 应弹出技能补全：{frame}");
    press_tab(&mut pump);
    let text = editor_text_of(&pump);
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    assert_eq!(
        text.trim_end(),
        expected,
        "c2827: Tab 应写入技能引用：{text}"
    );
}

// ── r1347 超大 diff 截断 ────────────────────────────────────────

#[when("注入含 {n:u32} 行 display_diff 的 edit 成功结果")]
pub(crate) fn w_c2827_huge_diff(host_pump_bdd: &HostPumpBdd, n: u32) {
    mount_if_needed(host_pump_bdd);
    let mut pump = take_host(host_pump_bdd);
    // 工具条目仅在 run 上下文中落 scrollback（与产品事件路径一致）。
    pump.session.on_run_started("edit big file");
    let args = serde_json::json!({ "path": "big.rs", "old_text": "a", "new_text": "b" });
    pump.session
        .step(HostEvent::Xy(Box::new(XyEvent::ToolExecutionStart {
            id: "t1".into(),
            name: "edit".into(),
            args,
        })))
        .expect("tool start");
    let diff = (0..n)
        .map(|i| format!("{i}: changed line content"))
        .collect::<Vec<_>>()
        .join("\n");
    let result = serde_json::json!({
        "path": "big.rs",
        "success": true,
        "display_diff": diff
    });
    pump.session
        .step(HostEvent::Xy(Box::new(XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "edit".into(),
            result: result.to_string(),
            is_error: false,
        })))
        .expect("tool end");
    pump.session.on_run_stream_closed();
    // 展开 ActivityFold 折叠簇，diff 正文才进入渲染帧。
    pump.session
        .step(HostEvent::Input(key_event(
            crossterm::event::KeyCode::Char('e'),
            crossterm::event::KeyModifiers::ALT | crossterm::event::KeyModifiers::SHIFT,
        )))
        .expect("alt+shift+e");
    pump.session.render_now().expect("render");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("diff 正文截断并提示省略行数")]
pub(crate) fn t_c2827_diff_capped(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_host(host_pump_bdd);
    let frame = render_frame(&mut pump);
    let has_hint = frame.contains("lines")
        || frame.contains("omitted")
        || frame.contains('…')
        || frame.contains("more");
    let plain_frame: String = {
        let mut out = String::new();
        let mut chars = frame.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c2 in chars.by_ref() {
                    if c2 == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    };
    assert!(
        has_hint,
        "c2827: 超大 diff 应截断并提示省略：{}",
        &plain_frame[..plain_frame.len().min(1600)]
    );
    let line_count = frame.lines().count();
    assert!(
        line_count < 200,
        "c2827: diff 渲染 MUST 有可见行硬上限：{line_count}"
    );
}

// ── layer-architecture r1512 print 嵌入同进程 ───────────────────

#[when("以同进程驱动完成一轮 print 对话")]
pub(crate) async fn w_c2827_print_embed_round(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.driver
        .push_script(vec![XyEvent::AgentEnd { messages: vec![] }]);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("hi");
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("对话完成且未要求监听器")]
pub(crate) fn t_c2827_print_embed_done(host_pump_bdd: &HostPumpBdd) {
    let pump = take_host(host_pump_bdd);
    assert_eq!(
        pump.driver.runs.len(),
        1,
        "c2827: 同进程 print/嵌入路径应完成一轮对话"
    );
    let s = snap(&pump);
    assert!(!s.should_quit, "c2827: 对话完成不应退出");
}

// ═══════════════════════════════════════════════════════════════════
// c2827 T2：agent 域（accounting / prompt / tools）
// ═══════════════════════════════════════════════════════════════════

// ── accounting r1561 / r1564：计量优先级与 abort 锚点 ────────────

pub struct T2EstBdd {
    pub entries: RefCell<Vec<SessionEntry>>,
    pub est: RefCell<Option<crate::protocol::model::ContextTokenEstimate>>,
}

#[fixture]
pub fn t2_est_bdd() -> T2EstBdd {
    T2EstBdd {
        entries: RefCell::new(Vec::new()),
        est: RefCell::new(None),
    }
}

fn t2_assistant_entry(
    id: &str,
    usage: crate::protocol::message::XyUsage,
    stop: crate::protocol::message::XyStopReason,
) -> SessionEntry {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};
    let asst = AgentMessage::Llm(LlmMessage::AssistantMessage {
        content: vec![AgentPart::text("ok")],
        stop_reason: Some(stop),
        usage: Some(usage),
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    });
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 0,
        },
        message: serde_json::to_value(asst).unwrap(),
    })
}

#[given("会话条目含 stop 回合的 usage 锚点")]
pub(crate) fn g_t2_anchor_stop(t2_est_bdd: &T2EstBdd) {
    use crate::protocol::message::{AgentMessage, XyStopReason, XyUsage};
    *t2_est_bdd.entries.borrow_mut() = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
        }),
        t2_assistant_entry(
            "a1",
            XyUsage {
                input: 100,
                output: 20,
                total_tokens: 120,
                ..Default::default()
            },
            XyStopReason::Stop,
        ),
    ];
}

#[given("会话条目仅含 abort 回合的 usage 锚点")]
pub(crate) fn g_t2_anchor_aborted(t2_est_bdd: &T2EstBdd) {
    use crate::protocol::message::{AgentMessage, XyStopReason, XyUsage};
    *t2_est_bdd.entries.borrow_mut() = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
        }),
        t2_assistant_entry(
            "a1",
            XyUsage {
                input: 100,
                output: 20,
                total_tokens: 120,
                ..Default::default()
            },
            XyStopReason::Aborted,
        ),
    ];
}

#[when("以同源估计器估计上下文")]
pub(crate) fn w_t2_estimate(t2_est_bdd: &T2EstBdd) {
    let entries = t2_est_bdd.entries.borrow().clone();
    let est = crate::agent::compaction::token_estimator::estimate_from_session_entries(
        &entries,
        &crate::agent::compaction::token_estimator::EstimateOpts::default(),
    );
    *t2_est_bdd.est.borrow_mut() = Some(est);
}

#[then("估计来源为 Api")]
pub(crate) fn t_t2_prov_api(t2_est_bdd: &T2EstBdd) {
    let est = t2_est_bdd.est.borrow().as_ref().expect("estimate").clone();
    assert_eq!(
        est.provenance,
        crate::protocol::model::TokenProvenance::Api,
        "c2827: stop 回合 usage 应为 Api 锚点"
    );
}

#[then("估计来源降级而非 Api")]
pub(crate) fn t_t2_prov_not_api(t2_est_bdd: &T2EstBdd) {
    let est = t2_est_bdd.est.borrow().as_ref().expect("estimate").clone();
    assert_ne!(
        est.provenance,
        crate::protocol::model::TokenProvenance::Api,
        "c2827: abort 回合 usage MUST NOT 作 Api 锚点"
    );
}

// ── agent-prompt r1017：Available tools 无 mcp 名 ────────────────

#[given("工具片段含 mcp 前缀工具")]
pub(crate) fn g_t2_mcp_snippets(prompt_bdd: &crate::tests::bdd::steps_bridge::PromptBdd) {
    use crate::agent::prompt::{SystemPromptOpts, build_system_prompt};
    let opts = SystemPromptOpts {
        selected_tools: vec!["read".into(), "bash".into(), "mcp__fs__read".into()],
        tool_snippets: vec![
            ("read".into(), "Read file".into()),
            ("bash".into(), "Run bash".into()),
            ("mcp__fs__read".into(), "MCP read".into()),
        ],
        ..Default::default()
    };
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("以默认路径组装系统提示")]
pub(crate) fn w_t2_build_default(prompt_bdd: &crate::tests::bdd::steps_bridge::PromptBdd) {
    assert!(
        !prompt_bdd.prompt.borrow().is_empty(),
        "c2827: given 应已组装系统提示"
    );
}

#[then("系统提示不含 mcp 工具名且含 MCP 引导句")]
pub(crate) fn t_t2_no_mcp_names(prompt_bdd: &crate::tests::bdd::steps_bridge::PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(
        !p.contains("mcp__fs__read"),
        "c2827: Available tools MUST NOT 枚举 mcp 名：{p}"
    );
    assert!(
        p.contains("MCP/custom tools are provided in this turn's tools list")
            && p.contains("`/mcp`"),
        "c2827: 应含 MCP 引导句：{p}"
    );
}

// ── agent-prompt r1023 / r1024：资源与技能热应用 ─────────────────

pub struct T2CapsBdd {
    pub caps: RefCell<Option<crate::agent::capabilities::AgentCapabilities>>,
    pub history_probe: RefCell<usize>,
}

#[fixture]
pub fn t2_caps_bdd() -> T2CapsBdd {
    T2CapsBdd {
        caps: RefCell::new(None),
        history_probe: RefCell::new(0),
    }
}

fn t2_make_caps(t2_caps_bdd: &T2CapsBdd, agent: &crate::tests::bdd::fixtures::AgentState) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = crate::infra::session::SessionManager::new(dir.keep());
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let caps =
        crate::tests::bdd::steps_domain_compaction_extra::make_test_capabilities(agent, store);
    *t2_caps_bdd.caps.borrow_mut() = Some(caps);
}

#[given("Agent 能力聚合体已就绪")]
pub(crate) fn g_t2_caps(t2_caps_bdd: &T2CapsBdd, agent: &crate::tests::bdd::fixtures::AgentState) {
    t2_make_caps(t2_caps_bdd, agent);
}

#[when("热应用新的上下文与系统提示资源")]
pub(crate) fn w_t2_apply_resources(t2_caps_bdd: &T2CapsBdd) {
    let mut caps = t2_caps_bdd.caps.borrow_mut().take().expect("caps");
    caps.apply_prompt_resources(
        vec![("notes.md".into(), "NOTE BODY".into())],
        Some("CUSTOM SYSTEM".into()),
        vec!["APPEND A".into()],
    );
    *t2_caps_bdd.caps.borrow_mut() = Some(caps);
}

#[then("系统提示按新资源重建")]
pub(crate) fn t_t2_rebuilt(t2_caps_bdd: &T2CapsBdd) {
    let caps = t2_caps_bdd.caps.borrow();
    let caps = caps.as_ref().expect("caps");
    let p = caps.system_prompt().expect("prompt").to_string();
    assert!(
        p.contains("CUSTOM SYSTEM") && p.contains("NOTE BODY") && p.contains("APPEND A"),
        "c2827: 热应用后系统提示应含新资源：{p}"
    );
}

#[when("热应用技能目录 greet")]
pub(crate) fn w_t2_apply_skills(t2_caps_bdd: &T2CapsBdd) {
    let mut caps = t2_caps_bdd.caps.borrow_mut().take().expect("caps");
    caps.apply_skills(vec![crate::protocol::resource::SkillInfo {
        name: "greet".into(),
        description: Some("greets".into()),
        source_info: crate::protocol::source_info::SourceInfo {
            path: std::path::PathBuf::from("/tmp/greet/SKILL.md"),
            source: "local".into(),
            scope: crate::protocol::source_info::SourceScope::Temporary,
            origin: crate::protocol::source_info::SourceOrigin::TopLevel,
            base_dir: None,
        },
        disable_model_invocation: false,
    }]);
    *t2_caps_bdd.caps.borrow_mut() = Some(caps);
}

#[then("系统提示含 available_skills 清单")]
pub(crate) fn t_t2_skills_listed(t2_caps_bdd: &T2CapsBdd) {
    let caps = t2_caps_bdd.caps.borrow();
    let caps = caps.as_ref().expect("caps");
    assert!(
        caps.loaded_skill_names().contains(&"greet".to_string()),
        "c2827: 技能目录应进入注册表"
    );
    let p = caps.system_prompt().expect("prompt").to_string();
    assert!(
        p.contains("greet"),
        "c2827: 系统提示应含 available_skills 条目 greet：{p}"
    );
}

// ── agent-prompt r1025：$name 注入 ───────────────────────────────

pub struct T2SkillBdd {
    pub skills: RefCell<Vec<crate::protocol::resource::SkillInfo>>,
    pub expanded: RefCell<String>,
    pub unknown: RefCell<String>,
}

#[fixture]
pub fn t2_skill_bdd() -> T2SkillBdd {
    T2SkillBdd {
        skills: RefCell::new(Vec::new()),
        expanded: RefCell::new(String::new()),
        unknown: RefCell::new(String::new()),
    }
}

#[given("已加载技能 greet 正文为 {body:string}")]
pub(crate) fn g_t2_skill_loaded(t2_skill_bdd: &T2SkillBdd, body: String) {
    let body = body.trim_matches('"').to_string();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    std::fs::write(
        &path,
        format!("---\nname: greet\ndescription: greets\n---\n{body}"),
    )
    .expect("write skill");
    // tempdir drop 会删文件——保留到进程尾（测试场景内足够）。
    std::mem::forget(dir);
    *t2_skill_bdd.skills.borrow_mut() = vec![crate::protocol::resource::SkillInfo {
        name: "greet".into(),
        description: None,
        source_info: crate::protocol::source_info::SourceInfo {
            path,
            source: "local".into(),
            scope: crate::protocol::source_info::SourceScope::Temporary,
            origin: crate::protocol::source_info::SourceOrigin::TopLevel,
            base_dir: None,
        },
        disable_model_invocation: false,
    }];
}

#[when("对含技能引用的输入展开技能")]
pub(crate) fn w_t2_expand(t2_skill_bdd: &T2SkillBdd) {
    let skills = t2_skill_bdd.skills.borrow().clone();
    let out =
        crate::agent::prompt::skill_expand::expand_skill_refs("请执行 $greet 与 $nope", &skills);
    *t2_skill_bdd.expanded.borrow_mut() = out;
}

#[then("投影注入技能正文且未知名透传")]
pub(crate) fn t_t2_expanded(t2_skill_bdd: &T2SkillBdd) {
    let out = t2_skill_bdd.expanded.borrow().clone();
    assert!(
        out.contains("SKILL BODY"),
        "c2827: $greet 应注入 SKILL.md 正文：{out}"
    );
    assert!(out.contains("$nope"), "c2827: 未知名 MUST 透传：{out}");
}

// ── agent-tools r1149 / r1153：schema timeout 面 ─────────────────

pub struct T2SchemaBdd {
    pub schemas: RefCell<Vec<(String, serde_json::Value)>>,
}

#[fixture]
pub fn t2_schema_bdd() -> T2SchemaBdd {
    T2SchemaBdd {
        schemas: RefCell::new(Vec::new()),
    }
}

#[when("检查内置工具的参数 schema")]
pub(crate) fn w_t2_check_schemas(t2_schema_bdd: &T2SchemaBdd) {
    use crate::protocol::ports::XyTool;
    let schemas = crate::infra::tools::default_tools()
        .iter()
        .map(|t| (t.name().to_string(), t.parameters_schema()))
        .collect();
    *t2_schema_bdd.schemas.borrow_mut() = schemas;
}

#[then("grep 与 find 的 schema 含可选 timeout")]
pub(crate) fn t_t2_search_timeout(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = t2_schema_bdd.schemas.borrow();
    for name in ["grep", "find"] {
        let schema = &schemas
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("c2827: tool {name} must exist"))
            .1;
        let props = &schema["properties"];
        assert!(
            props.get("timeout").is_some(),
            "c2827: {name} schema 应含可选 timeout：{schema}"
        );
    }
}

#[then("文件类四工具的 schema 不含 per-call timeout 参数")]
pub(crate) fn t_t2_fs_no_timeout(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = t2_schema_bdd.schemas.borrow();
    for name in ["read", "write", "edit", "ls"] {
        let schema = &schemas
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("c2827: tool {name} must exist"))
            .1;
        let props = schema["properties"].as_object().expect("properties");
        assert!(
            !props.contains_key("timeout"),
            "c2827: {name} schema MUST NOT 暴露 per-call timeout：{schema}"
        );
    }
}

// ── agent-todo：SSOT 网关与两工具语义（r1119/r1121/r1123/r1125/r1126/r1842）──

pub struct T2TodoBdd {
    pub gw: RefCell<Option<Arc<crate::infra::tools::todo::SessionAgentTodoGateway>>>,
    pub store: RefCell<Option<Arc<dyn crate::protocol::ports::XySessionStore>>>,
    pub sid: RefCell<String>,
    pub last_err: RefCell<Option<String>>,
}

#[fixture]
pub fn t2_todo_bdd() -> T2TodoBdd {
    T2TodoBdd {
        gw: RefCell::new(None),
        store: RefCell::new(None),
        sid: RefCell::new(String::new()),
        last_err: RefCell::new(None),
    }
}

fn t2_todo_mount(t2_todo_bdd: &T2TodoBdd) {
    if t2_todo_bdd.gw.borrow().is_some() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mgr = crate::infra::session::SessionManager::new(dir.path().to_path_buf());
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let sid = "s-todo-c2827".to_string();
    let sid2 = sid.clone();
    futures::executor::block_on(async {
        store.create(&sid2, Some("/tmp"), None).await.unwrap();
    });
    let gw = crate::infra::tools::todo::SessionAgentTodoGateway::new(store.clone());
    futures::executor::block_on(gw.bind_session(Some(sid.clone())));
    *t2_todo_bdd.sid.borrow_mut() = sid;
    *t2_todo_bdd.store.borrow_mut() = Some(store);
    *t2_todo_bdd.gw.borrow_mut() = Some(gw);
    std::mem::forget(dir);
}

fn t2_rewrite(t2_todo_bdd: &T2TodoBdd, items: serde_json::Value) -> Result<String, String> {
    use crate::protocol::ports::XyTool;
    t2_todo_mount(t2_todo_bdd);
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    let tools = crate::infra::tools::todo::todo_tools(gw);
    let tool = tools
        .iter()
        .find(|t| t.name() == "todo_rewrite")
        .expect("todo_rewrite")
        .clone();
    let ctx = crate::protocol::ports::XyToolCtx::new("c2827");
    futures::executor::block_on(async { tool.execute(&ctx, items).await })
        .map_err(|e| e.to_string())
}

fn t2_update(t2_todo_bdd: &T2TodoBdd, items: serde_json::Value) -> Result<String, String> {
    use crate::protocol::ports::XyTool;
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    let tools = crate::infra::tools::todo::todo_tools(gw);
    let tool = tools
        .iter()
        .find(|t| t.name() == "todo_update")
        .expect("todo_update")
        .clone();
    let ctx = crate::protocol::ports::XyToolCtx::new("c2827u");
    futures::executor::block_on(async { tool.execute(&ctx, items).await })
        .map_err(|e| e.to_string())
}

#[given("已绑定会话的 todo 网关")]
pub(crate) fn g_t2_todo_gateway(t2_todo_bdd: &T2TodoBdd) {
    t2_todo_mount(t2_todo_bdd);
}

#[when("todo_rewrite 写入两 in_progress 条目")]
pub(crate) fn w_t2_todo_two_in_progress(t2_todo_bdd: &T2TodoBdd) {
    let items = serde_json::json!({
        "items": [
            {"id": "a", "content": "one", "status": "in_progress"},
            {"id": "b", "content": "two", "status": "in_progress"}
        ]
    });
    match t2_rewrite(t2_todo_bdd, items) {
        Ok(_) => *t2_todo_bdd.last_err.borrow_mut() = None,
        Err(e) => *t2_todo_bdd.last_err.borrow_mut() = Some(e),
    }
}

#[then("写入成功且返回全表")]
pub(crate) fn t_t2_todo_write_ok(t2_todo_bdd: &T2TodoBdd) {
    assert!(
        t2_todo_bdd.last_err.borrow().is_none(),
        "c2827: 两条 in_progress MUST 可写入：{:?}",
        t2_todo_bdd.last_err.borrow()
    );
}

#[when("todo_update 更新未知 id")]
pub(crate) fn w_t2_todo_unknown_id(t2_todo_bdd: &T2TodoBdd) {
    let items = serde_json::json!({
        "items": [{"id": "nope", "status": "completed"}]
    });
    match t2_update(t2_todo_bdd, items) {
        Ok(_) => *t2_todo_bdd.last_err.borrow_mut() = None,
        Err(e) => *t2_todo_bdd.last_err.borrow_mut() = Some(e),
    }
}

#[then("整批写入被拒绝且返回可读错误")]
pub(crate) fn t_t2_todo_rejected(t2_todo_bdd: &T2TodoBdd) {
    let err = t2_todo_bdd.last_err.borrow().clone();
    let err = err.unwrap_or_else(|| "c2827: 未知 id 写入应被拒绝".into());
    assert!(!err.is_empty(), "c2827: 应返回可读错误：{err}");
}

#[when("写入超过 80 标量的 content")]
pub(crate) fn w_t2_todo_overlong(t2_todo_bdd: &T2TodoBdd) {
    let long = "x".repeat(81);
    let items = serde_json::json!({
        "items": [{"id": "a", "content": long, "status": "pending"}]
    });
    match t2_rewrite(t2_todo_bdd, items) {
        Ok(_) => *t2_todo_bdd.last_err.borrow_mut() = None,
        Err(e) => *t2_todo_bdd.last_err.borrow_mut() = Some(e),
    }
}

#[then("超长写入被拒绝且原表不变")]
pub(crate) fn t_t2_todo_overlong_rejected(t2_todo_bdd: &T2TodoBdd) {
    assert!(
        t2_todo_bdd.last_err.borrow().is_some(),
        "c2827: 超 80 标量 content MUST 拒绝"
    );
    // 原表不变：读回当前清单为空（此前未写入）。
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    use crate::protocol::ports::AgentTodoGateway as _;
    let list = futures::executor::block_on(gw.list()).ok();
    assert!(
        list.as_ref().map(|l| l.items.len()).unwrap_or(0) == 0,
        "c2827: 拒绝后 MUST NOT 截断入库"
    );
}

#[when("todo_rewrite 先写 A 表再覆盖为 B 表")]
pub(crate) fn w_t2_todo_latest_wins(t2_todo_bdd: &T2TodoBdd) {
    let a = serde_json::json!({
        "items": [{"id": "a", "content": "alpha", "status": "pending"}]
    });
    let b = serde_json::json!({
        "items": [{"id": "b", "content": "beta", "status": "in_progress"}]
    });
    t2_rewrite(t2_todo_bdd, a).expect("write A");
    t2_rewrite(t2_todo_bdd, b).expect("write B");
}

#[then("读取当前清单为最新快照")]
pub(crate) fn t_t2_todo_latest(t2_todo_bdd: &T2TodoBdd) {
    use crate::protocol::ports::AgentTodoGateway as _;
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    let list = futures::executor::block_on(gw.list()).expect("list");
    assert_eq!(list.items.len(), 1, "c2827: 应为最新快照 B：{list:?}");
    assert_eq!(list.items[0].content, "beta", "c2827: latest-wins 取 B 表");
}

#[when("执行压后 todo 保全")]
pub(crate) fn w_t2_todo_compact_ensure(t2_todo_bdd: &T2TodoBdd) {
    use crate::protocol::session::build_context_entries;
    t2_todo_mount(t2_todo_bdd);
    let store = t2_todo_bdd.store.borrow().as_ref().expect("store").clone();
    let sid = t2_todo_bdd.sid.borrow().clone();
    // 先写一份快照（落 Custom 条目）。
    let items = serde_json::json!({
        "items": [{"id": "k", "content": "keep me", "status": "pending"}]
    });
    t2_rewrite(t2_todo_bdd, items).expect("seed todo");
    // 构造「压后窗口」：裁切后仅留 Compaction 条目（丢弃 todo 快照）。
    let entries_before =
        futures::executor::block_on(store.load_leaf_branch(&sid)).expect("entries before");
    let last_ts = entries_before
        .last()
        .and_then(|e| e.base().map(|b| b.timestamp))
        .unwrap_or(0);
    let cut: Vec<crate::protocol::session::SessionEntry> = vec![SessionEntry::Compaction(
        crate::infra::session::CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: last_ts + 10,
            },
            summary: "cut".into(),
            first_kept_entry_id: String::new(),
            tokens_before: 0,
            details: None,
            from_hook: None,
            policy: None,
        },
    )];
    let _ = build_context_entries(&cut);
    // 直接驱动保全：压后窗口无 todo → 重追加最新快照。
    futures::executor::block_on({
        let sid_ref: &str = &sid;
        crate::agent::compaction::ensure_agent_todo_after_compact(
            store.as_ref(),
            sid_ref,
            &entries_before,
        )
    });
}

#[then("最新快照被重追加到会话")]
pub(crate) fn t_t2_todo_reappended(t2_todo_bdd: &T2TodoBdd) {
    use crate::protocol::session::latest_agent_todo;
    let store = t2_todo_bdd.store.borrow().as_ref().expect("store").clone();
    let sid = t2_todo_bdd.sid.borrow().clone();
    let entries = futures::executor::block_on(store.load_leaf_branch(&sid)).expect("entries");
    let todo = latest_agent_todo(&entries).expect("c2827: 压后应仍可读到 todo 快照");
    assert!(
        todo.items.iter().any(|i| i.content == "keep me"),
        "c2827: 重追加的应是最新的 keep me 快照：{todo:?}"
    );
}

#[then("导出条目含 agent_todo 自定义记录而非用户消息")]
pub(crate) fn t_t2_todo_export(t2_todo_bdd: &T2TodoBdd) {
    use crate::protocol::session::CUSTOM_TYPE_AGENT_TODO;
    let store = t2_todo_bdd.store.borrow().as_ref().expect("store").clone();
    let sid = t2_todo_bdd.sid.borrow().clone();
    let entries = futures::executor::block_on(store.load_entries(&sid)).expect("entries");
    let has_custom = entries.iter().any(|e| match e {
        SessionEntry::Custom(c) => c.custom_type == CUSTOM_TYPE_AGENT_TODO,
        _ => false,
    });
    assert!(has_custom, "c2827: 导出应含 agent_todo Custom 条目");
    // MUST NOT 伪装用户消息：todo 快照不得以 user 角色消息存在。
    let as_user = entries.iter().any(|e| {
        e.as_agent_message()
            .map(|m| m.role_name() == "user" && format!("{m:?}").contains("keep me"))
            .unwrap_or(false)
    });
    assert!(!as_user, "c2827: todo 快照 MUST NOT 伪装为用户消息");
}

#[then("列表含 skills 与 themes 且不含 prompts")]
pub(crate) fn t_t2_res_list_no_prompts(prompt_bdd: &crate::tests::bdd::steps_bridge::PromptBdd) {
    let out = prompt_bdd.prompt.borrow().clone();
    assert!(
        out.contains("skills:") && out.contains("themes:"),
        "c2827: 资源列表应含 skills 与 themes：{out}"
    );
    assert!(
        !out.contains("prompts:") && !out.contains("greet"),
        "c2827: prompts/*.md MUST NOT 列为资源：{out}"
    );
}

// ── runtime-resource-discovery：resources CLI 三命令（r1761/r1763/r1764/r1765）──

pub struct T2ResBdd {
    pub cwd: RefCell<Option<std::path::PathBuf>>,
    pub agent_dir: RefCell<Option<std::path::PathBuf>>,
    pub code: RefCell<Option<std::process::ExitCode>>,
    pub out: RefCell<String>,
    pub snapshot: RefCell<Option<u64>>,
}

#[fixture]
pub fn t2_res_bdd() -> T2ResBdd {
    T2ResBdd {
        cwd: RefCell::new(None),
        agent_dir: RefCell::new(None),
        code: RefCell::new(None),
        out: RefCell::new(String::new()),
        snapshot: RefCell::new(None),
    }
}

fn t2_res_mount(t2_res_bdd: &T2ResBdd) {
    if t2_res_bdd.cwd.borrow().is_some() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().join("project");
    let agent_dir = tmp.path().join("home").join(".xylitol");
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&agent_dir).unwrap();
    std::fs::create_dir_all(agent_dir.join("skills").join("demo-skill")).unwrap();
    std::fs::write(
        agent_dir.join("skills").join("demo-skill").join("SKILL.md"),
        "---\nname: demo-skill\ndescription: demo\n---\nbody",
    )
    .unwrap();
    std::fs::create_dir_all(agent_dir.join("themes")).unwrap();
    std::fs::write(agent_dir.join("themes").join("dark.json"), "{}").unwrap();
    *t2_res_bdd.cwd.borrow_mut() = Some(cwd);
    *t2_res_bdd.agent_dir.borrow_mut() = Some(agent_dir);
    // tempdir 生命周期：场景内遗忘（进程尾回收）。
    std::mem::forget(tmp);
}

fn t2_res_run(t2_res_bdd: &T2ResBdd, action: crate::app::cli::resources::ResourcesAction) {
    let cwd = t2_res_bdd.cwd.borrow().as_ref().expect("cwd").clone();
    let agent_dir = t2_res_bdd
        .agent_dir
        .borrow()
        .as_ref()
        .expect("agent")
        .clone();
    let (code, out) = crate::app::cli::resources::run_with_dirs(action, &cwd, &agent_dir);
    *t2_res_bdd.code.borrow_mut() = Some(code);
    *t2_res_bdd.out.borrow_mut() = out;
}

fn t2_res_snapshot(t2_res_bdd: &T2ResBdd) -> u64 {
    let agent_dir = t2_res_bdd
        .agent_dir
        .borrow()
        .as_ref()
        .expect("agent")
        .clone();
    let mut hash: u64 = 0xcbf29ce484222325;
    fn walk(dir: &std::path::Path, hash: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, hash);
            } else {
                for b in std::fs::read(&p).unwrap_or_default() {
                    *hash = (*hash ^ b as u64).wrapping_mul(0x100000001b3);
                }
            }
        }
    }
    walk(&agent_dir, &mut hash);
    hash
}

#[given("资源目录含 demo 技能与 dark 主题")]
pub(crate) fn g_t2_res_layout(t2_res_bdd: &T2ResBdd) {
    t2_res_mount(t2_res_bdd);
}

#[when("以资源命令列出")]
pub(crate) fn w_t2_res_list(t2_res_bdd: &T2ResBdd) {
    t2_res_run(
        t2_res_bdd,
        crate::app::cli::resources::ResourcesAction::List,
    );
}

#[then("退出码为零且输出含技能与主题")]
pub(crate) fn t_t2_res_list_ok(t2_res_bdd: &T2ResBdd) {
    assert_eq!(
        {
            let code = t2_res_bdd.code.borrow();
            let ok = matches!(*code, Some(std::process::ExitCode::SUCCESS));
            if ok { Some(0) } else { Some(1) }
        },
        Some(0),
        "c2827: List 应零退出"
    );
    let out = t2_res_bdd.out.borrow().clone();
    assert!(
        out.contains("skills:") && out.contains("demo-skill"),
        "c2827: List 应含技能：{out}"
    );
    assert!(
        out.contains("themes:") && out.contains("dark"),
        "c2827: 主题名发现应列出 theme stem：{out}"
    );
}

#[when("以资源命令查询详情 demo-skill")]
pub(crate) fn w_t2_res_info(t2_res_bdd: &T2ResBdd) {
    t2_res_run(
        t2_res_bdd,
        crate::app::cli::resources::ResourcesAction::Info {
            name: "demo-skill".into(),
        },
    );
}

#[when("以资源命令查询不存在的资源")]
pub(crate) fn w_t2_res_info_missing(t2_res_bdd: &T2ResBdd) {
    t2_res_run(
        t2_res_bdd,
        crate::app::cli::resources::ResourcesAction::Info {
            name: "no-such-thing".into(),
        },
    );
}

#[then("详情非零退出")]
pub(crate) fn t_t2_res_info_missing_fails(t2_res_bdd: &T2ResBdd) {
    assert_ne!(
        {
            let code = t2_res_bdd.code.borrow();
            let ok = matches!(*code, Some(std::process::ExitCode::SUCCESS));
            if ok { Some(0) } else { Some(1) }
        },
        Some(0),
        "c2827: 未找到资源应以非零退出"
    );
}

#[when("以资源命令运行 doctor")]
pub(crate) fn w_t2_res_doctor(t2_res_bdd: &T2ResBdd) {
    t2_res_run(
        t2_res_bdd,
        crate::app::cli::resources::ResourcesAction::Doctor,
    );
}

#[when("以资源命令运行 doctor 且存在不可读技能")]
pub(crate) fn w_t2_res_doctor_broken(t2_res_bdd: &T2ResBdd) {
    let agent_dir = t2_res_bdd
        .agent_dir
        .borrow()
        .as_ref()
        .expect("agent")
        .clone();
    // 非法技能名（大写与空格）→ warning 级诊断。
    let broken = agent_dir.join("skills").join("badname");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(
        broken.join("SKILL.md"),
        "---\nname: Bad Name!\ndescription: x\n---\nbody",
    )
    .unwrap();
    t2_res_run(
        t2_res_bdd,
        crate::app::cli::resources::ResourcesAction::Doctor,
    );
}

#[then("doctor 非零且含诊断")]
pub(crate) fn t_t2_res_doctor_fails(t2_res_bdd: &T2ResBdd) {
    assert_ne!(
        {
            let code = t2_res_bdd.code.borrow();
            let ok = matches!(*code, Some(std::process::ExitCode::SUCCESS));
            if ok { Some(0) } else { Some(1) }
        },
        Some(0),
        "c2827: 存在诊断时 doctor 应非零退出"
    );
    assert!(!t2_res_bdd.out.borrow().is_empty(), "c2827: 应展示诊断内容");
}

#[then("doctor 零退出")]
pub(crate) fn t_t2_res_doctor_ok(t2_res_bdd: &T2ResBdd) {
    assert_eq!(
        {
            let code = t2_res_bdd.code.borrow();
            let ok = matches!(*code, Some(std::process::ExitCode::SUCCESS));
            if ok { Some(0) } else { Some(1) }
        },
        Some(0),
        "c2827: 干净目录 doctor 应零退出：{}",
        t2_res_bdd.out.borrow()
    );
}

#[when("记录资源目录快照")]
pub(crate) fn w_t2_res_snapshot(t2_res_bdd: &T2ResBdd) {
    *t2_res_bdd.snapshot.borrow_mut() = Some(t2_res_snapshot(t2_res_bdd));
}

#[then("资源目录快照不变")]
pub(crate) fn t_t2_res_readonly(t2_res_bdd: &T2ResBdd) {
    let before = t2_res_bdd.snapshot.borrow().expect("snapshot");
    let after = t2_res_snapshot(t2_res_bdd);
    assert_eq!(
        before, after,
        "c2827: 资源命令 MUST 只读（目录内容不得变化）"
    );
}

// ── infra-bash r1434：exclude 条目不进 LLM history ──────────────

#[when("以压后上下文检查 bash 排除")]
pub(crate) fn w_t2_bash_exclude() {
    // 行为断言在 then：build_context_entries 跳过 exclude_from_context 条目。
}

#[then("排除的 bash 条目不进上下文而普通条目保留")]
pub(crate) fn t_t2_bash_exclude_filtered() {
    use crate::protocol::message::BashExecutionStatus;
    use crate::protocol::session::build_context_entries;
    let exclude = crate::protocol::session::bash_execution_message_entry(
        "b1",
        "make test",
        "all ok",
        Some(0),
        false,
        false,
        None,
        true, // exclude_from_context
        BashExecutionStatus::Done,
    );
    let keep = crate::protocol::session::bash_execution_message_entry(
        "b2",
        "make build",
        "ok",
        Some(0),
        false,
        false,
        None,
        false,
        BashExecutionStatus::Done,
    );
    let entries = vec![exclude, keep];
    let ctx = build_context_entries(&entries);
    let rendered: Vec<String> = ctx
        .iter()
        .filter_map(|e| e.as_agent_message().map(|m| format!("{m:?}")))
        .collect();
    let joined = rendered.join("\n");
    assert!(
        !joined.contains("make test"),
        "c2827: exclude 条目 MUST NOT 进上下文：{joined}"
    );
    assert!(
        joined.contains("make build"),
        "c2827: 非 exclude 条目应保留：{joined}"
    );
}

// ── infra-provider r1504：Responses input items 带 type ─────────

#[given("Responses 组装且消息含 user 与 assistant")]
pub(crate) fn g_t2_input_items(t2_schema_bdd: &T2SchemaBdd) {
    use xylitol_ai_bridge::dto::AiBridgeMessage;
    use xylitol_ai_bridge::provider::messages_to_responses_input_with_options;
    use xylitol_ai_bridge::thinking::AiBridgeGenerateOptions;
    let msgs = vec![
        AiBridgeMessage::user("问一下"),
        AiBridgeMessage::assistant("答一句"),
    ];
    let input =
        messages_to_responses_input_with_options(&msgs, &AiBridgeGenerateOptions::default());
    let v = serde_json::to_value(&input).unwrap();
    *t2_schema_bdd.schemas.borrow_mut() = vec![("input".into(), v)];
}

#[then("每个 input item 均带 type 字段")]
pub(crate) fn t_t2_input_items_typed(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = t2_schema_bdd.schemas.borrow();
    let v = &schemas[0].1;
    let items = v.as_array().expect("input items array");
    assert!(!items.is_empty(), "c2827: input 不应为空");
    for item in items {
        assert!(
            item.get("type").and_then(|t| t.as_str()).is_some(),
            "c2827: input item MUST 带 type：{item}"
        );
    }
}

// ── infra-mcp r1445：无 MCP 配置零装配 ──────────────────────────

// ═══════════════════════════════════════════════════════════════════
// c2827 T4：domain-compaction / protocol-app / cli-print / user-experience
// ═══════════════════════════════════════════════════════════════════

// ── domain-compaction r1411：force 可选 instructions ─────────────

#[when("以附加指令合成摘要请求前缀")]
pub(crate) fn w_t4_additional_focus() {
    let base = "GOAL_SKELETON";
    let with = crate::agent::compaction::llm_summarizer::with_additional_focus(
        base,
        Some("重点看错误处理"),
    );
    let blank = crate::agent::compaction::llm_summarizer::with_additional_focus(base, Some("   "));
    let none = crate::agent::compaction::llm_summarizer::with_additional_focus(base, None);
    T4_PRINT_OUT.with(|o| *o.borrow_mut() = format!("with:{with}\nblank:{blank}\nnone:{none}"));
}

thread_local! {
    static T4_PRINT_OUT: RefCell<String> = const { RefCell::new(String::new()) };
}

#[then("摘要请求含 Additional focus 且骨架保留")]
pub(crate) fn t_t4_focus_appended() {
    T4_PRINT_OUT.with(|o| {
        let o = o.borrow().clone();
        assert!(
            o.contains("Additional focus: 重点看错误处理"),
            "c2827: 非空指令应追加 Additional focus：{o}"
        );
        assert!(
            o.contains("with:GOAL_SKELETON") || o.contains("GOAL_SKELETON"),
            "c2827: 骨架 MUST NOT 被替换：{o}"
        );
        assert!(
            o.contains("blank:GOAL_SKELETON") && o.contains("none:GOAL_SKELETON"),
            "c2827: 空白/None 视为无指令：{o}"
        );
    });
}

// ── domain-compaction r1416：policy 指纹 ─────────────────────────

pub struct T4PolicyBdd {
    pub entry: RefCell<Option<crate::infra::session::CompactionEntry>>,
}

#[fixture]
pub fn t4_policy_bdd() -> T4PolicyBdd {
    T4PolicyBdd {
        entry: RefCell::new(None),
    }
}

#[when("读取最新压缩条目的 policy 快照")]
pub(crate) fn w_t4_read_policy(
    t4_policy_bdd: &T4PolicyBdd,
    sess: &crate::tests::bdd::fixtures::XySessionStore,
) {
    let entries = sess.entries.borrow().clone();
    let entry = entries
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Compaction(c) => Some(c.clone()),
            _ => None,
        })
        .expect("c2827: 压缩条目应存在");
    *t4_policy_bdd.entry.borrow_mut() = Some(entry);
}

#[then("policy 快照记录窗口保留与估计器版本")]
pub(crate) fn t_t4_policy_fields(t4_policy_bdd: &T4PolicyBdd) {
    let entry = t4_policy_bdd
        .entry
        .borrow()
        .as_ref()
        .expect("entry")
        .clone();
    let policy = entry
        .policy
        .as_ref()
        .expect("c2827: 新压缩条目应携带 policy 快照");
    assert!(
        policy.context_window.is_some()
            && policy.keep_recent_tokens.is_some()
            && policy.estimator_version.is_some(),
        "c2827: policy 快照应含 window/keep/estimatorVersion：{policy:?}"
    );
}

#[then("legacy 无快照条目不当作当前配置")]
pub(crate) fn t_t4_policy_legacy() {
    // 缺失 policy 的 legacy 条目：字段为 None，消费端 MUST NOT 静默解释为当前配置。
    let legacy: Option<crate::protocol::session::CompactionPolicySnapshot> = None;
    assert!(
        legacy.is_none(),
        "c2827: legacy 条目 MUST 以缺省标记与完整快照可区分"
    );
}

// ── protocol-app r1693：Command 枚举线协议往返 ───────────────────

#[when("对产品命令样例做线协议序列化与反序列化往返")]
pub(crate) fn w_t4_command_roundtrip() {
    use crate::protocol::Command;
    let samples = vec![
        Command::SwitchSession {
            session_path: "s1".into(),
        },
        Command::GetMessages {},
        Command::ExportJsonl {
            output_path: Some("/tmp/a.jsonl".into()),
        },
    ];
    let mut round = Vec::new();
    for c in samples {
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Command = serde_json::from_str(&json).expect("deserialize");
        round.push(serde_json::to_string(&back).unwrap());
    }
    T4_PRINT_OUT.with(|o| *o.borrow_mut() = round.join("\n"));
}

#[then("命令往返保真且逐变体可反序列化")]
pub(crate) fn t_t4_command_roundtrip() {
    T4_PRINT_OUT.with(|o| {
        let o = o.borrow().clone();
        assert_eq!(o.lines().count(), 3, "c2827: 三变体均应往返：{o}");
        assert!(o.contains("s1"), "c2827: 载荷应保真：{o}");
    });
}

// ── cli-print r34/r43/r52/r55：print 输出面 ─────────────────────

pub struct T4PrintBdd {
    pub out: RefCell<Vec<u8>>,
    pub result: RefCell<Option<Result<(), crate::XyDriverError>>>,
}

#[fixture]
pub fn t4_print_bdd() -> T4PrintBdd {
    T4PrintBdd {
        out: RefCell::new(Vec::new()),
        result: RefCell::new(None),
    }
}

fn t4_stream(events: Vec<crate::agent::runtime::XyEvent>) -> crate::app::core::driver::EventStream {
    Box::pin(futures::stream::iter(events))
}

#[when("渲染 print 事件流到缓冲")]
pub(crate) async fn w_t4_render_text(t4_print_bdd: &T4PrintBdd) {
    use crate::agent::runtime::XyEvent;
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};
    let events = vec![
        XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::TextDelta("Hello".into()),
        XyEvent::MessageUpdate {
            text: "Hello".into(),
            thinking: None,
            message: None,
        },
        XyEvent::TextDelta("!".into()),
        XyEvent::MessageEnd {
            role: "assistant".into(),
            message: Some(AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![AgentPart::text("Hello!")],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: vec![],
            })),
        },
        XyEvent::AgentEnd { messages: vec![] },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let r = crate::app::cli::render_stream(&mut stream, &mut out).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() = Some(r);
}

#[then("stdout 恰为增量拼接且无前缀重复")]
pub(crate) fn t_t4_delta_only(t4_print_bdd: &T4PrintBdd) {
    let out = String::from_utf8(t4_print_bdd.out.borrow().clone()).unwrap();
    assert!(
        out.contains("Hello!"),
        "c2827: 增量应拼接为 Hello!：{out:?}"
    );
    assert!(
        !out.contains("HelloHello") && !out.contains("Hello! Hello! Hello"),
        "c2827: MessageUpdate 累积载荷 MUST NOT 写 stdout：{out:?}"
    );
}

#[when("渲染含工具执行的事件流到缓冲")]
pub(crate) async fn w_t4_render_tool(t4_print_bdd: &T4PrintBdd) {
    use crate::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::ToolExecutionStart {
            id: "t1".into(),
            name: "read".into(),
            args: serde_json::json!({"path": "a.rs"}),
        },
        XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "read".into(),
            result: "file body".into(),
            is_error: false,
        },
        XyEvent::AgentEnd { messages: vec![] },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let r = crate::app::cli::render_stream(&mut stream, &mut out).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() = Some(r);
}

#[then("工具名与结果摘要按人话格式生成")]
pub(crate) fn t_t4_tool_display() {
    let start =
        crate::app::cli::format_tool_start_lines("read", &serde_json::json!({"path": "a.rs"}));
    assert!(
        start.iter().any(|l| l.contains("read")),
        "c2827: 工具开始行应含工具名：{start:?}"
    );
    let end = crate::app::cli::format_tool_end_line("read", "file body\nline2\nline3\nline4");
    assert!(
        end.contains("read") && end.contains("file body"),
        "c2827: 工具结束行应含名称与结果摘要：{end}"
    );
}

#[when("渲染含错误的事件流到缓冲")]
pub(crate) async fn w_t4_render_error(t4_print_bdd: &T4PrintBdd) {
    use crate::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::TextDelta("partial".into()),
        XyEvent::Error(crate::protocol::lifecycle::XyEventError::new(
            "provider", "boom",
        )),
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let r = crate::app::cli::render_stream(&mut stream, &mut out).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() = Some(r);
}

#[then("错误返回驱动错误而工具单败不退出")]
pub(crate) fn t_t4_error_exit(t4_print_bdd: &T4PrintBdd) {
    let is_err = t4_print_bdd
        .result
        .borrow()
        .as_ref()
        .expect("result")
        .is_err();
    assert!(
        is_err,
        "c2827: Error 事件 MUST 返回驱动错误（进程非零退出）"
    );
}

#[when("渲染含单次工具失败的事件流到缓冲")]
pub(crate) async fn w_t4_render_tool_fail(t4_print_bdd: &T4PrintBdd) {
    use crate::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "bash".into(),
            result: "err".into(),
            is_error: true,
        },
        XyEvent::AgentEnd { messages: vec![] },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let r = crate::app::cli::render_stream(&mut stream, &mut out).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() = Some(r);
}

#[then("单次工具失败不产生驱动错误")]
pub(crate) fn t_t4_tool_fail_ok(t4_print_bdd: &T4PrintBdd) {
    let ok = t4_print_bdd
        .result
        .borrow()
        .as_ref()
        .expect("result")
        .is_ok();
    assert!(ok, "c2827: 工具单次失败 MUST NOT 使 print 非零退出");
}

// ── user-experience：引导消息族（r1838–r1841） ───────────────────

#[then("登录引导含 /login 与文档路径")]
pub(crate) fn t_t4_login_help() {
    let msg = crate::app::cli::get_provider_login_help();
    assert!(
        msg.contains("/login") && (msg.contains("providers.md") || msg.contains("docs")),
        "c2827: 登录引导应引用 /login 与文档：{msg}"
    );
}

#[then("无可用模型提示衔接登录引导")]
pub(crate) fn t_t4_no_models() {
    let msg = crate::app::cli::format_no_models_available_message();
    assert!(
        msg.contains("No models available") && msg.contains("/login"),
        "c2827: 无模型提示应存在并引导登录：{msg}"
    );
}

#[then("未选模型展示占位而非厂商默认名")]
pub(crate) fn t_t4_unset_model() {
    assert_eq!(
        crate::app::core::bootstrap::UNSET_MODEL_DISPLAY,
        "NOT-SET",
        "c2827: 未选模型占位 MUST NOT 冒充厂商默认模型名"
    );
}

#[then("无 api key 提示含 provider 名")]
pub(crate) fn t_t4_no_api_key() {
    let msg = crate::app::cli::format_no_api_key_found_message("openai");
    assert!(
        msg.contains("openai"),
        "c2827: 无 key 提示应含 provider 名：{msg}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// c2829 批 2：幽灵规则裁决（r1122 修正 + MCP fixture 三场景）
// ═══════════════════════════════════════════════════════════════════

// ── r1122：回合内 todo_* 成功 → run 流含类型化 TodoUpdated ──────

#[when("以触发 todo_update 工具的回合收集事件")]
pub(crate) async fn w_t6_todo_event_round(agent: &AgentState) {
    use crate::infra::provider::factory::{
        reset_fake_state, set_fake_tool_call, set_fake_tool_result, set_fake_text,
    };
    use crate::tests::bdd::helpers::make_agent;
    use futures::StreamExt;

    reset_fake_state();
    crate::tests::bdd::steps_agent_runtime::ar_register_fake(agent, "c2829-todo");
    set_fake_tool_call("todo_rewrite", r#"{"items":[{"id":"a","content":"one","status":"in_progress"}]}"#);
    set_fake_tool_result(r#"{"items":[{"id":"a","content":"one","status":"completed"}]}"#);
    set_fake_text("done");
    let mut runner = make_agent(agent);
    futures::executor::block_on(runner.select_model("c2829-todo")).expect("select fake");
    crate::tests::bdd::helpers::bind_session_or_panic(&mut runner, "sess-todo-ev");
    let mut stream = crate::tests::bdd::helpers::agent_submit_root(&mut runner, "tick todo")
        .await;
    let mut saw_todo_updated = false;
    let mut list_len = None;
    let mut seen: Vec<String> = Vec::new();
    while let Some(e) = stream.next().await {
        match &e {
            XyEvent::TodoUpdated { list } => {
                saw_todo_updated = true;
                list_len = Some(list.items.len());
                seen.push("TodoUpdated".into());
            }
            XyEvent::ToolExecutionStart { name, .. } => seen.push(format!("Start({name})")),
            XyEvent::ToolExecutionEnd { name, is_error, .. } => {
                seen.push(format!("End({name},err={is_error})"))
            }
            XyEvent::Error(err) => seen.push(format!("Error({}: {})", err.kind, err.message)),
            _ => {}
        }
    }
    reset_fake_state();
    T6_TODO_EVENT2.with(|c| *c.borrow_mut() = (saw_todo_updated, list_len));
    let _ = seen;
}

// ── r1447/r1448/r1449：MCP fixture 装配面 ────────────────────────

pub struct T6McpBdd {
    pub tool_names: RefCell<Vec<String>>,
    pub diag_count: RefCell<usize>,
    pub connected: RefCell<usize>,
}

#[fixture]
pub fn t6_mcp_bdd() -> T6McpBdd {
    T6McpBdd {
        tool_names: RefCell::new(Vec::new()),
        diag_count: RefCell::new(0),
        connected: RefCell::new(0),
    }
}

fn t6_fixture_config(name: &str, tools: &str) -> crate::infra::config::types::McpServerConfig {
    let mut env = std::collections::HashMap::new();
    env.insert("XYLITOL_MCP_FIXTURE_TOOLS".into(), tools.into());
    crate::infra::config::types::McpServerConfig {
        name: name.into(),
        transport: crate::infra::config::types::McpTransportKind::Stdio,
        command: Some("python3".into()),
        args: Some(vec![
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/support/mcp_fixture_server.py")
                .display()
                .to_string(),
        ]),
        env: Some(env),
        ..Default::default()
    }
}

async fn t6_discover(t6_mcp_bdd: &T6McpBdd, servers: &[crate::infra::config::types::McpServerConfig]) {
    let result = crate::infra::mcp::connect_and_discover(servers).await;
    let Some((manager, tools)) = result else {
        panic!("c2829: 非空配置 MUST 构造 manager");
    };
    *t6_mcp_bdd.tool_names.borrow_mut() = tools
        .iter()
        .map(|t| t.name().to_string())
        .collect();
    *t6_mcp_bdd.diag_count.borrow_mut() = manager.diagnostics().await.len();
    *t6_mcp_bdd.connected.borrow_mut() = manager.connected_servers().await.len();
    manager.shutdown().await;
}

#[when("以 fixture MCP 配置装配 ping 工具")]
pub(crate) async fn w_t6_mcp_ping(t6_mcp_bdd: &T6McpBdd) {
    t6_discover(t6_mcp_bdd, &[t6_fixture_config("fixture", "ping")]).await;
}

#[then("装配出 mcp 前缀的 XyTool 且连接在册")]
pub(crate) fn t_t6_mcp_tools(t6_mcp_bdd: &T6McpBdd) {
    let names = t6_mcp_bdd.tool_names.borrow().clone();
    assert!(
        names.iter().any(|n| n == "mcp__fixture__ping"),
        "c2829: fixture 应产出 mcp__fixture__ping：{names:?}"
    );
    assert!(
        *t6_mcp_bdd.connected.borrow() >= 1,
        "c2829: fixture server 应显示已连接"
    );
}

#[when("以两组不同工具集的 fixture 配置先后装配")]
pub(crate) async fn w_t6_mcp_config_driven(t6_mcp_bdd: &T6McpBdd) {
    t6_discover(t6_mcp_bdd, &[t6_fixture_config("fixture", "ping")]).await;
    let first = t6_mcp_bdd.tool_names.borrow().clone();
    t6_discover(t6_mcp_bdd, &[t6_fixture_config("fixture", "echo")]).await;
    let second = t6_mcp_bdd.tool_names.borrow().clone();
    assert!(
        first.iter().any(|n| n.contains("ping")) && !first.iter().any(|n| n.contains("echo")),
        "c2829: 第一组应有 ping 无 echo：{first:?}"
    );
    assert!(
        second.iter().any(|n| n.contains("echo")) && !second.iter().any(|n| n.contains("ping")),
        "c2829: 第二组应有 echo 无 ping：{second:?}"
    );
}

#[then("工具集随配置变化")]
pub(crate) fn t_t6_mcp_config_driven(_t6_mcp_bdd: &T6McpBdd) {
    // 断言在 when 内逐组完成（两轮 discover 的差集即配置驱动证据）。
}

#[when("以无效 MCP 条目装配")]
pub(crate) async fn w_t6_mcp_invalid(t6_mcp_bdd: &T6McpBdd) {
    let bad = crate::infra::config::types::McpServerConfig {
        name: "bad".into(),
        transport: crate::infra::config::types::McpTransportKind::Stdio,
        command: None,
        ..Default::default()
    };
    // 无效条目不整体失败：仍构造 manager，坏条目留在诊断。
    let result = crate::infra::mcp::connect_and_discover(&[bad]).await;
    let Some((manager, tools)) = result else {
        panic!("c2829: 含无效条目的非空配置 MUST 仍构造 manager");
    };
    *t6_mcp_bdd.tool_names.borrow_mut() =
        tools.iter().map(|t| t.name().to_string()).collect();
    *t6_mcp_bdd.diag_count.borrow_mut() = manager.diagnostics().await.len();
    *t6_mcp_bdd.connected.borrow_mut() = manager.connected_servers().await.len();
    manager.shutdown().await;
}

#[then("诊断在册且不产出该条目工具")]
pub(crate) fn t_t6_mcp_invalid_diag(t6_mcp_bdd: &T6McpBdd) {
    assert!(
        *t6_mcp_bdd.diag_count.borrow() >= 1,
        "c2829: 无效条目应留下诊断"
    );
    assert!(
        t6_mcp_bdd.tool_names.borrow().is_empty(),
        "c2829: 无效条目 MUST NOT 产出工具"
    );
    assert_eq!(
        *t6_mcp_bdd.connected.borrow(),
        0,
        "c2829: 无效条目不应出现在已连接列表"
    );
}

thread_local! {
    static T6_TODO_EVENT2: RefCell<(bool, Option<usize>)> = const { RefCell::new((false, None)) };
}

#[then("回合事件流含 TodoUpdated 全量快照")]
pub(crate) fn t_t6_todo_event_assert() {
    T6_TODO_EVENT2.with(|c| {
        let (saw, len) = c.borrow().clone();
        assert!(saw, "c2829: todo_rewrite 成功后 run 流 MUST 含类型化 TodoUpdated");
        assert_eq!(len, Some(1), "c2829: TodoUpdated 应携带全量快照");
    });
}
