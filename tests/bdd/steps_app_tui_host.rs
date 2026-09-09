//! Steps for the host-pump family (att9 / att11, later ati busy keys).
//!
//! ati30 contract: BDD reuses [`HostSession`] + [`ScriptedDriver`] +
//! [`pump_host_driver`] — the very same pump the unit slice tests use; never a
//! second side-effect pump.

use crate::app::tui::TuiHostEvent as HostEvent;
use crate::app::tui::TuiHostSession as HostSession;
use crate::app::tui::harness::{ScriptedDriver, TestTerminal, enter_event, pump_host_driver};
use crate::app::tui::{BashBlockStatus, UiEntry};
use crate::protocol::ports::XyBashResult;
use crate::tests::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol_tui::Component;

/// One mounted host pump (session + scripted driver). Steps take it out,
/// mutate, and put it back — `HostSession` is not shareable behind `&self`.
pub struct HostPump {
    pub session: HostSession<TestTerminal>,
    pub driver: ScriptedDriver,
}

pub struct HostPumpBdd {
    pub pump: RefCell<Option<HostPump>>,
    /// Frames rendered for assertions (ANSI), newest last.
    pub ansi_frames: RefCell<Vec<String>>,
}

#[fixture]
pub fn host_pump_bdd() -> HostPumpBdd {
    HostPumpBdd {
        pump: RefCell::new(None),
        ansi_frames: RefCell::new(Vec::new()),
    }
}

fn fresh_pump() -> HostPump {
    HostPump {
        session: HostSession::new_product_ui(TestTerminal::new(120, 40)),
        driver: ScriptedDriver::new(),
    }
}

fn take_pump(bdd: &HostPumpBdd) -> HostPump {
    bdd.pump.borrow_mut().take().expect("host pump mounted")
}

fn put_pump(bdd: &HostPumpBdd, pump: HostPump) {
    *bdd.pump.borrow_mut() = Some(pump);
}

/// Submit `command` through the real host key path (editor text + Enter + pump).
async fn submit_bang(bdd: &HostPumpBdd, command: &str) {
    let mut pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text(command);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter step");
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    put_pump(bdd, pump);
}

fn render_frame(bdd: &HostPumpBdd, width: usize) -> String {
    let pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let frame = root.borrow_mut().render(width).join("\n");
    put_pump(bdd, pump);
    bdd.ansi_frames.borrow_mut().push(frame.clone());
    frame
}

/// (command, output, status) of the newest Bash block in the scrollback.
fn last_bash_entry(bdd: &HostPumpBdd) -> (String, String, BashBlockStatus) {
    let pump = take_pump(bdd);
    let out = pump
        .session
        .ui_model()
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEntry::Bash {
                command,
                output,
                status,
                ..
            } => Some((command.clone(), output.clone(), *status)),
            _ => None,
        })
        .expect("a Bash block in scrollback");
    put_pump(bdd, pump);
    out
}

// ---- att9：bang 块生命周期（进块 / 同块刷新 / 成功 / 取消 / 非零） ----

#[when("以主机泵提交 bang 命令并注入分段输出与成功结果")]
pub(crate) async fn w_att9_submit_success(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.driver.push_bash_result(XyBashResult {
        output: "chunk-a\nchunk-b".into(),
        exit_code: Some(0),
        ..Default::default()
    });
    put_pump(host_pump_bdd, pump);
    submit_bang(host_pump_bdd, "!echo hi").await;
}

#[then("命令进入单一 Bash 块且输出在同一块内且状态轨为成功")]
pub(crate) async fn t_att9_single_success_block(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let bash_count = pump
        .session
        .ui_model()
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Bash { .. }))
        .count();
    put_pump(host_pump_bdd, pump);
    assert_eq!(
        bash_count, 1,
        "att9: one bang = one Bash block in scrollback"
    );
    let (command, output, status) = last_bash_entry(host_pump_bdd);
    assert!(
        command.contains("echo hi"),
        "command must land in the scrollback block: {command:?}"
    );
    assert!(
        output.contains("chunk-a") && output.contains("chunk-b"),
        "output chunks refresh inside the same block: {output:?}"
    );
    assert_eq!(
        status,
        BashBlockStatus::Success,
        "success rail after exit 0"
    );
}

