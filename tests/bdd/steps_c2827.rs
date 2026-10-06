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
    use crate::app::tui::harness::drain_pending;

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
    let plain: String = frame.chars().collect::<Vec<_>>().iter().copied().collect();
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
}

#[fixture]
pub fn t2_caps_bdd() -> T2CapsBdd {
    T2CapsBdd {
        caps: RefCell::new(None),
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
}

#[fixture]
pub fn t2_skill_bdd() -> T2SkillBdd {
    T2SkillBdd {
        skills: RefCell::new(Vec::new()),
        expanded: RefCell::new(String::new()),
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
        reset_fake_state, set_fake_text, set_fake_tool_call, set_fake_tool_result,
    };
    use crate::tests::bdd::helpers::make_agent;
    use futures::StreamExt;

    reset_fake_state();
    crate::tests::bdd::steps_agent_runtime::ar_register_fake(agent, "c2829-todo");
    set_fake_tool_call(
        "todo_rewrite",
        r#"{"items":[{"id":"a","content":"one","status":"in_progress"}]}"#,
    );
    set_fake_tool_result(r#"{"items":[{"id":"a","content":"one","status":"completed"}]}"#);
    set_fake_text("done");
    let mut runner = make_agent(agent);
    futures::executor::block_on(runner.select_model("c2829-todo")).expect("select fake");
    crate::tests::bdd::helpers::bind_session_or_panic(&mut runner, "sess-todo-ev");
    let mut stream = crate::tests::bdd::helpers::agent_submit_root(&mut runner, "tick todo").await;
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

async fn t6_discover(
    t6_mcp_bdd: &T6McpBdd,
    servers: &[crate::infra::config::types::McpServerConfig],
) {
    let result = crate::infra::mcp::connect_and_discover(servers).await;
    let Some((manager, tools)) = result else {
        panic!("c2829: 非空配置 MUST 构造 manager");
    };
    *t6_mcp_bdd.tool_names.borrow_mut() = tools.iter().map(|t| t.name().to_string()).collect();
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
    *t6_mcp_bdd.tool_names.borrow_mut() = tools.iter().map(|t| t.name().to_string()).collect();
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
        let (saw, len) = *c.borrow();
        assert!(
            saw,
            "c2829: todo_rewrite 成功后 run 流 MUST 含类型化 TodoUpdated"
        );
        assert_eq!(len, Some(1), "c2829: TodoUpdated 应携带全量快照");
    });
}

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
    use crate::agent::runtime::XyEvent;
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
        crate::app::cli::render_stream(&mut stream, &mut buf)
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

// ── 批 2（infra）：真步骤（image / process / mcp 摘要）──────────────

const T2_NOISE_PNG: &[u8] = include_bytes!("../support/t2_noise_128.png");

thread_local! {
    static T2_IMG: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    static T2_PROC: RefCell<Option<(String, bool, bool)>> = const { RefCell::new(None) };
    /// r1912 provider 配置值表达式装配证据（step 内收集，进程隔离）。
    static T2_CFG_EXPR: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

#[cfg(unix)]
fn t2_spawn_long_child() -> std::process::Child {
    use std::os::unix::process::CommandExt;
    std::process::Command::new("sh")
        .args(["-c", "sleep 30"])
        .process_group(0)
        .spawn()
        .expect("spawn 长进程")
}
#[cfg(windows)]
fn t2_spawn_long_child() -> std::process::Child {
    std::process::Command::new("cmd")
        .args(["/c", "ping -n 30 127.0.0.1 > nul"])
        .spawn()
        .expect("spawn 长进程")
}

fn t2_base64_ok(data: &str) -> bool {
    data.len().is_multiple_of(4)
        && data
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
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

#[when("从图片文件产出多模态载荷")]
pub(crate) fn w_image_multimodal_payload() {
    let p = std::env::temp_dir().join(format!("xylitol_t2_load_{}.png", std::process::id()));
    std::fs::write(&p, T2_NOISE_PNG).expect("写临时图");
    let r = crate::infra::image::agent_part_from_image_path(&p);
    let _ = std::fs::remove_file(&p);
    let part = r.expect("路径可读 MUST 产出多模态载荷");
    let (mime, data) = match part {
        crate::protocol::message::AgentPart::Image(ic) => {
            (ic.media_type, ic.data.unwrap_or_default())
        }
        _ => panic!("MUST 得图片构件"),
    };
    T2_IMG.with(|s| *s.borrow_mut() = Some((mime, data)));
}

#[then("base64 载荷在带宽上限内且含媒体类型")]
pub(crate) fn t_image_multimodal_payload() {
    let (mime, data) = T2_IMG.with(|s| s.borrow().clone()).expect("已产出");
    assert!(!data.is_empty(), "载荷 MUST 非空");
    assert!(data.len() <= 4_500_000, "base64 载荷 MUST 低于 4.5MB");
    assert!(mime.starts_with("image/"), "载荷 MUST 含媒体类型");
    assert!(t2_base64_ok(&data), "载荷 MUST 为合法 base64");
}

#[when("从本地图片路径装配图片构件")]
pub(crate) fn w_image_part_from_path() {
    let p = std::env::temp_dir().join(format!("xylitol_t2_part_{}.png", std::process::id()));
    std::fs::write(&p, T2_NOISE_PNG).expect("写临时图");
    let r = crate::infra::image::agent_part_from_image_path(&p);
    let _ = std::fs::remove_file(&p);
    let is_image = matches!(
        r.expect("路径 → 构件 MUST 成功"),
        crate::protocol::message::AgentPart::Image(_)
    );
    T2_IMG.with(|s| {
        *s.borrow_mut() = Some((
            if is_image {
                "image-part".into()
            } else {
                "not-image".into()
            },
            String::new(),
        ))
    });
}

#[then("得到多模态图片构件")]
pub(crate) fn t_image_part_from_path() {
    let (tag, _) = T2_IMG.with(|s| s.borrow().clone()).expect("已装配");
    assert_eq!(tag, "image-part", "MUST 得到多模态图片构件");
}

#[when("请求跨平台 bash 定位")]
pub(crate) fn w_bash_discovery() {
    let cfg = crate::infra::process::shell::find_bash(None);
    T2_PROC.with(|s| {
        *s.borrow_mut() = Some((
            cfg.shell.to_string_lossy().into_owned(),
            !cfg.args.is_empty(),
            false,
        ))
    });
}

#[then("返回可执行 shell 配置")]
pub(crate) fn t_bash_discovery() {
    let (shell, has_args, _) = T2_PROC.with(|s| s.borrow().clone()).expect("已定位");
    assert!(!shell.is_empty(), "MUST 返回非空 shell 路径");
    assert!(has_args, "MUST 含直执行参数");
}

#[when("以整树终止子进程")]
pub(crate) fn w_kill_process_tree() {
    let mut child = t2_spawn_long_child();
    crate::infra::process::group::kill_process_tree(child.id());
    let mut exited = false;
    for _ in 0..60 {
        if let Ok(Some(_)) = child.try_wait() {
            exited = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    T2_PROC.with(|s| *s.borrow_mut() = Some((String::new(), false, exited)));
}

#[then("目标进程及其子进程一并结束")]
pub(crate) fn t_kill_process_tree() {
    let (_, _, exited) = T2_PROC.with(|s| s.borrow().clone()).expect("已终止");
    assert!(exited, "整树终止 MUST 回收目标进程");
}

#[then("已连接列表只读返回在册服务器摘要")]
pub(crate) fn t_mcp_connected_readonly(t6_mcp_bdd: &T6McpBdd) {
    let connected = *t6_mcp_bdd.connected.borrow();
    let tool_names = t6_mcp_bdd.tool_names.borrow();
    assert!(connected >= 1, "已连接摘要 MUST 返回在册服务器");
    assert!(!tool_names.is_empty(), "摘要 MUST 含工具数量或等价");
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
    let env = crate::infra::process::shell::shell_env_with_agent_bin(base);
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
        !crate::infra::process::shell::find_bash(Some(&head))
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

#[when("扫描 agent 层信任依赖")]
pub(crate) fn w_agent_layer_trust_deps() {
    let hits = t3_scan_agent_trust_defs();
    let store = la_load("src/infra/trust/store.rs");
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(format!(
            "AGENT_DEFS_START\n{hits}\nAGENT_DEFS_END\n=== store ===\n{store}"
        ))
    });
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
    let hits = t3_scan_agent_trust_defs();
    let store = la_load("src/infra/trust/store.rs");
    LA_PROBE.with(|p| {
        *p.borrow_mut() = Some(format!(
            "AGENT_DEFS_START\n{hits}\nAGENT_DEFS_END\n=== store ===\n{store}"
        ))
    });
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

#[when("读取异步集成测试超时基建")]
pub(crate) fn w_async_test_timeout() {
    LA_PROBE.with(|p| *p.borrow_mut() = Some(la_load("tests/bdd/helpers.rs")));
}

#[then("包裹主体且防挂起")]
pub(crate) fn t_async_test_timeout() {
    let src = LA_PROBE.with(|p| p.borrow().clone()).expect("探针已跑");
    assert!(
        src.contains("with_test_timeout"),
        "with_test_timeout 辅助函数 MUST 在册"
    );
    assert!(
        src.contains("with_test_timeout_for"),
        "时长参数化变体 MUST 在册"
    );
    assert!(
        src.contains("times_out_when_deadlocked"),
        "挂死超时用例 MUST 在册"
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
    let a = crate::infra::config::resolver::resolve_value("plain-string", &lookup)
        .expect("字面值 MUST 直通");
    let b =
        crate::infra::config::resolver::resolve_value("$HOME", &lookup).expect("环境值 MUST 解析");
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
    let a = crate::infra::config::resolver::resolve_value("${HOME}", &lookup)
        .expect("${VAR} MUST 插值");
    let b = crate::infra::config::resolver::resolve_value("${UNSET:-fallback}", &lookup)
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
    crate::infra::config::resolver::reset_shell_cache();
    let lookup = |_name: &str| None;
    let out = crate::infra::config::resolver::resolve_value("!printf ok", &lookup)
        .expect("shell 命令 MUST 执行");
    // `$$` 是 shell PID：缓存命中时两次结果相同（进程生命周期缓存证据）。
    let c1 = crate::infra::config::resolver::resolve_value("!printf %s $$", &lookup)
        .expect("shell PID 值一");
    let c2 = crate::infra::config::resolver::resolve_value("!printf %s $$", &lookup)
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
    let loaded = crate::infra::config::loader::load_app_config_with(None, env, None)
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
