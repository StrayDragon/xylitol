//! Steps for app-tui-preview (c2930): designing CLI leaf + scene slot + inject.

use crate::bdd::steps_app_tui_host::{HostPump, HostPumpBdd};
use rstest_bdd_macros::{then, when};
use xylitol::app::tui::TuiHostSession as HostSession;
use xylitol::app::tui::UiEntry;
use xylitol::app::tui::harness::{ScriptedDriver, TestTerminal};
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

#[when("打开预览组合并打开场景槽")]
fn w_open_preview_slot(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_preview_mode(true);
    pump.session.open_preview_scene_slot();
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
        "preview composition must open the scene slot"
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

#[when("在预览场景槽选中 {scene:string}")]
async fn w_select_scene(host_pump_bdd: &HostPumpBdd, scene: String) {
    let scene = scene.trim_matches('"').to_string();
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_preview_mode(true);
    pump.session.open_preview_scene_slot();
    pump.session
        .inject_preview_scene(&mut pump.driver, &scene)
        .await;
    put_pump(host_pump_bdd, pump);
}

#[then("画布为该场景且不再停留在空会话")]
fn t_canvas_injected(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let has_user = pump.session.ui_model().entries.iter().any(|e| {
        matches!(e, UiEntry::User { .. })
            || matches!(e, UiEntry::ScrollNotice { text } if text.contains("preview scene"))
    });
    put_pump(host_pump_bdd, pump);
    assert!(has_user, "canvas must leave the empty-session state");
}

#[when("预览组合请求未知场景")]
async fn w_unknown_scene(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_preview_mode(true);
    pump.session
        .inject_preview_scene(&mut pump.driver, "no-such-scene")
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

#[when("预览表面装载场景 {scene:string}")]
async fn w_preview_load_scene(host_pump_bdd: &HostPumpBdd, scene: String) {
    let scene = scene.trim_matches('"').to_string();
    let mut pump = take_pump(host_pump_bdd);
    pump.session.set_preview_mode(true);
    pump.session
        .inject_preview_scene(&mut pump.driver, &scene)
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
        matches!(e, UiEntry::ScrollNotice { text } if text.contains("preview scene") || text.contains("in-memory session"))
    });
    put_pump(host_pump_bdd, pump);
    assert!(!note.is_empty(), "unknown slash must prompt: {note}");
    assert!(!injected, "must not inject a scene via /debug");
}

#[then("按未知斜杠提示")]
fn t_unknown_slash(host_pump_bdd: &HostPumpBdd) {
    t_unknown_slash_no_inject(host_pump_bdd);
}