#[when("注入取消的结果")]
pub(crate) async fn w_att9_submit_cancelled(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.driver.push_bash_result(XyBashResult {
        cancelled: true,
        ..Default::default()
    });
    put_pump(host_pump_bdd, pump);
    submit_bang(host_pump_bdd, "!sleep 5").await;
}

#[then("块内呈现 (cancelled) 而非 agent 的 Aborted")]
pub(crate) async fn t_att9_cancelled_not_aborted(host_pump_bdd: &HostPumpBdd) {
    let (command, output, status) = last_bash_entry(host_pump_bdd);
    assert!(
        command.contains("sleep 5"),
        "cancelled bang keeps its own block: {command:?}"
    );
    assert_eq!(status, BashBlockStatus::Cancelled);
    assert!(
        output.contains("(cancelled)"),
        "att9: bang cancel renders (cancelled): {output:?}"
    );
    assert!(
        !output.contains("Aborted"),
        "att9: bang cancel MUST NOT borrow the agent's Aborted copy: {output:?}"
    );
}

#[when("注入非零退出码结果")]
pub(crate) async fn w_att9_submit_nonzero(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.driver.push_bash_result(XyBashResult {
        output: "boom".into(),
        exit_code: Some(1),
        ..Default::default()
    });
    put_pump(host_pump_bdd, pump);
    submit_bang(host_pump_bdd, "!false").await;
}

#[then("块状态为 error 且以错误轨强调")]
pub(crate) async fn t_att9_error_rail(host_pump_bdd: &HostPumpBdd) {
    let (_, output, status) = last_bash_entry(host_pump_bdd);
    assert_eq!(status, BashBlockStatus::Error);
    assert!(
        output.contains("boom") && output.contains("exit 1"),
        "non-zero exit shows output + exit code emphasis: {output:?}"
    );
    let ansi = render_frame(host_pump_bdd, 120);
    assert!(
        ansi.contains("\x1b[48;2;") && ansi.contains("\x1b[49m"),
        "att9/att11: rail uses bg cells closed by the 49m reset:\n{ansi:?}"
    );
}

// ---- att11：bang 块 rail 复用（轨+gutter，无整行洗底） ----

#[when("以主机泵提交 bang 命令并完成成功结果")]
pub(crate) async fn w_att11_submit_done(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.driver.push_bash_result(XyBashResult {
        output: "ok".into(),
        exit_code: Some(0),
        ..Default::default()
    });
    put_pump(host_pump_bdd, pump);
    submit_bang(host_pump_bdd, "!echo done").await;
}

#[then("bang 块行带单列状态轨加无底色 gutter 且内容区无整行洗底")]
pub(crate) async fn t_att11_rail_no_wash(host_pump_bdd: &HostPumpBdd) {
    use crate::tests::bdd::steps_app_tui_transcript::rail_prefix;
    let ansi = render_frame(host_pump_bdd, 120);
    let header = ansi
        .lines()
        .find(|l| l.contains("echo done"))
        .unwrap_or_else(|| panic!("bang header line in frame:\n{ansi}"));
    let theme = crate::app::tui::LayoutTheme::product_dark();
    let p = theme.palette();
    let prefix = rail_prefix(xylitol_tui::mix_rgb(p.surface, p.success, 0.72));
    assert!(
        header.starts_with(&prefix),
        "att11: bang rail = 1 bg cell + 49m reset + bare gutter (paint_left_rail_line):\n{header:?}"
    );
    let rest = &header[prefix.len()..];
    assert!(
        !rest.contains("48;2;"),
        "att11: no full-row tool-*-bg wash past the rail:\n{header:?}"
    );
}

// ---- ati busy 键序族（ati2/3/10/14/16/19/20/28/30/31/32/43） ----

fn ctrl_c_event() -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn alt_enter_event() -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

use crate::app::tui::harness::esc_event;

