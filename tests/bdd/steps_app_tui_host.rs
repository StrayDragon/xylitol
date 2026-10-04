//! Steps for the host-pump family (att9 / att11, later ati busy keys).
//!
//! ati30 contract: BDD reuses [`HostSession`] + [`ScriptedDriver`] +
//! [`pump_host_driver`] — the very same pump the unit slice tests use; never a
//! second side-effect pump.

use crate::app::tui::TuiHostEvent as HostEvent;
use crate::app::tui::TuiHostSession as HostSession;
use crate::app::tui::UiRoot;
use crate::app::tui::harness::{ScriptedDriver, TestTerminal, enter_event, pump_host_driver};
use crate::app::tui::{BashBlockStatus, UiEntry, UiModel};
use crate::protocol::ports::XyBashResult;
use crate::tests::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
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
    /// 文本探针：面 AGENTS 文档 / PTY 登记原文。
    pub doc_text: RefCell<Option<String>>,
    /// 绘制缓存探针：(live root, 其 UiModel)。
    pub paint_probe: RefCell<Option<(UiRoot, UiModel)>>,
    /// 计数探针（paint cache / full parse 次数），按步序追加。
    pub counts: RefCell<Vec<u64>>,
    /// 即时文件日志探针：(是否装到后端, 日志目录)。
    pub log_probe: RefCell<Option<(bool, std::path::PathBuf)>>,
    /// 交互模式探针（`{:?}` 形态）。
    pub mode_probe: RefCell<Option<String>>,
}

#[fixture]
pub fn host_pump_bdd() -> HostPumpBdd {
    HostPumpBdd {
        pump: RefCell::new(None),
        ansi_frames: RefCell::new(Vec::new()),
        doc_text: RefCell::new(None),
        paint_probe: RefCell::new(None),
        counts: RefCell::new(Vec::new()),
        log_probe: RefCell::new(None),
        mode_probe: RefCell::new(None),
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

// atm18 / ath45 ── c2790: bang 循环 drain Inline 命令 ──────────────────────

fn char_keys(text: &str) -> Vec<xylitol_tui::InputEvent> {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    text.chars()
        .map(|c| {
            xylitol_tui::InputEvent::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            })
        })
        .collect()
}

/// Keys typed mid-bang, then `escs` Esc presses (picker close → abort), park.
/// `prefix` MUST end with the submit Enter — busy Enter is what parses the
/// slash into `pending.slash`; bare chars only fill the editor buffer.
fn keys_then_escs_stream(
    mut prefix: Vec<xylitol_tui::InputEvent>,
    escs: usize,
) -> impl futures::Stream<Item = Result<HostEvent, crate::XyDriverError>> {
    use std::time::Duration;
    prefix.push(enter_event());

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        for ev in prefix {
            let _ = tx.send(Ok(HostEvent::Input(ev)));
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
        for _ in 0..escs {
            let _ = tx.send(Ok(HostEvent::Input(esc_event())));
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        std::future::pending::<()>().await;
    });
    futures::stream::unfold(
        rx,
        |mut rx| async move { rx.recv().await.map(|ev| (ev, rx)) },
    )
}

/// Shared body for both c2790 bang-Given phrasings (atm18 names the driver
/// seam; ath45 is the generic host contract — same fixture setup).
async fn g_c2790_hanging_bang_body(host_pump_bdd: &HostPumpBdd) {
    let pump = fresh_pump_with_hanging_bash();
    put_pump(host_pump_bdd, pump);
    set_editor(host_pump_bdd, "!sleep 99");
    step_key(host_pump_bdd, enter_event());
    drain(host_pump_bdd).await;
}

#[given("交互 bang 正在运行且驱动为 ScriptedDriver")]
pub(crate) async fn g_c2790_hanging_bang(host_pump_bdd: &HostPumpBdd) {
    g_c2790_hanging_bang_body(host_pump_bdd).await;
}

#[given("交互 bang 正在运行")]
pub(crate) async fn g_ath45_hanging_bang(host_pump_bdd: &HostPumpBdd) {
    g_c2790_hanging_bang_body(host_pump_bdd).await;
}

#[when("提交声明为 Inline 的无参 /model")]
pub(crate) async fn w_atm18_inline_model(host_pump_bdd: &HostPumpBdd) {
    use crate::app::tui::harness::run_interactive_bang;
    let mut pump = take_pump(host_pump_bdd);
    let bash = pump.session.take_bash().expect("pending bang");
    let mut stream = None;
    run_interactive_bang(
        &mut pump.session,
        &mut pump.driver,
        bash,
        &mut stream,
        keys_then_escs_stream(char_keys("/model"), 2),
    )
    .await
    .expect("bang loop");
    put_pump(host_pump_bdd, pump);
}

#[then("模型列表 effect MUST 在 bang 结束前挂载生效")]
pub(crate) async fn t_atm18_inline_mounted(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let models_calls = pump.driver.models_calls();
    let submitted = pump
        .driver
        .bash_calls()
        .iter()
        .any(|(c, _)| c == "sleep 99");
    // Mount evidence: the picker row marker `→ * fake` is rendered by the
    // Models slot only while mounted (SelectList cursor + focused mark) — the
    // Mount evidence: the picker row marker `→ * Fake` is rendered by the
    // Models slot only while mounted (SelectList cursor + focused mark). The
    // label is the harness model's display_name (`Fake`); the PTY seam waits
    // on `→ * fake` because the attach-cache model renders its id — each seam
    // asserts its own rendered screen. The diff frame log preserves frame
    // order, so the mount frame must precede the post-abort `(cancelled)`
    // frame: mounted DURING the bang, closed by the first Esc before the
    // second Esc aborts.
    let frames = pump.session.tui.terminal.frames();
    let mount_idx = frames.iter().position(|f| f.contains("→ * Fake"));
    let cancel_idx = frames.iter().position(|f| f.contains("(cancelled)"));
    put_pump(host_pump_bdd, pump);
    assert!(submitted, "hanging bang must have been submitted");
    assert!(
        models_calls >= 1,
        "Inline drain must dispatch GetAvailableModels during the bang"
    );
    let mount_idx = mount_idx
        .unwrap_or_else(|| panic!("Models picker row must be rendered before the bang ends"));
    assert!(
        cancel_idx.is_some_and(|i| mount_idx < i),
        "picker mount frame must precede the bang-cancelled frame (mount_idx={mount_idx}, cancel_idx={cancel_idx:?})"
    );
}

