use crate::bdd::prelude::*;
use crate::bdd::steps_app_tui_host::{
    HostPump, HostPumpBdd, c2826_slot, c2911_click, put_pump, stats, step_key, take_pump,
};
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use rstest_bdd_macros::{given, then, when};
use xylitol_app_tui::McpDetailViewport;
use xylitol_app_tui::TuiHostEvent as HostEvent;
use xylitol_app_tui::TuiHostSession as HostSession;
use xylitol_app_tui::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};
use xylitol_app_tui::harness::{
    ScriptedDriver, TestTerminal, enter_event, key_event, pump_host_driver,
};
use xylitol_tui::{Component, InputEvent};

thread_local! {
    static LOADED_CALLS_BEFORE: Cell<usize> = const { Cell::new(0) };
}

fn long_details() -> Vec<String> {
    (0..8)
        .map(|i| format!("detail {i}: {}", "x".repeat(200)))
        .collect()
}

fn pump_with(width: u16, height: u16, details: Vec<String>) -> HostPump {
    let driver = ScriptedDriver::new();
    driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_connected: vec![("fixture".into(), 2)],
        mcp_servers: vec![McpServerSnapshot {
            id: "fixture".into(),
            phase: McpServerPhase::Connected,
            tools_armed: true,
            tool_count: 2,
        }],
        mcp_details: details,
        mcp_bootstrap_complete: true,
        tools_table_frozen: true,
        ..Default::default()
    });
    HostPump {
        session: HostSession::new_product_ui(TestTerminal::new(width, height)),
        driver,
    }
}

async fn open_mcp(host_pump_bdd: &HostPumpBdd, width: u16, height: u16, details: Vec<String>) {
    let mut pump = pump_with(width, height, details);
    pump.session.refresh_loaded_resources(&pump.driver).await;
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("/mcp");
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
    put_pump(host_pump_bdd, pump);
}

async fn open_mcp_on_existing(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    root.borrow_mut().set_editor_text("/mcp");
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
    put_pump(host_pump_bdd, pump);
}

fn render_now(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.render_now().expect("render");
    put_pump(host_pump_bdd, pump);
}

fn painted_lines(host_pump_bdd: &HostPumpBdd) -> Vec<String> {
    let mut pump = take_pump(host_pump_bdd);
    pump.session.render_now().expect("render");
    let lines = pump.session.tui.last_painted_lines().to_vec();
    put_pump(host_pump_bdd, pump);
    lines
}

fn frame(host_pump_bdd: &HostPumpBdd, width: usize) -> String {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let frame = root.borrow_mut().render(width).join("\n");
    put_pump(host_pump_bdd, pump);
    frame
}

fn detail_viewport(host_pump_bdd: &HostPumpBdd) -> Option<McpDetailViewport> {
    let pump = take_pump(host_pump_bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let viewport = root.borrow().mcp_detail_viewport();
    put_pump(host_pump_bdd, pump);
    viewport
}

fn loaded_calls(host_pump_bdd: &HostPumpBdd) -> usize {
    let pump = take_pump(host_pump_bdd);
    let calls = pump.driver.loaded_resources_snapshot_calls();
    put_pump(host_pump_bdd, pump);
    calls
}

fn wheel_down_at(host_pump_bdd: &HostPumpBdd, viewport: McpDetailViewport) {
    let event = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: (viewport.width / 2) as u16,
        row: viewport.top as u16,
        modifiers: KeyModifiers::NONE,
    });
    step_key(host_pump_bdd, event);
}

fn page_down(host_pump_bdd: &HostPumpBdd) {
    step_key(
        host_pump_bdd,
        key_event(KeyCode::PageDown, KeyModifiers::NONE),
    );
}

#[given("终端宽 100 列高 30 行且 `/mcp` 含超过 4 行的诊断详情")]
async fn g_wide_mcp_details(host_pump_bdd: &HostPumpBdd) {
    open_mcp(host_pump_bdd, 100, 30, long_details()).await;
}

#[when("渲染槽")]
fn w_render_slot(host_pump_bdd: &HostPumpBdd) {
    render_now(host_pump_bdd);
}

#[then("详情视口高度不超过 4 行且 status 与 editor 可见且详情可 PageDown 滚动")]
fn t_wide_detail_scroll_status_editor(host_pump_bdd: &HostPumpBdd) {
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    assert!(
        viewport.rows <= 4,
        "detail rows MUST be capped: {viewport:?}"
    );
    assert!(
        viewport.content_rows > viewport.rows,
        "detail content MUST overflow: {viewport:?}"
    );
    let lines = painted_lines(host_pump_bdd);
    assert_eq!(lines.len(), 30, "terminal frame MUST keep 30 rows");
    let plain = lines.join("\n");
    assert!(
        plain.contains("MCP"),
        "slot summary MUST be visible: {plain}"
    );
    assert!(
        plain.contains("Esc · Enter closes"),
        "slot footer MUST be visible: {plain}"
    );
    page_down(host_pump_bdd);
    render_now(host_pump_bdd);
    let after = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    assert!(after.scroll_top > 0, "PageDown MUST scroll detail");
}

#[given("终端高 16 行且 `/mcp` 槽打开")]
async fn g_short_terminal_mcp(host_pump_bdd: &HostPumpBdd) {
    open_mcp(host_pump_bdd, 100, 16, long_details()).await;
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Mcp");
}