/// HostEvent stream for hanging bang/reload: pause, then Esc (+optional
/// backlog), then park forever (no EOF).
fn esc_stream(
    after_ms: u64,
    backlog_esc: usize,
) -> impl futures::Stream<Item = Result<HostEvent, crate::XyDriverError>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(after_ms)).await;
        let _ = tx.send(Ok(HostEvent::Input(esc_event())));
        for _ in 0..backlog_esc {
            let _ = tx.send(Ok(HostEvent::Input(esc_event())));
        }
        std::future::pending::<()>().await;
    });
    futures::stream::unfold(
        rx,
        |mut rx| async move { rx.recv().await.map(|ev| (ev, rx)) },
    )
}

/// Start a busy agent turn on a fresh pump and mount it.
fn open_busy(bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.session.on_run_started("hello");
    put_pump(bdd, pump);
}

/// Mount a fresh pump with sample tree data (for tree-slot keys).
fn open_tree_ready(bdd: &HostPumpBdd) {
    use crate::app::tui::harness::harness_sample_message_history_tree;
    let mut pump = fresh_pump();
    pump.driver
        .set_message_history_tree(harness_sample_message_history_tree());
    put_pump(bdd, pump);
}

fn set_editor(bdd: &HostPumpBdd, text: &str) {
    let pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text(text);
    put_pump(bdd, pump);
}

fn editor_text(bdd: &HostPumpBdd) -> String {
    let pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let text = root.borrow().editor_text();
    put_pump(bdd, pump);
    text
}

fn step_key(bdd: &HostPumpBdd, ev: xylitol_tui::InputEvent) {
    let mut pump = take_pump(bdd);
    pump.session.step(HostEvent::Input(ev)).expect("key step");
    put_pump(bdd, pump);
}

async fn drain(bdd: &HostPumpBdd) {
    use crate::app::tui::harness::drain_pending;
    let mut pump = take_pump(bdd);
    let mut stream = None;
    drain_pending(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("drain");
    put_pump(bdd, pump);
}

async fn pump_once(bdd: &HostPumpBdd) {
    let mut pump = take_pump(bdd);
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    put_pump(bdd, pump);
}

/// Read-only probe of (session state, driver counters).
#[derive(Clone, Debug)]
struct PumpStats {
    is_busy: bool,
    bash_active: bool,
    run_active: bool,
    should_quit: bool,
    tree_open: bool,
    aborts: usize,
    runs: Vec<String>,
    steers: Vec<String>,
    follow_ups: Vec<String>,
    bash_calls: Vec<(String, bool)>,
    tree_calls: usize,
    reload_active: bool,
    notices: Vec<String>,
}

fn stats(bdd: &HostPumpBdd) -> PumpStats {
    let pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let s = PumpStats {
        is_busy: pump.session.is_busy(),
        bash_active: pump.session.bash_active(),
        run_active: pump.session.run_active(),
        should_quit: pump.session.should_quit(),
        tree_open: root.borrow().tree_open(),
        aborts: pump.driver.abort_count(),
        runs: pump.driver.runs.clone(),
        steers: pump.driver.steer_calls.clone(),
        follow_ups: pump.driver.follow_up_calls.clone(),
        bash_calls: pump.driver.bash_calls(),
        tree_calls: pump.driver.session_tree_calls(),
        reload_active: pump.session.reload_active(),
        notices: pump
            .session
            .ui_model()
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::ScrollNotice { text } => Some(text.clone()),
                _ => None,
            })
            .collect(),
    };
    put_pump(bdd, pump);
    s
}

/// Submit current editor text with a key event, then pump.
async fn submit_with(bdd: &HostPumpBdd, ev: xylitol_tui::InputEvent) {
    step_key(bdd, ev);
    pump_once(bdd).await;
}

// ati2 ─────────────────────────────────────────────────────────

#[when("以主机泵开启忙碌流并按下 Esc")]
pub(crate) async fn w_ati2_busy_esc(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
}