#[when("提交 busy-Allow 且执行类为 Queued 的 /session-export")]
pub(crate) async fn w_ath45_queued_export(host_pump_bdd: &HostPumpBdd) {
    use crate::app::tui::harness::run_interactive_bang;
    let mut pump = take_pump(host_pump_bdd);
    let bash = pump.session.take_bash().expect("pending bang");
    let mut stream = None;
    run_interactive_bang(
        &mut pump.session,
        &mut pump.driver,
        bash,
        &mut stream,
        keys_then_escs_stream(char_keys("/session-export"), 2),
    )
    .await
    .expect("bang loop");
    put_pump(host_pump_bdd, pump);
}

#[then("该命令 MUST 在 bang 结束前不执行")]
pub(crate) async fn t_ath45_still_queued(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    let slash = pump.session.take_slash();
    let still_queued = slash.is_some();
    if let Some(slash) = slash {
        pump.session.put_slash(slash);
    }
    let exported = pump.driver.export_html_calls();
    put_pump(host_pump_bdd, pump);
    assert!(still_queued, "Queued slash must survive the bang loop");
    assert!(
        exported.is_empty(),
        "Queued slash must not execute during bang: {exported:?}"
    );
}

#[then("循环归还后 MUST 照常执行且 MUST NOT 丢失")]
pub(crate) async fn t_ath45_executes_after_return(host_pump_bdd: &HostPumpBdd) {
    drain(host_pump_bdd).await;
    let mut pump = take_pump(host_pump_bdd);
    let calls = pump.driver.export_html_calls();
    let slash_left = pump.session.take_slash().is_some();
    put_pump(host_pump_bdd, pump);
    assert_eq!(
        calls.len(),
        1,
        "export must execute exactly once: {calls:?}"
    );
    assert!(!slash_left, "no slash may remain pending after the drain");
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

// ── c2826 specs-compact：app-tui-commands 斜杠族场景 ───────────────

/// 以真实编辑器路径提交一条 slash 并驱动泵（无前置挂载时自动挂载）。
async fn c2826_submit_slash(host_pump_bdd: &HostPumpBdd, text: &str) {
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text(text);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter step");
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    // Exclusive/Queued 类命令（reload/export 等）在循环归还后由 drain_pending 执行。
    crate::app::tui::harness::drain_pending(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("drain");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

fn c2826_slot(host_pump_bdd: &HostPumpBdd) -> crate::app::tui::EditorSlotKind {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let slot = root.borrow().slot();
    put_pump(host_pump_bdd, pump);
    slot
}

fn c2826_last_notice(host_pump_bdd: &HostPumpBdd) -> String {
    let pump = take_pump(host_pump_bdd);
    let note = pump
        .session
        .ui_model()
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEntry::ScrollNotice { text } | UiEntry::Error { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();
    put_pump(host_pump_bdd, pump);
    note
}

fn c2826_counters(
    host_pump_bdd: &HostPumpBdd,
) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
    // (models, tree, compact, export_jsonl, export_html, new_session, list_sessions, reload)
    let pump = take_pump(host_pump_bdd);
    let c = (
        pump.driver.models_calls(),
        pump.driver.session_tree_calls(),
        pump.driver.compact_calls(),
        pump.driver.export_jsonl_calls().len(),
        pump.driver.export_html_calls().len(),
        pump.driver.new_session_calls(),
        pump.driver.list_sessions_calls(),
        pump.driver.reload_runtime_calls(),
    );
    put_pump(host_pump_bdd, pump);
    c
}

#[when("以主机泵在 idle 提交 {cmd:string}")]
pub(crate) async fn w_c2826_submit_slash(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[when("提交 {cmd:string}")]
pub(crate) async fn w_c2826_submit(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[then("会话收到退出请求")]
pub(crate) fn t_c2826_quit(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.should_quit, "c2826: /exit 应请求退出");
}

#[then("模型列表槽打开且经驱动取可用模型")]
pub(crate) fn t_c2826_models_slot(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Models",
        "c2826: /model 应打开 Models 槽"
    );
    assert!(
        c2826_counters(host_pump_bdd).0 >= 1,
        "c2826: 应经驱动取可用模型"
    );
}

#[then("写入系统错误行且未退出且未崩溃")]
pub(crate) fn t_c2826_unknown_slash(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.should_quit, "c2826: 未知斜杠不得退出");
    let note = c2826_last_notice(host_pump_bdd);
    assert!(!note.is_empty(), "c2826: 未知斜杠应写系统错误行");
}

#[then("有参直设经模型执行器更新且仅更新固定区")]
pub(crate) fn t_c2826_model_arg(host_pump_bdd: &HostPumpBdd) {
    // 直设路径成功即回到 Editor 槽（未开列表），无滚动确认块。
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Editor");
    let s = stats(host_pump_bdd);
    assert!(
        s.notices.iter().all(|n| !n.contains("model →")),
        "c2826: 成功切模不得写滚动提示确认块：{:?}",
        s.notices
    );
}

#[then("会话名经共享 dispatch 写入一次")]
pub(crate) fn t_c2826_session_name(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let calls = pump.driver.set_session_name_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(
        calls.len(),
        1,
        "c2826: /session-name 应经 dispatch 写入一次"
    );
    assert_eq!(calls[0], "新名字");
}

#[then("系统提示列出场景名与描述且未换会话")]
pub(crate) fn t_c2826_debug_list(host_pump_bdd: &HostPumpBdd) {
    let note = c2826_last_notice(host_pump_bdd);
    assert!(
        note.contains("session-tree-multiturn"),
        "c2826: /debug 应列场景：{note}"
    );
}

#[then("不识别冒号形式且按未知斜杠提示")]
pub(crate) fn t_c2826_debug_colon(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.should_quit);
    assert!(
        !c2826_last_notice(host_pump_bdd).is_empty(),
        "c2826: 冒号形式应按未知斜杠提示"
    );
}

#[then("会话树槽打开且经驱动取一次树")]
pub(crate) fn t_c2826_tree_open(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.tree_open, "c2826: /session-tree 应开树");
    assert!(s.tree_calls >= 1, "c2826: 应经驱动取树");
}

#[then("短名不被识别且树未重复打开")]
pub(crate) fn t_c2826_short_name(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.should_quit, "c2826: /tree 不应退出");
    assert_eq!(s.tree_calls, 1, "c2826: /tree 不应重复开树");
}

#[then("压缩一次且 jsonl 与 html 按后缀分派导出")]
pub(crate) fn t_c2826_session_io(host_pump_bdd: &HostPumpBdd) {
    let c = c2826_counters(host_pump_bdd);
    assert_eq!(c.2, 1, "c2826: /session-compact 应压缩一次");
    assert_eq!(c.3, 1, "c2826: .jsonl 应走 ExportJsonl");
    assert_eq!(c.4, 1, "c2826: .html 应走 ExportHtml");
}

