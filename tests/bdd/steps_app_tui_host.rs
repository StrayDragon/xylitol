//! Steps for the host-pump family (att9 / att11, later ati busy keys).
//!
//! ati30 contract: BDD reuses [`HostSession`] + [`ScriptedDriver`] +
//! [`pump_host_driver`] — the very same pump the unit slice tests use; never a
//! second side-effect pump.

use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol::app::tui::TuiHostEvent as HostEvent;
use xylitol::app::tui::TuiHostSession as HostSession;
use xylitol::app::tui::harness::{ScriptedDriver, TestTerminal, enter_event, pump_host_driver};
use xylitol::app::tui::{BashBlockStatus, UiEntry};
use xylitol::protocol::ports::XyBashResult;
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
    use crate::steps_app_tui_transcript::rail_prefix;
    let ansi = render_frame(host_pump_bdd, 120);
    let header = ansi
        .lines()
        .find(|l| l.contains("echo done"))
        .unwrap_or_else(|| panic!("bang header line in frame:\n{ansi}"));
    let theme = xylitol::app::tui::LayoutTheme::product_dark();
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