#[then("abort 计一次且未退出且回到可输入 idle")]
pub(crate) async fn t_ati2_abort_once_idle(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 1, "Esc while busy must abort the driver run");
    assert!(!s.should_quit, "busy Esc MUST NOT quit the TUI");
    assert!(!s.is_busy, "session must return to input-able idle");
    assert!(
        s.notices
            .iter()
            .any(|t| t == "Aborted" || t == "Operation aborted"),
        "abort note expected: {:?}",
        s.notices
    );
}

#[when("再次开启忙碌流并按下 Ctrl+C")]
pub(crate) async fn w_ati2_busy_ctrl_c(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.on_run_started("again");
    put_pump(host_pump_bdd, pump);
    step_key(host_pump_bdd, ctrl_c_event());
    pump_once(host_pump_bdd).await;
}

#[then("abort 同语义计两次且未退出")]
pub(crate) async fn t_ati2_ctrl_c_same_semantics(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 2, "busy Ctrl+C shares the abort latch (c1570)");
    assert!(!s.should_quit, "busy Ctrl+C MUST NOT quit");
}

#[when("打开样例树槽后按下 Ctrl+C")]
pub(crate) async fn w_ati2_tree_ctrl_c(host_pump_bdd: &HostPumpBdd) {
    // 新 idle 语境（abort 后 suppress_idle_esc 会吞首个 idle Esc，树阶段
    // 独立开局），aborts 计数从零起证「未新增」。
    open_tree_ready(host_pump_bdd);
    step_key(host_pump_bdd, esc_event());
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await; // 树抓取经 driver 异步泵入
    let s = stats(host_pump_bdd);
    assert!(s.tree_open, "double Esc must open the tree first: {s:?}");
    step_key(host_pump_bdd, ctrl_c_event());
}

#[then("树槽关闭且未退出且未新增 abort")]
pub(crate) async fn t_ati2_overlay_closes_first(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.tree_open, "Ctrl+C with an overlay closes the slot first");
    assert!(!s.should_quit);
    assert_eq!(s.aborts, 0, "overlay Ctrl+C MUST NOT abort");
}

#[when("编辑器输入草稿后按下 Ctrl+C")]
pub(crate) async fn w_ati2_draft_ctrl_c(host_pump_bdd: &HostPumpBdd) {
    set_editor(host_pump_bdd, "draft");
    step_key(host_pump_bdd, ctrl_c_event());
}

#[then("编辑器被清空且未退出且未新增 abort")]
pub(crate) async fn t_ati2_editor_cleared(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        editor_text(host_pump_bdd),
        "",
        "idle Ctrl+C clears the editor"
    );
    let s = stats(host_pump_bdd);
    assert!(!s.should_quit);
    assert_eq!(s.aborts, 0, "idle Ctrl+C on a draft MUST NOT abort");
}

#[when("清空编辑器再按下 Ctrl+C")]
pub(crate) async fn w_ati2_empty_ctrl_c(host_pump_bdd: &HostPumpBdd) {
    set_editor(host_pump_bdd, "");
    step_key(host_pump_bdd, ctrl_c_event());
}

#[then("会话请求退出")]
pub(crate) async fn t_ati2_quit_requested(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.should_quit, "idle empty-editor Ctrl+C quits the TUI");
}

// ati3 ─────────────────────────────────────────────────────────

#[when("以主机泵开启忙碌流并输入 nudge 后按 Enter")]
pub(crate) async fn w_ati3_steer(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    set_editor(host_pump_bdd, "nudge");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("steer 收到 nudge 且未发起第二次 run")]
pub(crate) async fn t_ati3_steer_queued(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.steers, vec!["nudge".to_string()]);
    assert!(
        s.runs.is_empty(),
        "busy Enter MUST NOT start a run: {:?}",
        s.runs
    );
}

#[when("输入 later 并按 Alt+Enter")]
pub(crate) async fn w_ati3_follow_up(host_pump_bdd: &HostPumpBdd) {
    set_editor(host_pump_bdd, "later");
    submit_with(host_pump_bdd, alt_enter_event()).await;
}

#[then("follow_up 收到 later 且未发起第二次 run")]
pub(crate) async fn t_ati3_follow_up_queued(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.follow_ups, vec!["later".to_string()]);
    assert!(s.runs.is_empty());
}