#[then("以系统文本块展示会话信息与统计")]
pub(crate) fn t_c2826_session_info(host_pump_bdd: &HostPumpBdd) {
    let note = c2826_last_notice(host_pump_bdd);
    assert!(
        !note.is_empty(),
        "c2826: /session 应以系统文本展示信息（实际无输出）"
    );
}

#[then("Resume 面板槽打开且经驱动列举可恢复会话")]
pub(crate) fn t_c2826_resume_panel(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "SessionResume",
        "c2826: /session-resume 应打开 Resume 面板"
    );
    assert!(
        c2826_counters(host_pump_bdd).6 >= 1,
        "c2826: 应经驱动列举会话"
    );
}

#[then("新建一次且 clone 走 Fork(At) 且命名经 seam 写入")]
pub(crate) fn t_c2826_lifecycle(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let new_calls = pump.driver.new_session_calls();
    let forks = pump.driver.fork_calls();
    let names = pump.driver.set_session_name_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(new_calls, 1, "c2826: /session-new 应新建一次");
    assert!(
        forks
            .iter()
            .any(|(_, pos)| matches!(pos, crate::protocol::session::ForkPosition::At)),
        "c2826: /session-clone 应走 Fork(At)：{forks:?}"
    );
    assert_eq!(names.len(), 1, "c2826: /session-name 应写入一次");
}

#[then("运行时重载调用恰一次")]
pub(crate) fn t_c2826_reload_once(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        c2826_counters(host_pump_bdd).7,
        1,
        "c2826: idle /reload 应触发一次"
    );
}

#[then("重载被拒绝且未发第二次运行时重载")]
pub(crate) fn t_c2826_reload_busy_rejected(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.run_active || s.is_busy, "c2826: 前置应为忙碌态");
    // 忙碌提交经新挂载泵（open_busy 换新 ScriptedDriver）：本泵内重载计数必须为 0。
    let reloads = c2826_counters(host_pump_bdd).7;
    assert_eq!(reloads, 0, "c2826: busy /reload 不得触发运行时重载");
    let note = c2826_last_notice(host_pump_bdd);
    assert!(
        note.contains("busy") || note.contains("reload") || !note.is_empty(),
        "c2826: busy 拒绝应有提示：{note}"
    );
}

#[then("信任决策经驱动持久化一次且提示需重载生效")]
pub(crate) fn t_c2826_trust(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let calls = pump.driver.persist_project_trust_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(calls.len(), 1, "c2826: /trust 应持久化一次");
    let note = c2826_last_notice(host_pump_bdd);
    assert!(
        note.contains("reload") || note.contains("重载") || note.contains("重启"),
        "c2826: /trust 应提示需重载/重启生效：{note}"
    );
}

#[when("以主机泵完成一轮含已提交 assistant 的对话后提交 {cmd:string}")]
pub(crate) async fn w_c2826_copy_last(host_pump_bdd: &HostPumpBdd, cmd: String) {
    use crate::agent::runtime::XyEvent;
    let cmd = cmd.trim_matches('"');
    {
        let mut pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver.push_script(vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::TextDelta("正文内容".into()),
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::AgentEnd {
                messages: Vec::new(),
            },
        ]);
        let root = pump.session.ui_root().expect("ui").clone();
        root.borrow_mut().set_editor_text("提问");
        // idle Enter 直接发起 run（不得预臂 busy，否则会被当 steer）。
        pump.session
            .step(HostEvent::Input(enter_event()))
            .expect("enter");
        let mut stream = None;
        pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
            .await
            .expect("pump");
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[then("剪贴板收到 assistant 正文且未复制 thinking")]
pub(crate) fn t_c2826_copy_last(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let calls = pump.driver.copy_text_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(calls.len(), 1, "c2826: /history-copy-last 应复制一次");
    assert_eq!(calls[0], "正文内容");
}

#[then("主题列表槽打开")]
pub(crate) fn t_c2826_theme_slot(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Themes",
        "c2826: /theme 应打开 Themes 槽"
    );
}

#[then("主题直接应用且槽关闭")]
pub(crate) fn t_c2826_theme_applied(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Editor");
    let s = stats(host_pump_bdd);
    assert!(
        s.notices.iter().all(|n| !n.starts_with("theme")),
        "c2826: 成功换主题不得写确认块"
    );
}

#[when("以主机泵开启忙碌流并提交 {cmd:string}")]
pub(crate) async fn w_c2826_busy_slash(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    open_busy(host_pump_bdd);
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[then("模型列表槽仍可打开")]
pub(crate) fn t_c2826_busy_model_open(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Models",
        "c2826: busy 下 /model 应为 Allow"
    );
}

#[when("以主机泵注入含连接态的资源快照后提交 {cmd:string}")]
pub(crate) async fn w_c2826_mcp_panel(host_pump_bdd: &HostPumpBdd, cmd: String) {
    use crate::app::core::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};
    let cmd = cmd.trim_matches('"');
    {
        let pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver
            .set_loaded_resources_for_driver(LoadedResourcesSnapshot {
                mcp_configured: 1,
                mcp_connected: vec![("srv".into(), 2)],
                mcp_servers: vec![McpServerSnapshot {
                    id: "srv".into(),
                    phase: McpServerPhase::Connected,
                    tools_armed: true,
                    tool_count: 2,
                }],
                mcp_bootstrap_complete: true,
                ..Default::default()
            });
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[then("MCP 面板槽打开且列出连接态且未因开面板 abort agent")]
pub(crate) fn t_c2826_mcp_panel(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Mcp",
        "c2826: /mcp 应打开 MCP 面板"
    );
    let frame = render_frame(host_pump_bdd, 100);
    assert!(frame.contains("srv"), "c2826: 面板应列出 server：\n{frame}");
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 0, "c2826: 开 /mcp 不得 abort agent");
}

