//! Product TUI BDD scenarios (c715) — rstest-bdd, HostSession harness.
//!
//! Feature files live under `tests/features/app-tui-*.feature`.

use std::cell::RefCell;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use xylitol_tui::{InputEvent, Terminal};

use super::bridge::{BashBlockStatus, UiEntry};
use super::effects::{drain_pending, run_interactive_bang};
use super::harness::{ScriptedDriver, pump_host_driver};
use super::host::{HostEvent, HostSession};
use super::layout::EditorSlot;
use crate::app::core::driver::Driver;
use crate::domain::lifecycle::XyEvent;

struct TestTerminal {
    cols: u16,
    rows: u16,
    frames: Vec<String>,
    started: bool,
    stopped: bool,
}

impl TestTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            frames: Vec::new(),
            started: false,
            stopped: false,
        }
    }
}

impl Terminal for TestTerminal {
    fn write(&mut self, data: &str) {
        self.frames.push(data.to_string());
    }
    fn columns(&self) -> u16 {
        self.cols
    }
    fn rows(&self) -> u16 {
        self.rows
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }
    fn start(&mut self) {
        self.started = true;
    }
    fn stop(&mut self) {
        self.stopped = true;
    }
}

fn key(code: KeyCode, mods: KeyModifiers) -> InputEvent {
    InputEvent::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn enter_event() -> InputEvent {
    key(KeyCode::Enter, KeyModifiers::NONE)
}

fn esc_event() -> InputEvent {
    key(KeyCode::Esc, KeyModifiers::NONE)
}

fn alt_enter_event() -> InputEvent {
    key(KeyCode::Enter, KeyModifiers::ALT)
}

fn alt_up_event() -> InputEvent {
    key(KeyCode::Up, KeyModifiers::ALT)
}

pub struct TuiBdd {
    session: RefCell<HostSession<TestTerminal>>,
    driver: RefCell<ScriptedDriver>,
    stream: RefCell<Option<crate::app::core::driver::EventStream>>,
}

impl TuiBdd {
    fn new() -> Self {
        Self {
            session: RefCell::new(HostSession::new_product_ui(TestTerminal::new(80, 24))),
            driver: RefCell::new(ScriptedDriver::new()),
            stream: RefCell::new(None),
        }
    }
}

#[fixture]
fn tui() -> TuiBdd {
    TuiBdd::new()
}

#[given("产品 TUI harness 已启动")]
fn g_harness_ready(_tui: &TuiBdd) {}

#[when("agent 忙碌时按 Esc 且在 drain 前注入迟到 TextDelta")]
async fn w_busy_esc_before_drain_delta(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    session.on_run_started("busy");
    session.step(HostEvent::Input(esc_event())).expect("esc");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(
            "SHOULD_NOT_APPEAR".into(),
        ))))
        .expect("delta");
    pump_host_driver(&mut session, &mut *driver, &mut stream)
        .await
        .expect("pump");
}

#[when("agent 忙碌时按 Esc abort")]
async fn w_busy_esc_abort(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    session.on_run_started("busy");
    session.step(HostEvent::Input(esc_event())).expect("esc");
    pump_host_driver(&mut session, &mut *driver, &mut stream)
        .await
        .expect("pump");
}

#[when("提交 hanging bang 并 Esc 取消")]
async fn w_hanging_bang_esc(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    driver.set_hang_bash_until_abort(true);
    let root = session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("!sleep 99");
    session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    drain_pending(&mut session, &mut *driver, &mut stream)
        .await
        .expect("drain");
    let bash = session.take_bash().expect("bang");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        let _ = tx.send(Ok(HostEvent::Input(esc_event())));
        std::future::pending::<()>().await;
    });
    let input = futures::stream::unfold(
        rx,
        |mut rx| async move { rx.recv().await.map(|ev| (ev, rx)) },
    );
    run_interactive_bang(&mut session, &mut *driver, bash, &mut stream, input)
        .await
        .expect("bang");
}

#[when("agent 忙碌且编辑器有文本时按 Enter")]
async fn w_busy_enter_steer(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    session.on_run_started("busy");
    let root = session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("nudge");
    session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    pump_host_driver(&mut session, &mut *driver, &mut stream)
        .await
        .expect("pump");
}

#[when("agent 忙碌且编辑器有文本时按 Alt+Enter")]
async fn w_busy_alt_enter(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    session.on_run_started("busy");
    let root = session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("later");
    session
        .step(HostEvent::Input(alt_enter_event()))
        .expect("alt-enter");
    pump_host_driver(&mut session, &mut *driver, &mut stream)
        .await
        .expect("pump");
}