// ati10 ────────────────────────────────────────────────────────

#[when("以主机泵开启忙碌流并按下 Esc 后收流关闭")]
pub(crate) async fn w_ati10_busy_esc(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    step_key(host_pump_bdd, esc_event());
    let mut pump = take_pump(host_pump_bdd);
    pump.session.on_run_stream_closed();
    put_pump(host_pump_bdd, pump);
    drain(host_pump_bdd).await;
}

#[then("会话树未打开")]
pub(crate) async fn t_ati10_no_tree(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.tree_open, "busy Esc MUST NOT open the session tree");
    assert_eq!(s.tree_calls, 0);
}

// ati14 ────────────────────────────────────────────────────────

#[then("run 不再活跃且 abort 已计一次")]
pub(crate) async fn t_ati14_run_closed(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.run_active);
    assert_eq!(s.aborts, 1);
}

#[then("abort 计一次且清队为 steer 不清 follow_up")]
pub(crate) async fn t_ati10_clear_steer_only(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 1);
    let pump = take_pump(host_pump_bdd);
    let clear = pump.driver.clear_calls.clone();
    put_pump(host_pump_bdd, pump);
    assert!(
        clear.iter().any(|&(steer, follow_up)| steer && !follow_up),
        "clear_queue(steer=true, follow_up=false): {clear:?}"
    );
}

#[when("输入 second 并按 Enter")]
pub(crate) async fn w_ati14_second_run(host_pump_bdd: &HostPumpBdd) {
    set_editor(host_pump_bdd, "second");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("run 收到 second 且无粘性 aborted 错误")]
pub(crate) async fn t_ati14_runs_again(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.runs, vec!["second".to_string()]);
    let abort_notes = s
        .notices
        .iter()
        .filter(|t| *t == "Aborted" || *t == "Operation aborted")
        .count();
    assert_eq!(
        abort_notes, 1,
        "abort note appears exactly once — no sticky aborted: {:?}",
        s.notices
    );
}

// ati16 ────────────────────────────────────────────────────────

#[when("以主机泵在 idle 提交 bang 命令")]
pub(crate) async fn w_ati16_bang(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.driver.push_bash_result(XyBashResult {
        output: "ok".into(),
        exit_code: Some(0),
        ..Default::default()
    });
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "!echo hi");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("execute_bash 收到命令体且未调用 run")]
pub(crate) async fn t_ati16_bash_not_run(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.bash_calls, vec![("echo hi".to_string(), false)]);
    assert!(s.runs.is_empty(), "bang MUST NOT go through Driver::run");
}

#[when("提交双感叹号命令")]
pub(crate) async fn w_ati16_bangbang(host_pump_bdd: &HostPumpBdd) {
    set_editor(host_pump_bdd, "!!echo x");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("exclude_from_context 为真")]
pub(crate) async fn t_ati16_exclude_true(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.bash_calls.iter().any(|(c, excl)| c == "echo x" && *excl),
        "!! sets exclude_from_context: {:?}",
        s.bash_calls
    );
}

#[when("提交空命令体的感叹号")]
pub(crate) async fn w_ati16_empty_bang(host_pump_bdd: &HostPumpBdd) {
    set_editor(host_pump_bdd, "!");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("有提示且未执行 bash 且未调用 run")]
pub(crate) async fn t_ati16_empty_rejected(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.bash_calls.len(), 2, "empty bang body must not execute");
    assert!(s.runs.is_empty());
    assert!(
        !s.notices.is_empty(),
        "empty bang body must surface a notice: {:?}",
        s.notices
    );
}

// ati19 ────────────────────────────────────────────────────────

fn fresh_pump_with_hanging_bash() -> HostPump {
    let pump = fresh_pump();
    pump.driver.set_hang_bash_until_abort(true);
    pump
}