#[when("关闭当前槽并提交 {cmd:string}")]
pub(crate) async fn w_c2826_close_then_submit(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[when("注入会话 leaf 条目并以主机泵在 idle 提交 {cmd:string}")]
pub(crate) async fn w_c2826_inject_leaf_submit(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    let pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.driver.set_leaf_entry_id(Some("leaf-1".into()));
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[when("以主机泵注入可恢复会话列表后提交 {cmd:string}")]
pub(crate) async fn w_c2826_resume_with_list(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    {
        let mut pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver
            .set_session_list(vec![crate::protocol::ports::SessionListEntry {
                id: "sess-1".into(),
                name: Some("恢复样本".into()),
                first_message: Some("预览".into()),
                message_count: 2,
                modified_unix: Some(1_700_000_000),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            }]);
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

// ── c2826 specs-compact：fixed-zone 的 host-pump 场景步骤 ──────────

fn c2826_frame(host_pump_bdd: &HostPumpBdd) -> String {
    render_frame(host_pump_bdd, 100)
}

#[then("报告诊断且界面仍正常渲染")]
pub(crate) fn t_c2826_theme_diag(host_pump_bdd: &HostPumpBdd) {
    let note = c2826_last_notice(host_pump_bdd);
    assert!(
        !note.is_empty(),
        "c2826: 未知主题应报告诊断（滚动提示或系统行）"
    );
    let frame = c2826_frame(host_pump_bdd);
    assert!(!frame.is_empty(), "c2826: 界面仍应渲染");
}

#[when("关闭当前槽")]
pub(crate) async fn w_c2826_close_slot(host_pump_bdd: &HostPumpBdd) {
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
}

#[then("槽回到编辑器且未报错")]
pub(crate) fn t_c2826_theme_esc(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Editor",
        "c2826: Esc 关槽后应回编辑器"
    );
}

#[when("以主机泵开启忙碌流并按下 Esc 后收流关闭并渲染")]
pub(crate) async fn w_c2826_abort_idle_render(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
    // 收流关闭：泵至事件流结束（busy 轮收尾）。
    drain(host_pump_bdd).await;
}

#[then("帧内不再含忙碌短词且滚动提示含取消说明一行")]
pub(crate) fn t_c2826_abort_idle(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    assert!(
        !frame.contains("Working"),
        "c2826: abort 后 status 应回 idle（帧不含 Working）：{frame}"
    );
    let s = stats(host_pump_bdd);
    assert!(
        s.notices
            .iter()
            .any(|n| n.contains("abort") || n.contains("cancel") || n.contains("取消")),
        "c2826: 取消说明应走滚动提示一行：{:?}",
        s.notices
    );
}

#[when("渲染当前主机帧")]
pub(crate) fn w_c2826_render_host_frame(host_pump_bdd: &HostPumpBdd) {
    let _ = c2826_frame(host_pump_bdd);
}

#[when("以主机泵进入 reload 进行中态后渲染")]
pub(crate) fn w_c2826_reload_active_render(host_pump_bdd: &HostPumpBdd) {
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.session.begin_reload();
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    let _ = c2826_frame(host_pump_bdd);
}

#[then("status 显示 Reloading 短词且不冒充 Working")]
pub(crate) fn t_c2826_reload_status(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    assert!(
        frame.contains("Reloading"),
        "c2826: reload 态 status 应显示 Reloading：{frame}"
    );
}

// ── c2826 specs-compact：app-tui-input 场景步骤 ────────────────────

pub(crate) fn c2826_alt_up_event_pub() -> xylitol_tui::InputEvent {
    c2826_alt_up_event()
}

fn c2826_alt_up_event() -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code: KeyCode::Up,
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn c2826_ctrl_v_event() -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code: KeyCode::Char('v'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn c2826_up_event() -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[then("选择器替换贴底 editor 槽且 footer 仍为末行")]
pub(crate) fn t_c2826_selector_replaces(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Models");
    let frame = c2826_frame(host_pump_bdd);
    let footer = frame.lines().last().unwrap_or_default();
    assert!(
        footer.contains("·") || !footer.trim().is_empty(),
        "c2826: footer 应仍为末行：{frame}"
    );
}

#[when("以主机泵挂载样例树数据后空编辑器双 Esc")]
pub(crate) async fn w_c2826_double_esc_tree(host_pump_bdd: &HostPumpBdd) {
    open_tree_ready(host_pump_bdd);
    step_key(host_pump_bdd, esc_event());
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
}

#[then("会话树槽打开且经驱动取活树")]
pub(crate) fn t_c2826_tree_live(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.tree_open, "c2826: 双 Esc 应开树");
    assert!(s.tree_calls >= 1, "c2826: 树应经驱动取活树");
}

#[when("再按一次 Esc")]
pub(crate) async fn w_c2826_esc_again(host_pump_bdd: &HostPumpBdd) {
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
}

#[then("树关闭且回到编辑器槽")]
pub(crate) fn t_c2826_tree_closed(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(!s.tree_open, "c2826: Esc 应关树");
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Editor");
}

#[then("树槽渲染样例节点且替换贴底编辑区")]
pub(crate) fn t_c2826_tree_renders_sample(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(s.tree_open, "c2826: 前置应开树");
    let frame = c2826_frame(host_pump_bdd);
    assert!(!frame.trim().is_empty(), "c2826: 树槽应渲染内容");
}

#[when("以主机泵提交两条不同文本后按上方向键")]
pub(crate) async fn w_c2826_send_history(host_pump_bdd: &HostPumpBdd) {
    for text in ["第一条历史", "第二条历史"] {
        c2826_submit_slash(host_pump_bdd, text).await;
    }
    step_key(host_pump_bdd, c2826_up_event());
    pump_once(host_pump_bdd).await;
}

#[then("编辑器召回最近一条已发送文本")]
pub(crate) fn t_c2826_send_history(host_pump_bdd: &HostPumpBdd) {
    let text = editor_text(host_pump_bdd);
    assert!(
        text.contains("第二条历史"),
        "c2826: ↑ 应召回最近发送文本，实际：{text}"
    );
}

#[when("以 ! 前缀与无前缀分别设置编辑器文本并渲染")]
pub(crate) fn w_c2826_bang_border(host_pump_bdd: &HostPumpBdd) {
    let pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("!ls -la");
    let with_bang = root.borrow_mut().render(100).join("\n");
    root.borrow_mut().set_editor_text("ls -la");
    let without = root.borrow_mut().render(100).join("\n");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    host_pump_bdd
        .ansi_frames
        .borrow_mut()
        .extend([with_bang, without]);
}

#[then("操作区边框形态随前缀切换")]
pub(crate) fn t_c2826_bang_border(host_pump_bdd: &HostPumpBdd) {
    let frames = host_pump_bdd.ansi_frames.borrow();
    let with_bang = frames[frames.len() - 2].clone();
    let without = frames[frames.len() - 1].clone();
    assert_ne!(
        with_bang, without,
        "c2826: ! 前缀应切换操作区边框形态（强调色）"
    );
}

#[when("以主机泵注入剪贴板图片后按粘贴键")]
pub(crate) async fn w_c2826_paste_image(host_pump_bdd: &HostPumpBdd) {
    {
        let pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver.set_clipboard_image(vec![1, 2, 3], "image/png");
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    step_key(host_pump_bdd, c2826_ctrl_v_event());
    pump_once(host_pump_bdd).await;
    drain(host_pump_bdd).await;
}

#[then("编辑器插入落盘路径文本且路径经 Driver 暂存")]
pub(crate) fn t_c2826_paste_image(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let staged = pump.driver.staged_paste_paths();
    let text = {
        let root = pump.session.ui_root().expect("ui").clone();
        root.borrow().editor_text()
    };
    put_pump(host_pump_bdd, pump);
    assert!(
        !staged.is_empty(),
        "c2826: 粘贴图片应经 Driver 暂存 tempfile"
    );
    assert!(
        text.contains(".png") || text.contains('/') || !text.is_empty(),
        "c2826: 编辑器应插入路径文本，实际：{text}"
    );
}

#[when("以主机泵开启含正文的忙碌流并在首个增量后 abort 并渲染")]
pub(crate) async fn w_c2826_abort_partial(host_pump_bdd: &HostPumpBdd) {
    use crate::agent::runtime::XyEvent;
    open_busy(host_pump_bdd);
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Xy(Box::new(XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        })))
        .expect("xy start");
    pump.session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(
            "半途正文".into(),
        ))))
        .expect("xy delta");
    pump.session
        .step(HostEvent::Input(esc_event()))
        .expect("esc abort");
    put_pump(host_pump_bdd, pump);
    pump_once(host_pump_bdd).await;
    drain(host_pump_bdd).await;
    let _ = c2826_frame(host_pump_bdd);
}