#[then("槽总行数不超过 term_rows 且 status lead 可见")]
fn t_short_terminal_fits(host_pump_bdd: &HostPumpBdd) {
    let root_frame = frame(host_pump_bdd, 100);
    assert!(
        root_frame.lines().count() <= 16,
        "slot total rows MUST fit term_rows: {root_frame}"
    );
    let lines = painted_lines(host_pump_bdd);
    assert_eq!(lines.len(), 16, "terminal frame MUST keep 16 rows");
    let plain = lines.join("\n");
    assert!(
        plain.contains("MCP"),
        "slot summary MUST be visible: {plain}"
    );
    assert!(
        plain.contains("Esc · Enter closes"),
        "slot footer MUST be visible: {plain}"
    );
}

#[given("`/mcp` 详情超过视口高度")]
async fn g_detail_overflow(host_pump_bdd: &HostPumpBdd) {
    open_mcp(host_pump_bdd, 100, 30, long_details()).await;
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    assert!(viewport.content_rows > viewport.rows);
}

#[when("按 PageDown 到底")]
fn w_page_down_bottom(host_pump_bdd: &HostPumpBdd) {
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    let max = viewport.content_rows.saturating_sub(viewport.rows);
    let mut last = viewport.scroll_top;
    for _ in 0..20 {
        page_down(host_pump_bdd);
        render_now(host_pump_bdd);
        let top = detail_viewport(host_pump_bdd)
            .expect("mcp detail viewport")
            .scroll_top;
        if top == max || top == last {
            break;
        }
        last = top;
    }
    assert_eq!(
        detail_viewport(host_pump_bdd)
            .expect("mcp detail viewport")
            .scroll_top,
        max,
        "PageDown MUST reach bottom"
    );
}

#[then("末段字段可见且不触发 agent")]
fn t_tail_visible_no_agent(host_pump_bdd: &HostPumpBdd) {
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    assert_eq!(viewport.scroll_top, viewport.content_rows - viewport.rows);
    let plain = painted_lines(host_pump_bdd).join("\n");
    assert!(
        plain.contains("detail 7"),
        "tail detail MUST be visible at bottom: {plain}"
    );
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 0, "detail scroll MUST NOT abort agent");
    assert!(s.runs.is_empty(), "detail scroll MUST NOT run agent");
}

#[given("`/mcp` 详情视口已渲染")]
async fn g_detail_rendered(host_pump_bdd: &HostPumpBdd) {
    open_mcp(host_pump_bdd, 100, 30, long_details()).await;
    render_now(host_pump_bdd);
}

#[when("在视口内滚轮向下")]
fn w_wheel_down_inside(host_pump_bdd: &HostPumpBdd) {
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    wheel_down_at(host_pump_bdd, viewport);
    render_now(host_pump_bdd);
}

#[then("滚动位置前进且不启动 transcript 拖选")]
fn t_wheel_scroll_no_selection(host_pump_bdd: &HostPumpBdd) {
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    assert!(viewport.scroll_top > 0, "wheel MUST scroll detail");
    let pump = take_pump(host_pump_bdd);
    let selected = pump.session.tui.application_owned_has_selection();
    put_pump(host_pump_bdd, pump);
    assert!(!selected, "wheel MUST NOT start transcript selection");
}

#[given("详情视口已向下滚动")]
async fn g_detail_scrolled(host_pump_bdd: &HostPumpBdd) {
    open_mcp(host_pump_bdd, 100, 30, long_details()).await;
    render_now(host_pump_bdd);
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    wheel_down_at(host_pump_bdd, viewport);
    render_now(host_pump_bdd);
    assert!(
        detail_viewport(host_pump_bdd)
            .expect("mcp detail viewport")
            .scroll_top
            > 0
    );
}

#[when("左键点击 gutter 顶行")]
async fn w_click_gutter_top(host_pump_bdd: &HostPumpBdd) {
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    let column = viewport.width.saturating_sub(1) as u16;
    c2911_click(host_pump_bdd, column, viewport.top as u16).await;
    render_now(host_pump_bdd);
}

#[then("滚动位置归零")]
fn t_scroll_zero(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        detail_viewport(host_pump_bdd)
            .expect("mcp detail viewport")
            .scroll_top,
        0,
        "gutter top click MUST reset scroll"
    );
}

#[given("agent 正在运行且 `/mcp` 可打开")]
async fn g_busy_mcp_openable(host_pump_bdd: &HostPumpBdd) {
    let mut pump = pump_with(100, 30, long_details());
    pump.session.refresh_loaded_resources(&pump.driver).await;
    pump.session.on_run_started("hello");
    put_pump(host_pump_bdd, pump);
}

#[when("打开 `/mcp` 并滚动详情")]
async fn w_busy_open_and_scroll(host_pump_bdd: &HostPumpBdd) {
    open_mcp_on_existing(host_pump_bdd).await;
    render_now(host_pump_bdd);
    assert_eq!(format!("{:?}", c2826_slot(host_pump_bdd)), "Mcp");
    let viewport = detail_viewport(host_pump_bdd).expect("mcp detail viewport");
    LOADED_CALLS_BEFORE.with(|c| c.set(loaded_calls(host_pump_bdd)));
    wheel_down_at(host_pump_bdd, viewport);
    render_now(host_pump_bdd);
}

#[then("agent 未被 abort 且详情滚动不触发 host MCP 重连")]
fn t_busy_no_abort_no_reconnect(host_pump_bdd: &HostPumpBdd) {
    let s = stats(host_pump_bdd);
    assert_eq!(s.aborts, 0, "opening /mcp MUST NOT abort agent");
    assert!(s.is_busy, "agent MUST remain busy");
    assert!(
        detail_viewport(host_pump_bdd)
            .expect("mcp detail viewport")
            .scroll_top
            > 0,
        "detail MUST scroll while agent runs"
    );
    let before = LOADED_CALLS_BEFORE.with(|c| c.get());
    assert_eq!(
        loaded_calls(host_pump_bdd),
        before,
        "detail scroll MUST NOT trigger host resource reload"
    );
}