#[when("以主机泵提交挂起 bang 并经输入流注入 Esc")]
pub(crate) async fn w_ati19_hanging_bang_esc(host_pump_bdd: &HostPumpBdd) {
    use crate::app::tui::harness::run_interactive_bang;
    let pump = fresh_pump_with_hanging_bash();
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "!sleep 99");
    step_key(host_pump_bdd, enter_event());
    drain(host_pump_bdd).await;
    let mut pump = take_pump(host_pump_bdd);
    let bash = pump.session.take_bash().expect("pending bang");
    let mut stream = None;
    run_interactive_bang(
        &mut pump.session,
        &mut pump.driver,
        bash,
        &mut stream,
        esc_stream(40, 0),
    )
    .await
    .expect("bang loop");
    put_pump(host_pump_bdd, pump);
}

#[then("abort 到达驱动且 Bash 块为 cancelled 且回到 idle")]
pub(crate) async fn t_ati19_cancelled_idle(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    let pump = take_pump(host_pump_bdd);
    let entries: Vec<UiEntry> = pump.session.ui_model().entries.clone();
    put_pump(host_pump_bdd, pump);
    assert!(s.aborts >= 1, "Esc during bang must reach Driver::abort");
    assert!(
        !s.is_busy && !s.bash_active,
        "back to input-able idle after cancel"
    );
    assert!(
        entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                command, status, ..
            } if command == "sleep 99" && *status == BashBlockStatus::Cancelled
        )),
        "bash block must be Cancelled: {entries:?}"
    );
}

#[then("无 agent 的 Aborted 滚动提示")]
pub(crate) async fn t_ati19_no_agent_aborted(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        !s.notices
            .iter()
            .any(|t| t == "Aborted" || t == "Operation aborted"),
        "bang cancel must not borrow the agent abort footer: {:?}",
        s.notices
    );
}

// ati20 ────────────────────────────────────────────────────────

#[when("以主机泵令 bash 执行中并提交第二条 bang")]
pub(crate) async fn w_ati20_second_bang(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.session.begin_bash_exec("sleep 99", false);
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "!echo second");
    step_key(host_pump_bdd, enter_event());
}

#[then("有硬拒提示且编辑器保留命令体")]
pub(crate) async fn t_ati20_hard_reject_notice(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.notices.iter().any(|t| t.contains("rejected")),
        "hard-reject notice expected: {:?}",
        s.notices
    );
    assert_eq!(
        editor_text(host_pump_bdd),
        "!echo second",
        "editor keeps the command body"
    );
}

#[then("未发起第二次 execute_bash 且未排队")]
pub(crate) async fn t_ati20_no_second_bash(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.bash_calls.is_empty(),
        "second bang must not execute: {:?}",
        s.bash_calls
    );
}

// ati28 ────────────────────────────────────────────────────────

#[when("以主机泵在 idle 提交斜杠 session-tree")]
pub(crate) async fn w_ati28_slash_tree(host_pump_bdd: &HostPumpBdd) {
    open_tree_ready(host_pump_bdd);
    set_editor(host_pump_bdd, "/session-tree");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("会话树打开且未作为 prompt 调用 run")]
pub(crate) async fn t_ati28_tree_opened_not_run(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.tree_open, "slash opens the tree slot");
    assert_eq!(s.tree_calls, 1);
    assert!(s.runs.is_empty(), "/session-tree is not an agent prompt");
}

#[when("开启忙碌流再提交斜杠 session-tree")]
pub(crate) async fn w_ati28_busy_slash_tree(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.on_run_started("busy");
    pump.session
        .ui_root()
        .expect("ui")
        .borrow_mut()
        .close_session_tree();
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "/session-tree");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("树未重复打开")]
pub(crate) async fn t_ati28_no_tree_while_busy(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.tree_open, "busy MUST NOT open the tree");
    assert_eq!(s.tree_calls, 1, "no second tree fetch");
}

#[when("在 idle 提交斜杠 tree")]
pub(crate) async fn w_ati28_slash_tree_word(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.on_run_stream_closed();
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "/tree");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("无该动词路径且未开树且未调用 run")]
pub(crate) async fn t_ati28_tree_not_a_verb(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.tree_open);
    assert_eq!(s.tree_calls, 1);
    assert!(s.runs.is_empty(), "/tree must not fall through to run");
    assert!(
        !s.notices.is_empty(),
        "unknown verb surfaces a notice: {:?}",
        s.notices
    );
}