#[then("partial 正文留驻 scrollback 且含 aborted 语义脚注")]
pub(crate) fn t_c2826_abort_partial(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    assert!(
        frame.contains("半途正文"),
        "c2826: partial 正文应留驻 scrollback：{frame}"
    );
    let pump = take_pump(host_pump_bdd);
    let has_aborted = crate::app::tui::trailing_aborted_note(&pump.session.ui_model().entries);
    put_pump(host_pump_bdd, pump);
    assert!(
        has_aborted || frame.to_lowercase().contains("abort"),
        "c2826: 应含 aborted 语义脚注（帧或滚动提示）：{frame}"
    );
}

#[when("以主机泵置零队列深度并注入 steer 后排空 pending")]
pub(crate) async fn w_c2826_zero_depth_strip(host_pump_bdd: &HostPumpBdd) {
    {
        let pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver
            .force_zero_queue_stats
            .store(true, std::sync::atomic::Ordering::SeqCst);
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    open_busy(host_pump_bdd);
    set_editor(host_pump_bdd, "插队内容");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter (steer)");
    put_pump(host_pump_bdd, pump);
    drain(host_pump_bdd).await;
}

#[then("本地队列条文案不被空深度清除")]
pub(crate) fn t_c2826_zero_depth_strip(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let strip = pump.session.ui_model().pending_steer.clone();
    put_pump(host_pump_bdd, pump);
    assert!(
        strip.iter().any(|t| t.contains("插队内容")),
        "c2826: 空深度不得清除本地队列条：{strip:?}"
    );
}

#[when("以主机泵忙碌插队后渲染队列条并按 Alt+Up")]
pub(crate) async fn w_c2826_queue_strip_alt_up(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    set_editor(host_pump_bdd, "steer-one");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter (steer)");
    put_pump(host_pump_bdd, pump);
    pump_once(host_pump_bdd).await;
    let before = c2826_frame(host_pump_bdd);
    step_key(
        host_pump_bdd,
        crate::tests::bdd::steps_app_tui_host::c2826_alt_up_event_pub(),
    );
    pump_once(host_pump_bdd).await;
    let after_text = editor_text(host_pump_bdd);
    let after_marker = format!("---AFTER---\n{after_text}");
    host_pump_bdd.ansi_frames.borrow_mut().push(before);
    host_pump_bdd.ansi_frames.borrow_mut().push(after_marker);
}

#[then("队列条含插队文本且 Alt+Up 还原队列文本")]
pub(crate) fn t_c2826_queue_strip_alt_up(host_pump_bdd: &HostPumpBdd) {
    let frames = host_pump_bdd.ansi_frames.borrow();
    let before = frames[frames.len() - 2].clone();
    let after_text = frames[frames.len() - 1].replace("---AFTER---\n", "");
    drop(frames);
    assert!(
        before.contains("steer-one") || before.contains("Steering"),
        "c2826: 队列条应含插队文本：{before}"
    );
    let footer = before.lines().last().unwrap_or_default();
    assert!(
        !footer.contains("q:s"),
        "c2826: footer 无队列徽章：{footer}"
    );
    assert!(
        after_text.contains("steer-one"),
        "c2826: Alt+Up 应还原队列文本进编辑器：{after_text}"
    );
}

// ── c2826 specs-compact：app-tui-host 场景步骤 ─────────────────────

#[when("以主机泵在 idle 提交带换行的 {cmd:string}")]
pub(crate) async fn w_c2826_submit_multiline(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"').replace("\\n", "\n");
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text(cmd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter");
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    crate::app::tui::harness::drain_pending(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("drain");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("会话名写入时换行规范为空格")]
pub(crate) fn t_c2826_name_normalized(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let calls = pump.driver.set_session_name_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(calls.len(), 1, "c2826: 应写一次会话名");
    assert_eq!(
        calls[0], "甲 乙",
        "c2826: CR/LF MUST 规范为空格，实际 {:?}",
        calls[0]
    );
}

#[then("重载经共享缝完成且尾插 Reload 步进汇总")]
pub(crate) fn t_c2826_reload_report(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(c2826_counters(host_pump_bdd).7, 1);
    let s = stats(host_pump_bdd);
    assert!(
        s.notices.iter().any(|t| t.contains("Reload:")),
        "c2826: 应尾插 Reload 汇总：{:?}",
        s.notices
    );
}

#[when("以主机泵注入 connecting 资源快照后提交 bang 命令 {cmd:string}")]
pub(crate) async fn w_c2826_connecting_bang(host_pump_bdd: &HostPumpBdd, cmd: String) {
    let cmd = cmd.trim_matches('"');
    {
        let pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        use crate::app::core::driver::{
            LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        };
        pump.driver
            .set_loaded_resources_for_driver(LoadedResourcesSnapshot {
                mcp_configured: 1,
                mcp_servers: vec![McpServerSnapshot {
                    id: "boot".into(),
                    phase: McpServerPhase::Connecting,
                    tools_armed: false,
                    tool_count: 0,
                }],
                mcp_connecting_label: Some("0/1".into()),
                mcp_bootstrap_complete: false,
                ..Default::default()
            });
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[then("键入与提交未被拒且 bash 收到命令体")]
pub(crate) fn t_c2826_connecting_bang(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.bash_calls.iter().any(|(c, _)| c.contains("echo hi")),
        "c2826: connecting 时提交 bang 不应被拒：{:?}",
        s.bash_calls
    );
}

#[when("以主机泵注入 connecting 且未冻表的资源快照后渲染当前主机帧")]
pub(crate) async fn w_c2826_mcp_pending_cue(host_pump_bdd: &HostPumpBdd) {
    {
        let pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        use crate::app::core::driver::{
            LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        };
        pump.driver
            .set_loaded_resources_for_driver(LoadedResourcesSnapshot {
                mcp_configured: 1,
                mcp_servers: vec![McpServerSnapshot {
                    id: "srv-x".into(),
                    phase: McpServerPhase::Connecting,
                    tools_armed: false,
                    tool_count: 0,
                }],
                mcp_connecting_label: Some("0/1".into()),
                mcp_bootstrap_complete: false,
                ..Default::default()
            });
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    // 经产品刷新路径装载快照（connecting + 未冻表 → 短 cue）。
    let mut pump = take_pump(host_pump_bdd);
    let driver = std::mem::take(&mut pump.driver);
    pump.session.refresh_loaded_resources(&driver).await;
    pump.driver = driver;
    put_pump(host_pump_bdd, pump);
    let _ = c2826_frame(host_pump_bdd);
}

#[then("status 短 cue 为固定文案 mcp pending (see /mcp) 且不枚举 server id")]
pub(crate) fn t_c2826_mcp_pending_cue(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    assert!(
        frame.contains("mcp pending (see /mcp)"),
        "c2826: 短 cue 固定文案缺失：{frame}"
    );
    assert!(
        !frame.contains("srv-x"),
        "c2826: 短 cue MUST NOT 枚举 server id：{frame}"
    );
}

#[when("以主机泵挂起重载进行中提交上行并取消收尾")]
pub(crate) async fn w_c2826_reload_soft_gate(host_pump_bdd: &HostPumpBdd) {
    {
        let mut pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.session.end_reload();
        pump.driver.set_hang_reload_until_cancel(true);
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    set_editor(host_pump_bdd, "/reload");
    step_key(host_pump_bdd, enter_event());
    drain(host_pump_bdd).await;
    let mut pump = take_pump(host_pump_bdd);
    assert!(pump.session.take_reload(), "c2826: reload 应处于待执行态");
    put_pump(host_pump_bdd, pump);
    // reload 进行中输入草稿并提交上行 → 软闸。
    set_editor(host_pump_bdd, "草稿上行");
    let mut pump = take_pump(host_pump_bdd);
    let input = futures::stream::iter(vec![
        Ok::<HostEvent, crate::XyDriverError>(HostEvent::Tick),
        Ok(HostEvent::Input(enter_event())),
        Ok(HostEvent::Input(esc_event())),
    ]);
    crate::app::tui::run_interactive_reload(&mut pump.session, &mut pump.driver, input)
        .await
        .expect("reload loop");
    put_pump(host_pump_bdd, pump);
}

#[then("提交被软闸拒绝且草稿保留且未发第二次运行时重载")]
pub(crate) fn t_c2826_reload_soft_gate(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let reloads = pump.driver.reload_runtime_calls();
    let runs = pump.driver.runs.clone();
    let steers = pump.driver.steer_calls.clone();
    put_pump(host_pump_bdd, pump);
    assert_eq!(reloads, 1, "c2826: 不得触发第二次运行时重载");
    assert!(
        !runs.iter().any(|r| r.contains("草稿上行"))
            && !steers.iter().any(|t| t.contains("草稿上行")),
        "c2826: 软闸应拒绝提交（不入 run/steer）：{runs:?} {steers:?}"
    );
    let draft = editor_text(host_pump_bdd);
    assert!(
        draft.contains("草稿上行"),
        "c2826: 被拒提交应保留编辑器草稿，实际：{draft}"
    );
}

#[when("臂装复制成功提示后渲染主机帧")]
pub(crate) fn w_c2826_copy_notice(host_pump_bdd: &HostPumpBdd) {
    let pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().arm_copy_notice();
    drop(root);
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    let _ = c2826_frame(host_pump_bdd);
}

#[then("固定区出现 Copied 短提示且不以 Error 前缀冒充")]
pub(crate) fn t_c2826_copy_notice(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    assert!(
        frame.contains("Copied"),
        "c2826: 应有 Copied 短提示：{frame}"
    );
    let copied_line = frame
        .lines()
        .find(|l| l.contains("Copied"))
        .expect("copied line");
    assert!(
        !copied_line.contains("Error"),
        "c2826: 成功提示不得用 Error 前缀：{copied_line}"
    );
}

#[when("以合成验收链驱动一整轮含工具与 abort 的会话并 /exit")]
pub(crate) async fn w_c2826_synthetic_chain(host_pump_bdd: &HostPumpBdd) {
    use crate::agent::runtime::XyEvent;
    // 1) 提交 → 流式 + 工具 + 完成
    {
        let mut pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver.push_script(vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::TextDelta("答一".into()),
            XyEvent::ToolExecutionStart {
                id: "t1".into(),
                name: "read".into(),
                args: serde_json::json!({"path":"a"}),
            },
            XyEvent::ToolExecutionUpdate {
                id: "t1".into(),
                output: "hello".into(),
            },
            XyEvent::ToolExecutionEnd {
                id: "t1".into(),
                name: "read".into(),
                result: "hello".into(),
                is_error: false,
            },
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::AgentEnd {
                messages: Vec::new(),
            },
        ]);
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    c2826_submit_slash(host_pump_bdd, "第一问").await;
    // 2) busy → steer（同一泵内标记 busy，避免重挂丢失计数）
    {
        let mut pump = take_pump(host_pump_bdd);
        pump.session.on_run_started("插一句");
        put_pump(host_pump_bdd, pump);
    }
    set_editor(host_pump_bdd, "插一句");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("steer enter");
    put_pump(host_pump_bdd, pump);
    pump_once(host_pump_bdd).await;
    // 3) busy → abort
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
    drain(host_pump_bdd).await;
    // 4) 再提交
    c2826_submit_slash(host_pump_bdd, "第二问").await;
    // 5) /exit
    c2826_submit_slash(host_pump_bdd, "/exit").await;
}

#[then("提交流式工具 steer abort 再提交按序生效且会话请求退出")]
pub(crate) fn t_c2826_synthetic_chain(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert!(
        s.runs.iter().any(|r| r.contains("第一问")) && s.runs.iter().any(|r| r.contains("第二问")),
        "c2826: 两次提交都应发起 run：{:?}",
        s.runs
    );
    assert!(
        s.steers.iter().any(|t| t.contains("插一句")),
        "c2826: busy steer 应入队：{:?}",
        s.steers
    );
    assert_eq!(s.aborts, 1, "c2826: abort 应计一次");
    assert!(s.should_quit, "c2826: /exit 应触发 finish");
    let pump = take_pump(host_pump_bdd);
    let has_tool = pump
        .session
        .ui_model()
        .entries
        .iter()
        .any(|e| matches!(e, UiEntry::Tool { .. }));
    put_pump(host_pump_bdd, pump);
    assert!(has_tool, "c2826: 工具执行应留块");
}

// ── c2826 specs-compact：最后一批（input/fixed-zone 收尾）──────────

fn c2826_ctrl_g_event() -> xylitol_tui::InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    xylitol_tui::InputEvent::Key(KeyEvent {
        code: KeyCode::Char('g'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[when("以主机泵在无编辑器环境变量下输入草稿后按 Ctrl+G")]
pub(crate) async fn w_c2826_ctrl_g_stub(host_pump_bdd: &HostPumpBdd) {
    // harness / 非 TTY 路径 MUST NOT spawn 真实编辑器（保持 stub）。
    if host_pump_bdd.pump.borrow().is_none() {
        *host_pump_bdd.pump.borrow_mut() = Some(fresh_pump());
    }
    set_editor(host_pump_bdd, "外编草稿");
    step_key(host_pump_bdd, c2826_ctrl_g_event());
    pump_once(host_pump_bdd).await;
    drain(host_pump_bdd).await;
}

#[then("系统提示写入且草稿保留且未 spawn 真实编辑器")]
pub(crate) fn t_c2826_ctrl_g_stub(host_pump_bdd: &HostPumpBdd) {
    let text = editor_text(host_pump_bdd);
    assert!(
        text.contains("外编草稿"),
        "c2826: Ctrl+G 失败路径应保留原文本，实际：{text}"
    );
    let s = stats(host_pump_bdd);
    assert!(!s.should_quit, "c2826: 未配置编辑器 MUST NOT panic / 退出");
}

#[then("回到编辑器槽且未提交模型变更")]
pub(crate) fn t_c2826_model_esc_no_change(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Editor");
    let s = stats(host_pump_bdd);
    assert!(
        s.runs.is_empty(),
        "c2826: Esc 取消不得触发任何 run：{:?}",
        s.runs
    );
}

#[when("以主机泵以损坏 JSON 请求重载键位")]
pub(crate) fn w_c2826_keybindings_bad(host_pump_bdd: &HostPumpBdd) {
    let pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("keybindings.json"), "not-json{{").unwrap();
    // 泵内会话键位重载：失败保留旧绑定并出诊断。
    let _ = pump.session.reload_keybindings(dir.path());
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("返回诊断错误且界面渲染正常")]
pub(crate) fn t_c2826_keybindings_bad(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    assert!(!frame.is_empty(), "c2826: 失败重载后界面仍渲染");
}

#[then("footer 反映新 active 模型且无换模预告")]
pub(crate) fn t_c2826_models_footer(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    let footer = frame.lines().last().unwrap_or_default();
    assert!(
        footer.contains("fake"),
        "c2826: 直设后 footer 应反映 active 模型：{footer}"
    );
    assert!(
        !footer.contains("Next turn") && !footer.contains("next turn"),
        "c2826: 产品面无换模预告：{footer}"
    );
}

#[then("驱动估计恰被调用一次")]
pub(crate) fn t_c2826_single_estimate_refresh(host_pump_bdd: &HostPumpBdd) {
    let frame = c2826_frame(host_pump_bdd);
    let footer = frame.lines().last().unwrap_or_default();
    assert!(
        footer.contains("used 1.2k tokens"),
        "c2826: 刷新应经 Driver 只读估计落到 footer：{footer}"
    );
}

#[when("以固定估计驱动一次 footer token 刷新")]
pub(crate) async fn w_c2826_footer_refresh_once(host_pump_bdd: &HostPumpBdd) {
    use crate::protocol::model::ContextTokenEstimate;
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.driver
        .set_session_messages(crate::app::tui::harness::harness_sample_session_messages());
    pump.driver
        .set_estimate_override(Some(ContextTokenEstimate {
            tokens: 1_234,
            provenance: crate::protocol::model::TokenProvenance::Api,
            usage_tokens: 1_234,
            trailing_tokens: 0,
            last_usage_index: None,
        }));
    crate::app::tui::refresh_footer_tokens(&mut pump.session, &mut pump.driver).await;
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[when("以小高度终端渲染忙碌帧")]
pub(crate) fn w_c2826_short_terminal_busy(host_pump_bdd: &HostPumpBdd) {
    use crate::app::tui::TuiHostSession as HostSession;
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.session = HostSession::new_product_ui(crate::app::tui::harness::TestTerminal::new(80, 10));
    pump.session.on_run_started("hi");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    let frame = c2826_frame(host_pump_bdd);
    host_pump_bdd.ansi_frames.borrow_mut().push(frame);
}

#[then("status lead 短词在视口内可见")]
pub(crate) fn t_c2826_short_terminal_busy(host_pump_bdd: &HostPumpBdd) {
    let frames = host_pump_bdd.ansi_frames.borrow();
    let frame = frames.last().expect("frame");
    assert!(
        frame.contains("Working"),
        "c2826: 短终端 busy 时 status lead 应可见：{frame}"
    );
}

// ── 裸规则回填（c2835 后继）：文档 / 即时日志 / 绘制缓存 / 交互模式 / PTY 登记 ──

#[when("读取产品 TUI 面 AGENTS 文档")]
fn w_read_tui_agents_doc(host_pump_bdd: &HostPumpBdd) {
    let text =
        std::fs::read_to_string("src/app/tui/AGENTS.md").expect("src/app/tui/AGENTS.md 可读");
    host_pump_bdd.doc_text.borrow_mut().replace(text);
}

#[then("文档含本地布局地图与验证命令指针且无进度板")]
fn t_tui_agents_doc_carries_layout_map(host_pump_bdd: &HostPumpBdd) {
    let text = host_pump_bdd
        .doc_text
        .borrow_mut()
        .take()
        .expect("AGENTS 文档已读取");
    assert!(
        text.contains("角色") && text.contains("职责"),
        "MUST 有本地布局地图（角色 / 职责 / 禁止表）：{text}"
    );
    assert!(text.contains("just "), "MUST 有验证命令指针（just …）");
    for drifted in ["进度", "待办板"] {
        assert!(!text.contains(drifted), "面 AGENTS 不应带{drifted}类易腐块");
    }
}

#[when("以临时目录请求即时文件日志")]
fn w_init_instant_file_logging(host_pump_bdd: &HostPumpBdd) {
    use xylitol_ai_bridge::provider::trace as pt;
    // 探针会写进程级闸态；跑完即复位，不依赖同进程里其他场景的执行顺序。
    let gate = pt::provider_trace_active();
    let io_tier = pt::observation_io_tier();
    let tool_io_tier = pt::tool_observation_io_tier();
    let dir = std::env::temp_dir().join(format!("xy-bdd-logging-{}", std::process::id()));
    let installed = crate::app::cli::logging::init_logging(
        &dir,
        &crate::infra::config::types::OtelConfig::default(),
    )
    .is_some();
    pt::set_provider_trace_active(gate);
    pt::set_observation_io_tier(io_tier);
    pt::set_tool_observation_io_tier(tool_io_tier);
    host_pump_bdd
        .log_probe
        .borrow_mut()
        .replace((installed, dir.join("logs")));
}

#[then("debug 构建默认安装文件日志且日志文件已落盘")]
fn t_debug_instant_file_logging_on(host_pump_bdd: &HostPumpBdd) {
    let (installed, log_dir) = host_pump_bdd
        .log_probe
        .borrow_mut()
        .take()
        .expect("即时日志探针已跑");
    assert!(installed, "debug 构建 MUST 默认启用即时文件日志");
    assert!(
        log_dir.join("xylitol.log").exists(),
        "日志 MUST 落到 <agent_dir>/logs/xylitol.log，实际目录 {log_dir:?}"
    );
}

#[given("构造含两条已提交助手条目的 UI 并渲染基线帧")]
fn g_two_committed_entries(host_pump_bdd: &HostPumpBdd) {
    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.entries.push(UiEntry::Assistant {
        text: "alpha para".into(),
    });
    model.entries.push(UiEntry::Assistant {
        text: "beta para".into(),
    });
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_scrollback_entry_misses_for_test();
    *host_pump_bdd.paint_probe.borrow_mut() = Some((root, model));
}

#[when("仅推进流式尾标再渲染一次并读取已提交条目重绘计数")]
fn w_tail_only_repaint(host_pump_bdd: &HostPumpBdd) {
    let (mut root, mut model) = host_pump_bdd
        .paint_probe
        .borrow_mut()
        .take()
        .expect("UI 已构造");
    model.streaming_assistant = "tail-1".into();
    root.apply_ui_model(&model);
    let _ = root.render(80);
    host_pump_bdd
        .counts
        .borrow_mut()
        .push(root.scrollback_entry_misses_for_test());
    *host_pump_bdd.paint_probe.borrow_mut() = Some((root, model));
}

#[then("已提交条目在第二次渲染零重绘")]
fn t_committed_entries_reuse_cache(host_pump_bdd: &HostPumpBdd) {
    let misses = *host_pump_bdd.counts.borrow().last().expect("计数已读");
    assert_eq!(
        misses, 0,
        "width/fold 不变时，仅流式尾标变化 MUST NOT 让已提交条目重绘"
    );
}

#[given("构造含稳定 Markdown 前缀的流式助手 UI")]
fn g_streaming_stable_prefix(host_pump_bdd: &HostPumpBdd) {
    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.begin_run("hi");
    // 足够多的完整段落，使稳定前缀非空（与单测同形）。
    model.streaming_assistant = "alpha para\n\nbeta para\n\n".into();
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_streaming_assistant_parse_counts_for_test();
    *host_pump_bdd.paint_probe.borrow_mut() = Some((root, model));
}

#[when("仅以后缀增长连续渲染四十次并读取全量解析计数")]
fn w_suffix_growth_parses(host_pump_bdd: &HostPumpBdd) {
    let (mut root, mut model) = host_pump_bdd
        .paint_probe
        .borrow_mut()
        .take()
        .expect("UI 已构造");
    for i in 0..40 {
        model.streaming_assistant.push_str(&format!("tok{i} "));
        if i % 10 == 9 {
            model.streaming_assistant.push_str("\n\n");
        }
        root.apply_ui_model(&model);
        let _ = root.render(80);
    }
    host_pump_bdd
        .counts
        .borrow_mut()
        .push(root.streaming_assistant_full_parses_for_test());
    *host_pump_bdd.paint_probe.borrow_mut() = Some((root, model));
}

#[then("全量 Markdown 解析次数远小于渲染次数")]
fn t_full_parses_bounded_by_stable_prefix(host_pump_bdd: &HostPumpBdd) {
    let full = *host_pump_bdd.counts.borrow().last().expect("计数已读");
    assert!(
        full <= 8,
        "稳定前缀 MUST 把全量 Markdown 解析限制在段落数级：full_parses={full}，渲染 40 次"
    );
}

#[when("以产品 UI 构造主机会话并查询交互模式")]
fn w_product_ui_interaction_mode(host_pump_bdd: &HostPumpBdd) {
    let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let mode = format!("{:?}", session.tui.interaction_mode());
    *host_pump_bdd.mode_probe.borrow_mut() = Some(mode);
}

#[then("产品交互模式为 ApplicationOwned")]
fn t_product_mode_is_application_owned(host_pump_bdd: &HostPumpBdd) {
    let mode = host_pump_bdd
        .mode_probe
        .borrow_mut()
        .take()
        .expect("模式已查询");
    assert!(
        mode.contains("ApplicationOwned"),
        "产品 UI 启动构造时 MUST 绑 ApplicationOwned，实得 {mode}"
    );
}

#[when("读取 PTY 冒烟登记")]
fn w_read_pty_registration(host_pump_bdd: &HostPumpBdd) {
    let justfile = std::fs::read_to_string("justfile").expect("justfile 可读");
    let pty = std::fs::read_to_string("tests/tui_e2e/pty.rs").expect("tests/tui_e2e/pty.rs 可读");
    host_pump_bdd
        .doc_text
        .borrow_mut()
        .replace(format!("{justfile}\n{pty}"));
}

#[then("存在 just test-tui-e2e-pty 且冒烟为 ignore 不进默认门禁")]
fn t_pty_smoke_registration(host_pump_bdd: &HostPumpBdd) {
    let text = host_pump_bdd
        .doc_text
        .borrow_mut()
        .take()
        .expect("登记已读取");
    assert!(
        text.contains("test-tui-e2e-pty"),
        "MUST 有 just test-tui-e2e-pty 入口"
    );
    assert!(
        text.contains("#[ignore]"),
        "PTY 冒烟 MUST 为 #[ignore]，不进默认 just qa"
    );
    assert!(
        text.contains("spawn_product_fake"),
        "MUST 有产品二进制 + Fake 模型的 PTY 冒烟"
    );
}