#[given("已入队 steer 与 follow-up")]
async fn g_queued(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    session.on_run_started("busy");
    let root = session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("s1");
    session
        .step(HostEvent::Input(enter_event()))
        .expect("steer");
    root.borrow_mut().set_editor_text("f1");
    session
        .step(HostEvent::Input(alt_enter_event()))
        .expect("fu");
    pump_host_driver(&mut session, &mut *driver, &mut stream)
        .await
        .expect("pump");
}

#[when("按 Alt+Up")]
async fn w_alt_up(tui: &TuiBdd) {
    let mut session = tui.session.borrow_mut();
    let mut driver = tui.driver.borrow_mut();
    let mut stream = tui.stream.borrow_mut();
    session
        .step(HostEvent::Input(alt_up_event()))
        .expect("alt-up");
    pump_host_driver(&mut session, &mut *driver, &mut stream)
        .await
        .expect("pump");
}

#[then("UI 含 Aborted 系统提示")]
fn t_aborted_note(tui: &TuiBdd) {
    let session = tui.session.borrow();
    assert!(
        session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted")),
        "entries: {:?}",
        session.ui_model().entries
    );
}

#[then("无 assistant 正文含 SHOULD_NOT_APPEAR")]
fn t_no_late_assistant(tui: &TuiBdd) {
    let session = tui.session.borrow();
    assert!(
        !session.ui_model().entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text } if text.contains("SHOULD_NOT_APPEAR")
        )),
        "entries: {:?}",
        session.ui_model().entries
    );
}

#[then("Driver abort 已被调用")]
fn t_abort_called(tui: &TuiBdd) {
    assert!(tui.driver.borrow().abort_count() >= 1);
}

#[then("bash 块为 Cancelled 且含 cancelled")]
fn t_bash_cancelled(tui: &TuiBdd) {
    let session = tui.session.borrow();
    assert!(
        session.ui_model().entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                status: BashBlockStatus::Cancelled,
                output,
                ..
            } if output.contains("(cancelled)")
        )),
        "entries: {:?}",
        session.ui_model().entries
    );
}

#[then("无 Aborted 系统提示")]
fn t_no_aborted(tui: &TuiBdd) {
    let session = tui.session.borrow();
    assert!(
        !session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
    );
}

#[then("EditorSlot 不是 Tree")]
fn t_not_tree(tui: &TuiBdd) {
    let session = tui.session.borrow();
    let root = session.ui_root().expect("ui");
    assert_ne!(root.borrow().slot(), EditorSlot::Tree);
}

#[then("Driver steer 被调用一次")]
fn t_steer_once(tui: &TuiBdd) {
    assert_eq!(tui.driver.borrow().steer_calls.len(), 1);
}

#[then("Driver follow_up 被调用一次")]
fn t_follow_up_once(tui: &TuiBdd) {
    assert_eq!(tui.driver.borrow().follow_up_calls.len(), 1);
}

#[then("Driver 双队列已清空")]
fn t_queues_clear(tui: &TuiBdd) {
    let stats = tui.driver.borrow().queue_stats();
    assert_eq!(stats.steer_count, 0);
    assert_eq!(stats.follow_up_count, 0);
}

#[scenario(
    path = "tests/features/app-tui-abort.feature",
    name = "busy Esc 后迟到 TextDelta 不得复活"
)]
async fn sc_abort_late(tui: TuiBdd) {}

#[scenario(
    path = "tests/features/app-tui-bang.feature",
    name = "hanging bang Esc 取消"
)]
async fn sc_bang_esc(tui: TuiBdd) {}

#[scenario(
    path = "tests/features/app-tui-esc-overlay.feature",
    name = "busy 无 overlay Esc 不开树"
)]
async fn sc_esc_no_tree(tui: TuiBdd) {}

#[scenario(
    path = "tests/features/app-tui-queue.feature",
    name = "busy Enter 触发 steer"
)]
async fn sc_steer(tui: TuiBdd) {}

#[scenario(
    path = "tests/features/app-tui-queue.feature",
    name = "busy Alt+Enter 触发 follow-up"
)]
async fn sc_follow_up(tui: TuiBdd) {}

#[scenario(
    path = "tests/features/app-tui-queue.feature",
    name = "Alt+Up 清空双队列"
)]
async fn sc_alt_up(tui: TuiBdd) {}