// ati30 ────────────────────────────────────────────────────────

#[when("以主机泵提交挂起 bang 并注入 Esc 加积压 Esc")]
pub(crate) async fn w_ati30_first_bang_esc_backlog(host_pump_bdd: &HostPumpBdd) {
    use crate::app::tui::harness::run_interactive_bang;
    let pump = fresh_pump_with_hanging_bash();
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "!sleep 1");
    step_key(host_pump_bdd, enter_event());
    drain(host_pump_bdd).await;
    let mut pump = take_pump(host_pump_bdd);
    let bash = pump.session.take_bash().expect("pending bang");
    let mut stream = None;
    run_interactive_bang(
        &mut pump.session,
        &mut pump.driver,
        bash,
        &mut stream,
        esc_stream(40, 2),
    )
    .await
    .expect("bang loop");
    put_pump(host_pump_bdd, pump);
}

#[when("再提交第二条挂起 bang 并注入 Esc")]
pub(crate) async fn w_ati30_second_bang_esc(host_pump_bdd: &HostPumpBdd) {
    use crate::app::tui::harness::run_interactive_bang;
    set_editor(host_pump_bdd, "!sleep 2");
    step_key(host_pump_bdd, enter_event());
    drain(host_pump_bdd).await;
    let mut pump = take_pump(host_pump_bdd);
    let bash = pump.session.take_bash().expect("second pending bang");
    let mut stream = None;
    run_interactive_bang(
        &mut pump.session,
        &mut pump.driver,
        bash,
        &mut stream,
        esc_stream(40, 0),
    )
    .await
    .expect("bang loop");
    put_pump(host_pump_bdd, pump);
}

#[then("第二次 bang 仍可被 Esc 中止且块为 cancelled")]
pub(crate) async fn t_ati30_second_still_abortable(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let entries: Vec<UiEntry> = pump.session.ui_model().entries.clone();
    let aborts = pump.driver.abort_count();
    let busy = pump.session.is_busy();
    put_pump(host_pump_bdd, pump);
    assert!(
        aborts >= 2,
        "suppress_idle_esc must not eat later aborts: {aborts}"
    );
    assert!(!busy);
    assert!(
        entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                command, status, ..
            } if command == "sleep 2" && *status == BashBlockStatus::Cancelled
        )),
        "second bang block Cancelled: {entries:?}"
    );
}

// ati31 ────────────────────────────────────────────────────────

#[when("以主机泵开启忙碌流注入正文增量后按下 Esc 再注入迟到增量")]
pub(crate) async fn w_ati31_late_xy(host_pump_bdd: &HostPumpBdd) {
    use crate::agent::runtime::XyEvent;
    open_busy(host_pump_bdd);
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta("draft".into()))))
        .expect("xy step");
    put_pump(host_pump_bdd, pump);
    step_key(host_pump_bdd, esc_event());
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(
            "SHOULD_NOT_APPEAR".into(),
        ))))
        .expect("late xy step");
    put_pump(host_pump_bdd, pump);
    drain(host_pump_bdd).await;
}

#[then("已流式正文保留且迟到增量不出现")]
pub(crate) async fn t_ati31_partial_kept(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let entries: Vec<UiEntry> = pump.session.ui_model().entries.clone();
    put_pump(host_pump_bdd, pump);
    assert!(
        entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text } if text.contains("draft")
        )),
        "c1595 partial must survive the abort: {entries:?}"
    );
    assert!(
        !entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text } if text.contains("SHOULD_NOT_APPEAR")
        )),
        "late delta must be suppressed: {entries:?}"
    );
}

#[then("abort 计一次且滚动提示为 aborted 语义")]
pub(crate) async fn t_ati31_abort_note(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 1, "abort still drains to Driver::abort");
    assert!(
        s.notices
            .iter()
            .any(|t| t == "Aborted" || t == "Operation aborted"),
        "abort semantics flushed: {:?}",
        s.notices
    );
}

// ati32 ────────────────────────────────────────────────────────

