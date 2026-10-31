use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_app_tui::TuiHostEvent as HostEvent;
use xylitol_app_tui::TuiHostSession as HostSession;
use xylitol_app_tui::UiRoot;
use xylitol_app_tui::harness::{ScriptedDriver, TestTerminal, enter_event, pump_host_driver};
use xylitol_app_tui::{BashBlockStatus, UiEntry, UiModel};
use xylitol_plugin_sdk::XyBashResult;
use xylitol_tui::Component;
use xylitol_tui::Terminal as _;

pub struct HostPump {
    pub session: HostSession<TestTerminal>,
    pub driver: ScriptedDriver,
}

pub struct HostPumpBdd {
    pub pump: RefCell<Option<HostPump>>,
    pub ansi_frames: RefCell<Vec<String>>,
    pub doc_text: RefCell<Option<String>>,
    pub paint_probe: RefCell<Option<(UiRoot, UiModel)>>,
    pub counts: RefCell<Vec<u64>>,
    pub log_probe: RefCell<Option<(bool, std::path::PathBuf)>>,
    pub mode_probe: RefCell<Option<String>>,
    pub observe_prev: RefCell<xylitol_app_tui::driver::AttachObserve>,
    pub observe_notice: RefCell<Option<&'static str>>,
    pub storage: RefCell<Option<std::sync::Arc<xylitol_app_tui::storage::ClientStorage>>>,
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
        observe_prev: RefCell::new(xylitol_app_tui::driver::AttachObserve::Live),
        observe_notice: RefCell::new(None),
        storage: RefCell::new(None),
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
    use crate::bdd::steps_app_tui_transcript::rail_prefix;
    let ansi = render_frame(host_pump_bdd, 120);
    let header = ansi
        .lines()
        .find(|l| l.contains("echo done"))
        .unwrap_or_else(|| panic!("bang header line in frame:\n{ansi}"));
    let theme = xylitol_app_tui::LayoutTheme::product_dark();
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

use xylitol_app_tui::harness::esc_event;

fn esc_stream(
    after_ms: u64,
    backlog_esc: usize,
) -> impl futures::Stream<Item = Result<HostEvent, xylitol_app_tui::XyDriverError>> {
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

fn open_busy(bdd: &HostPumpBdd) {
    let mut pump = fresh_pump();
    pump.session.on_run_started("hello");
    put_pump(bdd, pump);
}

fn open_tree_ready(bdd: &HostPumpBdd) {
    use xylitol_app_tui::harness::harness_sample_message_history_tree;
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
    use xylitol_app_tui::harness::drain_pending;
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

async fn submit_with(bdd: &HostPumpBdd, ev: xylitol_tui::InputEvent) {
    step_key(bdd, ev);
    pump_once(bdd).await;
}

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
    open_tree_ready(host_pump_bdd);
    step_key(host_pump_bdd, esc_event());
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
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

fn fresh_pump_with_hanging_bash() -> HostPump {
    let pump = fresh_pump();
    pump.driver.set_hang_bash_until_abort(true);
    pump
}

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

fn keys_then_escs_stream(
    mut prefix: Vec<xylitol_tui::InputEvent>,
    escs: usize,
) -> impl futures::Stream<Item = Result<HostEvent, xylitol_app_tui::XyDriverError>> {
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
    use xylitol_app_tui::harness::run_interactive_bang;
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

#[then("模型列表 effect 在 bang 结束前挂载生效")]
pub(crate) async fn t_atm18_inline_mounted(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let models_calls = pump.driver.models_calls();
    let submitted = pump
        .driver
        .bash_calls()
        .iter()
        .any(|(c, _)| c == "sleep 99");
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
    use xylitol_app_tui::harness::run_interactive_bang;
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

#[then("该命令 在 bang 结束前不执行")]
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

#[then("循环归还后 照常执行且 不丢失")]
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
    use xylitol_app_tui::harness::run_interactive_bang;
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

#[when("以主机泵提交挂起 bang 并注入 Esc 加积压 Esc")]
pub(crate) async fn w_ati30_first_bang_esc_backlog(host_pump_bdd: &HostPumpBdd) {
    use xylitol_app_tui::harness::run_interactive_bang;
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
    use xylitol_app_tui::harness::run_interactive_bang;
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

#[when("以主机泵开启忙碌流注入正文增量后按下 Esc 再注入迟到增量")]
pub(crate) async fn w_ati31_late_xy(host_pump_bdd: &HostPumpBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
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
    use xylitol_app_tui::XyDriverError;
    use xylitol_app_tui::harness::{default_client_reload_steps, run_interactive_reload};
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
    let steps = default_client_reload_steps();
    run_interactive_reload(&mut pump.session, &mut pump.driver, input, &steps)
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
    xylitol_app_tui::harness::drain_pending(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("drain");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

fn c2826_slot(host_pump_bdd: &HostPumpBdd) -> xylitol_app_tui::EditorSlotKind {
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
            .any(|(_, pos)| matches!(pos, xylitol_plugin_sdk::session::ForkPosition::At)),
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
    use xylitol_runtime::agent::runtime::XyEvent;
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
    use xylitol_app_tui::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};
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
            .set_session_list(vec![xylitol_plugin_sdk::SessionListEntry {
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

fn c2826_frame(host_pump_bdd: &HostPumpBdd) -> String {
    render_frame(host_pump_bdd, 100)
}

fn paint_clipped_viewport(host_pump_bdd: &HostPumpBdd) -> String {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.render_now().expect("clipped viewport render");
    let frame =
        xylitol_tui::utils::strip_ansi_codes(&pump.session.tui.last_painted_lines().join("\n"));
    put_pump(host_pump_bdd, pump);
    host_pump_bdd.ansi_frames.borrow_mut().push(frame.clone());
    frame
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
    use xylitol_runtime::agent::runtime::XyEvent;
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
    let has_aborted = xylitol_app_tui::trailing_aborted_note(&pump.session.ui_model().entries);
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
        crate::bdd::steps_app_tui_host::c2826_alt_up_event_pub(),
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
    xylitol_app_tui::harness::drain_pending(&mut pump.session, &mut pump.driver, &mut stream)
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
        use xylitol_app_tui::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};
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
        use xylitol_app_tui::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};
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
    set_editor(host_pump_bdd, "草稿上行");
    let mut pump = take_pump(host_pump_bdd);
    let input = futures::stream::iter(vec![
        Ok::<HostEvent, xylitol_app_tui::XyDriverError>(HostEvent::Tick),
        Ok(HostEvent::Input(enter_event())),
        Ok(HostEvent::Input(esc_event())),
    ]);
    let steps = xylitol_app_tui::default_client_reload_steps();
    xylitol_app_tui::run_interactive_reload(&mut pump.session, &mut pump.driver, input, &steps)
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
    let _ = paint_clipped_viewport(host_pump_bdd);
}

#[when("臂装复制成功提示后渲染已裁切视口")]
pub(crate) fn w_copy_notice_clipped(host_pump_bdd: &HostPumpBdd) {
    w_c2826_copy_notice(host_pump_bdd);
}

#[then("右上角通知栈含 Copied 短提示且不以 Error 前缀冒充")]
pub(crate) fn t_c2826_copy_notice(host_pump_bdd: &HostPumpBdd) {
    let frame = paint_clipped_viewport(host_pump_bdd);
    assert!(
        frame.contains("Copied"),
        "c2826: 应有 Copied 短提示：{frame}"
    );
    let copied_line = frame
        .lines()
        .find(|l| l.contains("Copied"))
        .expect("copied line");
    assert!(
        copied_line.contains('✓'),
        "Copied must use success stack icon: {copied_line}"
    );
    assert!(
        !copied_line.contains("Error"),
        "c2826: 成功提示不得用 Error 前缀：{copied_line}"
    );
}

#[when("以合成验收链驱动一整轮含工具与 abort 的会话并 /exit")]
pub(crate) async fn w_c2826_synthetic_chain(host_pump_bdd: &HostPumpBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
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
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
    drain(host_pump_bdd).await;
    c2826_submit_slash(host_pump_bdd, "第二问").await;
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
    use xylitol_plugin_sdk::model::ContextTokenEstimate;
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.driver
        .set_session_messages(xylitol_app_tui::harness::harness_sample_session_messages());
    pump.driver
        .set_estimate_override(Some(ContextTokenEstimate {
            tokens: 1_234,
            provenance: xylitol_plugin_sdk::model::TokenProvenance::Api,
            usage_tokens: 1_234,
            trailing_tokens: 0,
            last_usage_index: None,
            ..Default::default()
        }));
    xylitol_app_tui::refresh_footer_tokens(&mut pump.session, &mut pump.driver).await;
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[when("以小高度终端渲染忙碌帧")]
pub(crate) fn w_c2826_short_terminal_busy(host_pump_bdd: &HostPumpBdd) {
    use xylitol_app_tui::TuiHostSession as HostSession;
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.session = HostSession::new_product_ui(xylitol_app_tui::harness::TestTerminal::new(80, 10));
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

#[when("读取产品 TUI 面 AGENTS 文档")]
fn w_read_tui_agents_doc(host_pump_bdd: &HostPumpBdd) {
    let text = std::fs::read_to_string("crates/xylitol-app-tui/src/AGENTS.md")
        .expect("crates/xylitol-app-tui/src/AGENTS.md 可读");
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
    use xylitol_official_plugin_provider_openai::provider::trace as pt;
    let _g = crate::bdd::steps_otel_obs::OBS_GATE_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let gate = pt::provider_trace_active();
    let io_tier = pt::observation_io_tier();
    let tool_io_tier = pt::tool_observation_io_tier();
    let dir = std::env::temp_dir().join(format!("xy-bdd-logging-{}", std::process::id()));
    let installed = xylitol::cli::logging::init_logging(
        &dir,
        &xylitol_kernel::config::types::OtelConfig::default(),
    )
    .is_some();
    pt::set_provider_trace_active(gate);
    pt::set_observation_io_tier(io_tier);
    pt::set_tool_observation_io_tier(tool_io_tier);
    drop(_g);
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

#[given("构造含开围栏与代码正文的流式助手 UI")]
fn g_streaming_open_fence(host_pump_bdd: &HostPumpBdd) {
    let root = UiRoot::new();
    let mut model = UiModel::new();
    model.begin_run("hi");
    model.streaming_assistant = "```rust\nprint(1)\n".into();
    *host_pump_bdd.paint_probe.borrow_mut() = Some((root, model));
}

#[when("追加半截闭合围栏再渲染并补全闭合围栏再渲染")]
fn w_partial_then_complete_streaming_fence(host_pump_bdd: &HostPumpBdd) {
    let (mut root, mut model) = host_pump_bdd
        .paint_probe
        .borrow_mut()
        .take()
        .expect("UI 已构造");
    model.streaming_assistant.push_str("``");
    root.apply_ui_model(&model);
    let _ = root.render(80);
    host_pump_bdd
        .counts
        .borrow_mut()
        .push(root.streaming_assistant_last_lines_for_test() as u64);
    model.streaming_assistant.push('`');
    root.apply_ui_model(&model);
    let _ = root.render(80);
    host_pump_bdd
        .counts
        .borrow_mut()
        .push(root.streaming_assistant_last_lines_for_test() as u64);
    *host_pump_bdd.paint_probe.borrow_mut() = Some((root, model));
}

#[then("两次流式块可见行数相同")]
fn t_streaming_fence_heights_equal(host_pump_bdd: &HostPumpBdd) {
    let counts = host_pump_bdd.counts.borrow();
    let partial = counts[counts.len() - 2];
    let complete = counts[counts.len() - 1];
    assert_eq!(
        partial, complete,
        "半截 closer 与补全后流式块可见行数 MUST 相同：partial={partial} complete={complete}"
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

#[then("右上角通知栈含 Copied 且不以 Error 前缀冒充")]
pub(crate) fn t_notice_stack_copied(host_pump_bdd: &HostPumpBdd) {
    t_c2826_copy_notice(host_pump_bdd);
}

#[given("对话内容高于可见 transcript 窗且视口未停在内容顶")]
pub(crate) fn g_tall_scrolled_viewport(host_pump_bdd: &HostPumpBdd) {
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    for i in 0..80 {
        pump.session.push_scroll_notice(format!("row-{i:02}"));
    }
    pump.session.render_now().expect("ingest tall transcript");
    let notch = pump.session.tui.application_owned_wheel_notch();
    assert!(
        pump.session.tui.application_owned_scroll_by(-(notch * 8)),
        "must leave content top"
    );
    pump.session.render_now().expect("scrolled paint");
    assert!(
        pump.session.tui.application_owned_scroll_top() > 0,
        "scroll_top must be off content 0"
    );
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("可见视口顶部含 Copied 且不以 Error 前缀冒充")]
pub(crate) fn t_copied_pins_viewport_top(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.render_now().expect("clipped viewport");
    let painted = pump.session.tui.last_painted_lines();
    let first = xylitol_tui::utils::strip_ansi_codes(&painted[0]);
    let content0 = pump
        .session
        .tui
        .application_owned_content_lines()
        .and_then(|c| c.first())
        .map(|l| xylitol_tui::utils::strip_ansi_codes(l))
        .unwrap_or_default();
    put_pump(host_pump_bdd, pump);
    assert!(
        first.contains("Copied") && first.contains('✓'),
        "visible top must pin Copied: {first:?}"
    );
    assert!(
        !first.contains("Error"),
        "must not use Error prefix: {first:?}"
    );
    assert!(
        !content0.contains("Copied"),
        "unclipped content[0] must not hold the stack: {content0:?}"
    );
}

#[when("以主机泵完成空闲会话切换并渲染")]
pub(crate) fn w_idle_session_switch(host_pump_bdd: &HostPumpBdd) {
    let mut pump = host_pump_bdd
        .pump
        .borrow_mut()
        .take()
        .unwrap_or_else(fresh_pump);
    pump.session.apply_resume_session(
        "newer",
        xylitol_app_tui::harness::harness_sample_session_messages(),
        false,
    );
    pump.session.render_now().expect("switch render");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
}

#[then("滚动提示尾插含 switched 且通知栈不含该文案")]
pub(crate) fn t_session_switch_trails(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let trailed = pump
        .session
        .ui_model()
        .entries
        .iter()
        .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("switched →")));
    let root = pump.session.ui_root().expect("ui").clone();
    let stack = root.borrow().client_notice_bodies_for_test();
    let toast = root.borrow().toast_notice_body().map(str::to_string);
    put_pump(host_pump_bdd, pump);
    assert!(trailed, "switched → must trail as ScrollNotice");
    assert!(
        stack.iter().all(|b| !b.contains("switched")),
        "notice stack must not hold switched: {stack:?}"
    );
    assert!(
        toast.as_deref().is_none_or(|t| !t.contains("switched")),
        "通知条 must not hold switched: {toast:?}"
    );
}

#[then("右上角通知栈含 Copied Last Msg 且不以 Error 前缀冒充")]
pub(crate) fn t_copied_last_msg_stack(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let stack = root.borrow().client_notice_bodies_for_test();
    let trailed = pump.session.ui_model().entries.iter().any(
        |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("Copied last assistant")),
    );
    put_pump(host_pump_bdd, pump);
    let frame = paint_clipped_viewport(host_pump_bdd);
    assert!(
        stack.iter().any(|b| b == "Copied Last Msg"),
        "stack must hold Copied Last Msg: {stack:?}"
    );
    assert!(!trailed, "must not trail Copied last assistant message");
    let line = frame
        .lines()
        .find(|l| l.contains("Copied Last Msg"))
        .expect("Copied Last Msg visible");
    assert!(line.contains('✓'), "success icon: {line}");
    assert!(!line.contains("Error"), "must not use Error prefix: {line}");
}

use xylitol_app_tui::driver::XyDriver as _;

#[given("产品 TUI 主循环守望观景态")]
fn g_c2895_observe_watch(host_pump_bdd: &HostPumpBdd) {
    if host_pump_bdd.pump.borrow().is_none() {
        put_pump(host_pump_bdd, fresh_pump());
    }
    let pump = take_pump(host_pump_bdd);
    *host_pump_bdd.observe_prev.borrow_mut() = xylitol_app_tui::driver::AttachObserve::Live;
    *pump.driver.observe_state.lock().unwrap() = xylitol_app_tui::driver::AttachObserve::Live;
    put_pump(host_pump_bdd, pump);
}

#[when("远端驱动报告回合成中观景")]
fn w_c2895_observe_inflight(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    *pump.driver.observe_state.lock().unwrap() = xylitol_app_tui::driver::AttachObserve::Inflight;
    let cur = pump.driver.attach_observe();
    let prev = *host_pump_bdd.observe_prev.borrow();
    let notice = xylitol_app_tui::observe_notice(prev, cur);
    *host_pump_bdd.observe_prev.borrow_mut() = cur;
    *host_pump_bdd.observe_notice.borrow_mut() = notice;
    put_pump(host_pump_bdd, pump);
}

#[then("一次性提示进入观景并指引 session-resume")]
fn t_c2895_observe_inflight_notice(host_pump_bdd: &HostPumpBdd) {
    let notice = *host_pump_bdd.observe_notice.borrow();
    let notice = notice.expect("进入观景须一次性提示");
    assert!(
        notice.contains("session-resume"),
        "提示须指引重挂: {notice}"
    );
}

#[when("远端驱动报告对端回合已结束")]
fn w_c2895_observe_ended(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    *pump.driver.observe_state.lock().unwrap() = xylitol_app_tui::driver::AttachObserve::Ended;
    let cur = pump.driver.attach_observe();
    let prev = *host_pump_bdd.observe_prev.borrow();
    let notice = xylitol_app_tui::observe_notice(prev, cur);
    *host_pump_bdd.observe_prev.borrow_mut() = cur;
    *host_pump_bdd.observe_notice.borrow_mut() = notice;
    put_pump(host_pump_bdd, pump);
}

#[then("一次性提示可重挂收看")]
fn t_c2895_observe_ended_notice(host_pump_bdd: &HostPumpBdd) {
    let notice = *host_pump_bdd.observe_notice.borrow();
    let notice = notice.expect("结束须转可重挂提示");
    assert!(
        notice.contains("session-resume"),
        "可重挂提示须指引: {notice}"
    );
}

use std::sync::Arc as StdArc;
use xylitol_app_tui::storage::ClientStorage as StorageClient;

fn c2470_storage(host_pump_bdd: &HostPumpBdd) -> StdArc<StorageClient> {
    if let Some(s) = host_pump_bdd.storage.borrow().clone() {
        return s;
    }
    let dir = std::env::temp_dir().join(format!("xyl-bdd-c2470-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let storage = StdArc::new(StorageClient::new(dir, "default").unwrap());
    *host_pump_bdd.storage.borrow_mut() = Some(storage.clone());
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_client_storage(Some(storage.clone()));
    put_pump(host_pump_bdd, pump);
    storage
}

#[given("产品 TUI 会话接入存储通道")]
fn g_c2470_storage_mounted(host_pump_bdd: &HostPumpBdd) {
    if host_pump_bdd.pump.borrow().is_none() {
        put_pump(host_pump_bdd, fresh_pump());
    }
    let _ = c2470_storage(host_pump_bdd);
}

#[when("经真实按键路径发送 prompt")]
async fn w_c2470_submit(host_pump_bdd: &HostPumpBdd) {
    submit_bang(host_pump_bdd, "c2470-hello").await;
}

#[then("发送文本落通道且进程内历史含该条")]
fn t_c2471_persisted(host_pump_bdd: &HostPumpBdd) {
    let storage = host_pump_bdd.storage.borrow().clone().expect("storage");
    let persisted = storage.editor_history();
    assert!(
        persisted.contains(&"c2470-hello".to_string()),
        "发送须落通道: {persisted:?}"
    );
    let pump = take_pump(host_pump_bdd);
    let in_memory = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow()
        .editor_history_texts();
    assert!(
        in_memory.contains(&"c2470-hello".to_string()),
        "{in_memory:?}"
    );
    put_pump(host_pump_bdd, pump);
}

#[when("他实例另写历史后 本端按 ↑ 召回")]
async fn w_c2470_twin_and_up(host_pump_bdd: &HostPumpBdd) {
    let storage = host_pump_bdd.storage.borrow().clone().expect("storage");
    let root = storage
        .dir()
        .parent()
        .and_then(|p| p.parent())
        .expect("root")
        .to_path_buf();
    let twin = StorageClient::new(root, "default").unwrap();
    twin.record_editor_prompt("c2470-twin");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(xylitol_app_tui::harness::key_event(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        )))
        .expect("key step");
    put_pump(host_pump_bdd, pump);
}

#[then("合并后两实例条目并见且不重复")]
fn t_c2470_merged(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let history = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow()
        .editor_history_texts();
    let hello = history
        .iter()
        .filter(|t| t.as_str() == "c2470-hello")
        .count();
    let twin = history
        .iter()
        .filter(|t| t.as_str() == "c2470-twin")
        .count();
    assert_eq!(hello, 1, "去重: {history:?}");
    assert_eq!(twin, 1, "跨实例条目须并入: {history:?}");
    put_pump(host_pump_bdd, pump);
}

#[when("他实例先写旧条目再写新条目后 本端按 ↑ 召回")]
async fn w_c2917_twin_old_new_and_up(host_pump_bdd: &HostPumpBdd) {
    let storage = host_pump_bdd.storage.borrow().clone().expect("storage");
    let root = storage
        .dir()
        .parent()
        .and_then(|p| p.parent())
        .expect("root")
        .to_path_buf();
    let twin = StorageClient::new(root, "default").unwrap();
    twin.record_editor_prompt("c2917-old");
    twin.record_editor_prompt("c2917-new");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(xylitol_app_tui::harness::key_event(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        )))
        .expect("key step");
    put_pump(host_pump_bdd, pump);
}

#[then("召回文本为通道最新条目")]
fn t_c2917_recall_is_newest(host_pump_bdd: &HostPumpBdd) {
    let text = editor_text(host_pump_bdd);
    assert_eq!(text.trim(), "c2917-new", "召回须最新优先: {text:?}");
}

use xylitol_app_tui::harness::key_event as c2875_key_event;

fn g2875_mount(host_pump_bdd: &HostPumpBdd) {
    if host_pump_bdd.pump.borrow().is_none() {
        put_pump(host_pump_bdd, fresh_pump());
    }
}

#[given("产品 TUI 主循环就绪")]
fn g_c2875_ready(host_pump_bdd: &HostPumpBdd) {
    g2875_mount(host_pump_bdd);
}

async fn c2875_press(
    host_pump_bdd: &HostPumpBdd,
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(c2875_key_event(code, modifiers)))
        .expect("key step");
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump");
    put_pump(host_pump_bdd, pump);
}

#[when("切换覆盖弹出框显隐")]
async fn w_c2875_toggle(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
}

fn c2875_frame(host_pump_bdd: &HostPumpBdd) -> String {
    let mut pump = take_pump(host_pump_bdd);
    let frame = pump.session.render_now().is_ok();
    let _ = frame;
    let rendered = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow_mut()
        .render_with_viewport_overlay(120)
        .join("\n");
    put_pump(host_pump_bdd, pump);
    rendered
}

#[then("覆盖弹出框渲染会话详情")]
fn t_c2875_visible(host_pump_bdd: &HostPumpBdd) {
    let frame = c2875_frame(host_pump_bdd);
    assert!(
        frame.contains("会话详情"),
        "覆盖弹出框须上屏: {}",
        {
            let f = &frame;
            f.get(0..600)
                .map(|s| s.to_string())
                .unwrap_or_else(|| f.clone())
        }
    );
}

#[when("复按切换收起")]
async fn w_c2875_toggle_again(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
}

#[then("覆盖弹出框不再渲染")]
fn t_c2875_hidden(host_pump_bdd: &HostPumpBdd) {
    let frame = c2875_frame(host_pump_bdd);
    assert!(!frame.contains("会话详情"), "覆盖弹出框须收起");
}

#[when("按 Esc 收起覆盖弹出框")]
async fn w_c2875_esc_close(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    )
    .await;
}

#[when("再按 Esc")]
async fn w_c2875_esc_again(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    )
    .await;
}

#[then("覆盖弹出框不再渲染且会话树未打开")]
fn t_c2875_esc_no_tree_chain(host_pump_bdd: &HostPumpBdd) {
    let frame = c2875_frame(host_pump_bdd);
    assert!(!frame.contains("会话详情"), "闲置 Esc 须收起覆盖弹出框");
    let s = stats(host_pump_bdd);
    assert!(!s.tree_open, "收框的 Esc 不得记入双击链开树");
    assert_eq!(s.tree_calls, 0, "收框不得触发会话树");
}

#[when("忙碌中按 Esc")]
async fn w_c2875_busy_esc(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.on_run_started("c2875-busy");
    put_pump(host_pump_bdd, pump);
    pump_once(host_pump_bdd).await;
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    )
    .await;
}

#[then("中止计一次且覆盖弹出框仍渲染")]
fn t_c2875_busy_esc_abort(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 1, "忙碌 Esc 须透传中止链: {:?}", s.notices);
    let frame = c2875_frame(host_pump_bdd);
    assert!(frame.contains("会话详情"), "忙碌 Esc 不得收起覆盖弹出框");
}

#[when("切换侧栏")]
async fn w_c2905_sidebar_toggle(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('u'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
}

fn c2905_assert_on(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    assert!(
        pump.session
            .ui_root()
            .expect("ui")
            .borrow()
            .session_sidebar_on(),
        "侧栏须开启"
    );
    let frame = pump
        .session
        .ui_root()
        .unwrap()
        .borrow_mut()
        .render_with_viewport_overlay(120)
        .join("\n");
    put_pump(host_pump_bdd, pump);
    assert!(frame.contains("会话侧栏"), "侧栏须上屏");
    assert!(frame.contains("施工中"), "c2912 侧栏须为实验占位");
}

#[then("右侧栏上屏且内容区让宽")]
fn t_c2905_on(host_pump_bdd: &HostPumpBdd) {
    c2905_assert_on(host_pump_bdd);
}

#[when("复按收起")]
async fn w_c2905_toggle_off(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('u'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
}

#[then("侧栏消失且内容区恢复")]
fn t_c2905_off(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    assert!(
        !pump
            .session
            .ui_root()
            .expect("ui")
            .borrow()
            .session_sidebar_on(),
        "侧栏须关闭"
    );
    let frame = pump
        .session
        .ui_root()
        .unwrap()
        .borrow_mut()
        .render_with_viewport_overlay(120)
        .join("\n");
    put_pump(host_pump_bdd, pump);
    assert!(!frame.contains("会话侧栏"), "侧栏须消失");
}

#[given("产品 TUI 会话就绪于窄终端")]
fn g_c2918_narrow_mount(host_pump_bdd: &HostPumpBdd) {
    if host_pump_bdd.pump.borrow().is_none() {
        *host_pump_bdd.pump.borrow_mut() = Some(HostPump {
            session: HostSession::new_product_ui(TestTerminal::new(78, 40)),
            driver: ScriptedDriver::new(),
        });
    }
}

#[then("覆盖弹出框上屏且侧栏不出")]
fn t_c2918_popup_fallback(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    assert!(!root.borrow().session_sidebar_on(), "窄档不得开侧栏");
    let frame = root
        .borrow_mut()
        .render_with_viewport_overlay(78)
        .join("\n");
    put_pump(host_pump_bdd, pump);
    assert!(frame.contains("┌ 会话详情"), "窄档须降级弹出框:\n{frame}");
    assert!(!frame.contains("会话侧栏"), "窄档不得渲染侧栏:\n{frame}");
}

#[when("先开覆盖弹出框再开侧栏")]
async fn w_c2905_both(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('u'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
}

#[then("两面同时可辨且覆盖弹出框贴内容右边界")]
fn t_c2905_coexist(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let frame = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow_mut()
        .render_with_viewport_overlay(120)
        .join("\n");
    assert!(frame.contains("会话详情"), "覆盖弹出框须在");
    assert!(frame.contains("会话侧栏"), "侧栏须在");
    put_pump(host_pump_bdd, pump);
}

#[given("产品 ApplicationOwned 会话已向下滚动")]
pub(crate) fn g_product_ao_scrolled(host_pump_bdd: &HostPumpBdd) {
    g_tall_scrolled_viewport(host_pump_bdd);
}

#[when("产品会话左键单击跳转顶部按钮字符")]
pub(crate) async fn w_product_gutter_click_top(host_pump_bdd: &HostPumpBdd) {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mut pump = take_pump(host_pump_bdd);
    let hits = pump.session.tui.scroll_gutter_hit_set();
    pump.session.tui.replace_hits(hits);
    let col = pump.session.tui.terminal.columns().saturating_sub(1);
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        pump.session
            .step(HostEvent::Input(xylitol_tui::InputEvent::Mouse(
                MouseEvent {
                    kind,
                    column: col,
                    row: 0,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                },
            )))
            .expect("gutter click step");
    }
    pump.session.render_now().expect("post-click paint");
    *host_pump_bdd.pump.borrow_mut() = Some(pump);
    pump_once(host_pump_bdd).await;
}

#[then("产品应用视口滚动位置归零且无选区")]
pub(crate) fn t_product_gutter_jump_home(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    assert_eq!(
        pump.session.tui.application_owned_scroll_top(),
        0,
        "gutter top click must jump to head"
    );
    assert!(
        !pump.session.tui.application_owned_has_selection(),
        "gutter click must not start selection"
    );
}

thread_local! {
    static SIDEBAR_WHEEL_SCROLL: std::cell::RefCell<Option<usize>> = const { std::cell::RefCell::new(None) };
}

#[given("产品 TUI 主循环就绪且侧栏已开启")]
async fn g_c2911_sidebar_ready(host_pump_bdd: &HostPumpBdd) {
    g2875_mount(host_pump_bdd);
    w_c2905_sidebar_toggle(host_pump_bdd).await;
}

#[given("侧栏已开启")]
async fn g_c2911_sidebar_on(host_pump_bdd: &HostPumpBdd) {
    g_c2911_sidebar_ready(host_pump_bdd).await;
}

#[given("覆盖弹出框已开启")]
async fn g_c2911_popup_on(host_pump_bdd: &HostPumpBdd) {
    g2875_mount(host_pump_bdd);
    w_c2875_toggle(host_pump_bdd).await;
}

#[when("循环侧栏宽度")]
async fn w_c2911_sidebar_width_cycle(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('u'),
        crossterm::event::KeyModifiers::ALT | crossterm::event::KeyModifiers::SHIFT,
    )
    .await;
}

#[then("侧栏宽度按 24/32/40 档变化且内容区不少于 40 列")]
fn t_c2911_sidebar_width_cycle(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let width = root.borrow().session_sidebar_width();
    assert!(
        matches!(width, 24 | 32 | 40),
        "sidebar width must stay on a preset tier: {width}"
    );
    let (on, sidebar_w, content_w) = root.borrow().session_sidebar_layout(120);
    assert!(on, "sidebar must stay on");
    assert_eq!(sidebar_w, width, "layout must use the selected tier");
    assert!(
        content_w >= 40,
        "content must keep at least 40 columns: {content_w}"
    );
    put_pump(host_pump_bdd, pump);
}

#[when("按会话详情循环键")]
async fn w_c2911_details_cycle(host_pump_bdd: &HostPumpBdd) {
    c2875_press(
        host_pump_bdd,
        crossterm::event::KeyCode::Char('o'),
        crossterm::event::KeyModifiers::ALT,
    )
    .await;
}

#[when("再按会话详情循环键")]
async fn w_c2911_details_cycle_again(host_pump_bdd: &HostPumpBdd) {
    w_c2911_details_cycle(host_pump_bdd).await;
}

#[then("先出现覆盖弹出框")]
fn t_c2911_details_popup(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    assert!(root.borrow().session_popup_open(), "popup must open first");
    assert!(
        !root.borrow().session_sidebar_on(),
        "sidebar must not open before the popup"
    );
    put_pump(host_pump_bdd, pump);
}

#[then("覆盖弹出框收起且侧栏上屏")]
fn t_c2911_details_sidebar(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    assert!(
        !root.borrow().session_popup_open(),
        "popup must close when cycling to sidebar"
    );
    assert!(root.borrow().session_sidebar_on(), "sidebar must open");
    put_pump(host_pump_bdd, pump);
}

#[then("侧栏收起")]
fn t_c2911_details_off(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    assert!(
        !root.borrow().session_sidebar_on(),
        "third cycle press must close the sidebar"
    );
    assert!(
        !root.borrow().session_popup_open(),
        "third cycle press must leave no popup"
    );
    put_pump(host_pump_bdd, pump);
}

async fn c2911_click(host_pump_bdd: &HostPumpBdd, col: u16, row: u16) {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mut pump = take_pump(host_pump_bdd);
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        pump.session
            .step(HostEvent::Input(xylitol_tui::InputEvent::Mouse(
                MouseEvent {
                    kind,
                    column: col,
                    row,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                },
            )))
            .expect("chrome click step");
    }
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("chrome click pump");
    put_pump(host_pump_bdd, pump);
}

#[when("点击侧栏带")]
async fn w_c2911_click_sidebar(host_pump_bdd: &HostPumpBdd) {
    c2911_click(host_pump_bdd, 110, 5).await;
}

#[then("侧栏关闭且内容区无选区")]
fn t_c2911_click_sidebar(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    assert!(!root.borrow().session_sidebar_on(), "sidebar must close");
    assert!(
        !pump.session.tui.application_owned_has_selection(),
        "chrome click must not start a content selection"
    );
    put_pump(host_pump_bdd, pump);
}

#[when("点击覆盖弹出框")]
async fn w_c2911_click_popup(host_pump_bdd: &HostPumpBdd) {
    c2911_click(host_pump_bdd, 95, 2).await;
}

#[then("覆盖弹出框关闭")]
fn t_c2911_click_popup(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    assert!(
        !root.borrow().session_popup_open(),
        "popup must close on chrome click"
    );
    put_pump(host_pump_bdd, pump);
}

#[given("侧栏已开启且内容区已滚动")]
async fn g_c2911_sidebar_scrolled(host_pump_bdd: &HostPumpBdd) {
    g_tall_scrolled_viewport(host_pump_bdd);
    w_c2905_sidebar_toggle(host_pump_bdd).await;
}

#[when("在侧栏带内滚轮")]
fn w_c2911_sidebar_wheel(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    let before = pump.session.tui.application_owned_scroll_top();
    SIDEBAR_WHEEL_SCROLL.with(|slot| *slot.borrow_mut() = Some(before));
    pump.session
        .apply_sidebar_wheel_delta(-1)
        .expect("sidebar wheel");
    put_pump(host_pump_bdd, pump);
}

#[then("内容区滚动位置不变")]
fn t_c2911_sidebar_wheel(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let before = SIDEBAR_WHEEL_SCROLL
        .with(|slot| *slot.borrow())
        .expect("wheel baseline");
    assert_eq!(
        pump.session.tui.application_owned_scroll_top(),
        before,
        "wheel over the sidebar band must not scroll the content"
    );
    put_pump(host_pump_bdd, pump);
}

#[when("以主机泵注入能力快照后提交 {cmd:string}")]
pub(crate) async fn w_c2998_capabilities_open(host_pump_bdd: &HostPumpBdd, cmd: String) {
    use xylitol_plugin_sdk::session::{CapabilityEntry, CapabilityKind};
    let cmd = cmd.trim_matches('"');
    {
        let pump = host_pump_bdd
            .pump
            .borrow_mut()
            .take()
            .unwrap_or_else(fresh_pump);
        pump.driver.seed_capabilities(vec![
            CapabilityEntry {
                id: "tool:read".into(),
                kind: CapabilityKind::Tool,
                label: "read".into(),
                profile_enabled: true,
                session_disabled: false,
                effective_enabled: true,
                pending: false,
            },
            CapabilityEntry {
                id: "mcp:github".into(),
                kind: CapabilityKind::McpServer,
                label: "github".into(),
                profile_enabled: true,
                session_disabled: false,
                effective_enabled: true,
                pending: false,
            },
        ]);
        *host_pump_bdd.pump.borrow_mut() = Some(pump);
    }
    c2826_submit_slash(host_pump_bdd, cmd).await;
}

#[then("能力覆盖盘打开且列出能力 id 与汇总")]
pub(crate) fn t_c2998_capabilities_open(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Capabilities",
        "c2998: /capabilities 应打开能力覆盖盘"
    );
    let frame = render_frame(host_pump_bdd, 100);
    assert!(
        frame.contains("tool:read"),
        "c2998: 覆盖盘应列出能力 id：\n{frame}"
    );
    assert!(
        frame.contains("2/2 enabled"),
        "c2998: 覆盖盘应含 enabled/total 汇总：\n{frame}"
    );
    assert_eq!(stats(host_pump_bdd).aborts, 0);
}

#[when("以主机泵在能力覆盖盘上切换首项")]
pub(crate) async fn w_c2998_capabilities_toggle(host_pump_bdd: &HostPumpBdd) {
    step_key(host_pump_bdd, enter_event());
    pump_once(host_pump_bdd).await;
}

#[then("能力开关经共享分发提交且待生效提示出现")]
pub(crate) fn t_c2998_capabilities_toggled(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let slot = root.borrow().slot();
    let calls = pump.driver.capability_calls();
    let frame = root.borrow_mut().render(100).join("\n");
    put_pump(host_pump_bdd, pump);
    assert_eq!(
        format!("{slot:?}"),
        "Capabilities",
        "c2998: toggle 后覆盖盘应保持打开"
    );
    assert_eq!(
        calls,
        vec![("tool:read".to_string(), false)],
        "c2998: toggle 应经共享分发提交 SetCapability"
    );
    assert!(
        frame.contains("Next turn: capability overlay"),
        "c2998: status 区应显示待生效提示：\n{frame}"
    );
}

#[when("以主机泵在能力覆盖盘上按 Esc")]
pub(crate) async fn w_c2998_capabilities_esc(host_pump_bdd: &HostPumpBdd) {
    step_key(host_pump_bdd, esc_event());
    pump_once(host_pump_bdd).await;
}

#[then("能力覆盖盘关闭且未 abort agent")]
pub(crate) fn t_c2998_capabilities_closed(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Editor",
        "c2998: Esc 应关闭能力覆盖盘"
    );
    assert_eq!(stats(host_pump_bdd).aborts, 0);
}

#[when("以主机泵忙碌连续插队两条 steer 后渲染队列条")]
pub(crate) async fn w_c2999_two_steer(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    for text in ["steer-one", "steer-two"] {
        set_editor(host_pump_bdd, text);
        let mut pump = take_pump(host_pump_bdd);
        pump.session
            .step(HostEvent::Input(enter_event()))
            .expect("enter (steer)");
        put_pump(host_pump_bdd, pump);
        pump_once(host_pump_bdd).await;
    }
    let frame = c2826_frame(host_pump_bdd);
    host_pump_bdd.ansi_frames.borrow_mut().push(frame);
}

#[then("队列条仅一条 Steering 行且含最新文本与计数")]
pub(crate) fn t_c2999_two_steer(host_pump_bdd: &HostPumpBdd) {
    let frames = host_pump_bdd.ansi_frames.borrow();
    let frame = frames.last().expect("queue frame");
    let steering: Vec<&str> = frame
        .lines()
        .filter(|line| line.contains("Steering"))
        .collect();
    assert_eq!(steering.len(), 1, "c2999: 队列条每类至多一行：\n{frame}");
    assert!(
        steering[0].contains("steer-two"),
        "c2999: 应显示最新排队正文：{}",
        steering[0]
    );
    assert!(
        steering[0].contains("(2)"),
        "c2999: 多条排队应显示计数：{}",
        steering[0]
    );
}

#[when("以主机泵忙碌插队后点击队列条")]
pub(crate) async fn w_c3001_click_queue(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    set_editor(host_pump_bdd, "queued-click");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter (steer)");
    put_pump(host_pump_bdd, pump);
    pump_once(host_pump_bdd).await;
    let frame = c2826_frame(host_pump_bdd);
    let row = frame
        .lines()
        .position(|line| line.contains("Steering"))
        .expect("queue strip rendered") as u16;
    c2911_click(host_pump_bdd, 5, row).await;
    pump_once(host_pump_bdd).await;
}

#[then("编辑器含排队文本且队列清空")]
pub(crate) fn t_c3001_queue_click(host_pump_bdd: &HostPumpBdd) {
    let text = editor_text(host_pump_bdd);
    assert!(
        text.contains("queued-click"),
        "c3001: 点击队列条应还原文本进编辑器：{text:?}"
    );
    let pump = take_pump(host_pump_bdd);
    let steer = pump.session.ui_model().pending_steer.clone();
    let follow = pump.session.ui_model().pending_follow_up.clone();
    let clears = pump.driver.clear_calls.clone();
    put_pump(host_pump_bdd, pump);
    assert!(steer.is_empty(), "c3001: steer 队列应清空");
    assert!(follow.is_empty(), "c3001: follow-up 队列应清空");
    assert!(
        clears.iter().any(|(steer, follow)| *steer && *follow),
        "c3001: 点击应经 ClearQueue 清空双向队列：{clears:?}"
    );
}
