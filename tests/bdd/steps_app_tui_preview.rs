//! Steps for app-tui-preview: designing CLI leaf + scene slot + designing preview inject.

use crate::bdd::steps_app_tui_host::{HostPump, HostPumpBdd};
use rstest_bdd_macros::{then, when};
use xylitol::app::tui::TuiHostEvent as HostEvent;
use xylitol::app::tui::TuiHostSession as HostSession;
use xylitol::app::tui::UiEntry;
use xylitol::app::tui::harness::{
    ScriptedDriver, TestTerminal, enter_event, esc_event, pump_host_driver,
};
use xylitol::protocol::Command;
use xylitol_tui::Component;

fn take_pump(bdd: &HostPumpBdd) -> HostPump {
    bdd.pump.borrow_mut().take().unwrap_or_else(|| HostPump {
        session: HostSession::new_product_ui(TestTerminal::new(120, 40)),
        driver: ScriptedDriver::new(),
    })
}

fn put_pump(bdd: &HostPumpBdd, pump: HostPump) {
    *bdd.pump.borrow_mut() = Some(pump);
}

fn scenes_open(session: &HostSession<TestTerminal>) -> bool {
    session.ui_root().expect("ui").borrow().scenes_open()
}

#[when("打开 designing 组合并打开场景槽")]
fn w_open_designing_slot(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_designing_mode(true);
    pump.session.open_designing_scene_slot();
    pump.session.render_now().expect("render");
    put_pump(host_pump_bdd, pump);
}

#[then("槽内列出场景 id 与描述")]
fn t_slot_lists_catalog(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let frame = root.borrow_mut().render(80).join("\n");
    put_pump(host_pump_bdd, pump);
    assert!(
        root.borrow().scenes_open(),
        "designing composition must open the scene slot"
    );
    assert!(
        frame.contains("session-tree-multiturn"),
        "slot must list scene id: {frame}"
    );
    assert!(
        frame.contains("Multi-turn") || frame.contains("MessageHistory"),
        "slot must list description: {frame}"
    );
}

#[when("在场景槽选中 {scene:string}")]
async fn w_select_scene(host_pump_bdd: &HostPumpBdd, scene: String) {
    let scene = scene.trim_matches('"').to_string();
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_designing_mode(true);
    if !scenes_open(&pump.session) {
        pump.session.open_designing_scene_slot();
    }
    pump.session
        .inject_designing_scene(&mut pump.driver, &scene)
        .await;
    pump.session.close_scenes_slot();
    put_pump(host_pump_bdd, pump);
}

#[then("画布为该场景且不再停留在空会话")]
fn t_canvas_injected(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let has_user = pump.session.ui_model().entries.iter().any(|e| {
        matches!(e, UiEntry::User { .. })
            || matches!(e, UiEntry::ScrollNotice { text } if text.contains("designing preview"))
    });
    put_pump(host_pump_bdd, pump);
    assert!(has_user, "canvas must leave the empty-session state");
}

#[then("画布为后一场景")]
fn t_canvas_switched(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let latest = pump
        .session
        .ui_model()
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEntry::ScrollNotice { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();
    put_pump(host_pump_bdd, pump);
    assert!(
        latest.contains("session-tree-labeled"),
        "second select must inject the later scene; got: {latest}"
    );
}

#[when("关闭场景槽")]
fn w_close_scene_slot(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(esc_event()))
        .expect("esc");
    put_pump(host_pump_bdd, pump);
}

#[when("再打开场景槽")]
async fn w_reopen_scene_slot(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    if let Some(root) = pump.session.ui_root() {
        root.borrow_mut().set_editor_text("/designing");
    }
    pump.session
        .step(HostEvent::Input(enter_event()))
        .expect("enter /designing");
    let mut stream = None;
    pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("pump /designing");
    put_pump(host_pump_bdd, pump);
}

#[when("designing 请求未知场景")]
async fn w_unknown_scene(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_designing_mode(true);
    pump.session
        .inject_designing_scene(&mut pump.driver, "no-such-scene")
        .await;
    put_pump(host_pump_bdd, pump);
}

#[then("提示错误且进程不崩溃")]
fn t_unknown_scene_errors(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let toast = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow()
        .toast_notice_body()
        .map(str::to_string);
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
    assert!(
        toast.is_some() || !note.is_empty(),
        "unknown scene must show a short error"
    );
}

#[when("designing 装载场景 {scene:string}")]
async fn w_designing_load_scene(host_pump_bdd: &HostPumpBdd, scene: String) {
    let scene = scene.trim_matches('"').to_string();
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_designing_mode(true);
    pump.session
        .inject_designing_scene(&mut pump.driver, &scene)
        .await;
    put_pump(host_pump_bdd, pump);
}

#[when("枚举跨进程公开命令")]
fn w_enum_wire_commands() {}

#[then("不含场景装载命令")]
fn t_no_scene_command_on_wire() {
    use strum::VariantNames;
    let names = Command::VARIANTS;
    assert!(
        names.iter().all(|n| {
            let lower = n.to_ascii_lowercase();
            !lower.contains("debug") && !lower.contains("scene") && !lower.contains("preview")
        }),
        "wire Command must not grow a scene-load variant: {names:?}"
    );
}

#[then("按未知斜杠提示且未换会话")]
fn t_unknown_slash_no_session_switch(host_pump_bdd: &HostPumpBdd) {
    t_unknown_slash_no_inject(host_pump_bdd);
}

#[then("按未知斜杠提示且未注入场景")]
fn t_unknown_slash_no_inject(host_pump_bdd: &HostPumpBdd) {
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
    let injected = pump.session.ui_model().entries.iter().any(|e| {
        matches!(e, UiEntry::ScrollNotice { text } if text.contains("designing preview") || text.contains("in-memory session"))
    });
    put_pump(host_pump_bdd, pump);
    assert!(!note.is_empty(), "unknown slash must prompt: {note}");
    assert!(!injected, "must not inject a scene via /debug");
}

#[then("按未知斜杠提示")]
fn t_unknown_slash(host_pump_bdd: &HostPumpBdd) {
    t_unknown_slash_no_inject(host_pump_bdd);
}