#[when("以主机泵开启忙碌流并提交 bang 前缀文本")]
pub(crate) async fn w_ati32_busy_bang_prefix(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    set_editor(host_pump_bdd, "!ls");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("有硬拒提示且编辑器保留文本")]
pub(crate) async fn t_ati32_hard_reject_notice(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.notices.iter().any(|t| t.contains("rejected")),
        "hard-reject notice expected: {:?}",
        s.notices
    );
    assert_eq!(
        editor_text(host_pump_bdd),
        "!ls",
        "editor keeps the bang text"
    );
}

#[then("未入 steer 且未调用 execute_bash")]
pub(crate) async fn t_ati32_no_steer_no_bash(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.steers.is_empty(),
        "literal bang text must not steer: {:?}",
        s.steers
    );
    assert!(
        s.bash_calls.is_empty(),
        "agent busy must not execute bang: {:?}",
        s.bash_calls
    );
}

// ati43 ────────────────────────────────────────────────────────

#[when("以主机泵开启重载并输入草稿后按 Enter")]
pub(crate) async fn w_ati43_reload_gate(host_pump_bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.session.begin_reload();
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "draft while reloading");
    step_key(host_pump_bdd, enter_event());
}

#[then("通知条为 reloading 且草稿保留且未新增滚动提示")]
pub(crate) async fn t_ati43_reload_gate_toast(host_pump_bdd: &HostPumpBdd) {
    let notices_before = stats(host_pump_bdd).notices.len();
    let frame = render_frame(host_pump_bdd, 120);
    assert!(
        frame.contains("reloading — wait"),
        "soft gate toast body `reloading — wait` visible: {frame}"
    );
    assert_eq!(
        editor_text(host_pump_bdd),
        "draft while reloading",
        "soft gate keeps the draft"
    );
    let s = stats(host_pump_bdd);
    assert!(s.reload_active, "reload still active");
    assert_eq!(
        s.notices.len(),
        notices_before,
        "soft gate MUST NOT append ScrollNotice"
    );
    assert!(s.runs.is_empty() && s.bash_calls.is_empty());
}

#[when("经输入流注入 Esc 取消挂起重载")]
pub(crate) async fn w_ati43_cancel_hanging_reload(host_pump_bdd: &HostPumpBdd) {
    use crate::XyDriverError;
    use crate::app::tui::harness::run_interactive_reload;
    // 结束软闸阶段，换成挂起重载再取消（c1205 形态）。
    let mut pump = take_pump(host_pump_bdd);
    pump.session.end_reload();
    pump.driver.set_hang_reload_until_cancel(true);
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "/reload");
    step_key(host_pump_bdd, enter_event());
    drain(host_pump_bdd).await;
    let mut pump = take_pump(host_pump_bdd);
    assert!(
        pump.session.take_reload(),
        "reload must be pending after /reload"
    );
    let input = futures::stream::iter(vec![
        Ok::<HostEvent, XyDriverError>(HostEvent::Tick),
        Ok(HostEvent::Input(esc_event())),
    ]);
    run_interactive_reload(&mut pump.session, &mut pump.driver, input)
        .await
        .expect("reload loop");
    put_pump(host_pump_bdd, pump);
}

#[then("重载结束且通告为已取消")]
pub(crate) async fn t_ati43_reload_cancelled(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.reload_active, "reload no longer active");
    assert!(
        s.notices.iter().any(|t| t.contains("Reload cancelled")),
        "cancel note expected: {:?}",
        s.notices
    );
}

#[when("重载结束后提交 bang 命令")]
pub(crate) async fn w_ati43_keys_restored(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.driver.push_bash_result(XyBashResult {
        output: "ok".into(),
        exit_code: Some(0),
        ..Default::default()
    });
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "!echo restored");
    submit_with(host_pump_bdd, enter_event()).await;
}

#[then("键位恢复 idle 规则且 execute_bash 收到命令体")]
pub(crate) async fn t_ati43_idle_rules_restored(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.bash_calls.iter().any(|(c, _)| c == "echo restored"),
        "after reload ends, bang runs again: {:?}",
        s.bash_calls
    );
}
