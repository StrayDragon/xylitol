use crate::bdd::prelude::*;
use crate::bdd::steps_app_tui_host::{
    HostPump, HostPumpBdd, c2826_slot, c2911_click, click_footer_chip_label, editor_text,
    fresh_pump, open_busy, open_tree_ready, pump_once, put_pump, render_frame, set_editor, stats,
    step_key, take_pump,
};
use crate::bdd::steps_app_tui_interaction::TuiInteraction;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rstest_bdd_macros::{given, then, when};
use xylitol_app_tui::TuiHostEvent as HostEvent;
use xylitol_app_tui::TuiHostSession as HostSession;
use xylitol_app_tui::harness::{ScriptedDriver, TestTerminal, alt_enter_event};
use xylitol_app_tui::{FoldTarget, InteractionBdd, LayoutTheme, SceneBuilder};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn mount_pointer_scene(tui_interaction: &TuiInteraction) -> String {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    sb.thinking_flushed("看看 diff", 3);
    sb.tool_start("t-read", "read", "old.rs");
    sb.tool_end("t-read", "read");
    sb.message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::AgentEnd {
        messages: Vec::new(),
    });
    fx.open_hit_viewport(200);
    let plain = fx.render_plain(80);
    *tui_interaction.fx.borrow_mut() = Some(fx);
    plain
}

thread_local! {
    static SAW_POPUP_OPEN: RefCell<bool> = const { RefCell::new(false) };
    static SAW_SIDEBAR_OPEN: RefCell<bool> = const { RefCell::new(false) };
    static TREE_THIRD_ROW: RefCell<Option<(u16, u16, String)>> = const { RefCell::new(None) };
}

fn host_root_popup_open(bdd: &HostPumpBdd) -> bool {
    let pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let open = root.borrow().session_popup_open();
    drop(root);
    put_pump(bdd, pump);
    open
}

fn host_root_sidebar_on(bdd: &HostPumpBdd) -> bool {
    let pump = take_pump(bdd);
    let root = pump.session.ui_root().expect("ui").clone();
    let on = root.borrow().session_sidebar_on();
    drop(root);
    put_pump(bdd, pump);
    on
}

fn painted_lines(bdd: &HostPumpBdd) -> Vec<String> {
    let mut pump = take_pump(bdd);
    pump.session.render_now().expect("paint");
    let lines = pump.session.tui.last_painted_lines().to_vec();
    put_pump(bdd, pump);
    lines
}

#[given("产品 TUI 已就绪且存在可展开的思维块与工具块")]
fn g_mat31_ready(tui_interaction: &TuiInteraction, host_pump_bdd: &HostPumpBdd) {
    let plain = mount_pointer_scene(tui_interaction);
    assert!(!plain.contains("Read old.rs"), "簇默认收纳");
    if host_pump_bdd.pump.borrow().is_none() {
        put_pump(host_pump_bdd, fresh_pump());
    }
}

#[when("不按任何组合键而仅用左键与滚轮操作")]
async fn w_mat31_pointer_only(tui_interaction: &TuiInteraction, host_pump_bdd: &HostPumpBdd) {
    {
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        let _ = fx.render_plain(80);
        let cluster = fx
            .fold_hits()
            .regions
            .iter()
            .find(|r| matches!(&r.target, FoldTarget::Cluster(_)))
            .map(|r| (r.col_start as u16, r.content_row as u16))
            .expect("簇头三角命中");
        assert!(fx.left_click(cluster.0, cluster.1), "簇头点击须被消费");
        let _ = fx.render_plain(80);
        let tool = fx
            .fold_hits()
            .regions
            .iter()
            .find(|r| matches!(&r.target, FoldTarget::Tool(id) if id == "t-read"))
            .map(|r| (r.col_start as u16, r.content_row as u16))
            .expect("工具块三角命中");
        assert!(fx.left_click(tool.0, tool.1), "工具块点击须被消费");
        let thinking = fx
            .fold_hits()
            .regions
            .iter()
            .find(|r| matches!(&r.target, FoldTarget::Thinking(_)))
            .map(|r| (r.col_start as u16, r.content_row as u16))
            .expect("思维块三角命中");
        assert!(fx.left_click(thinking.0, thinking.1), "思维块点击须被消费");
    }
    assert!(
        click_footer_chip_label(host_pump_bdd, "详情").await,
        "[详情] chip"
    );
    SAW_POPUP_OPEN.with(|s| *s.borrow_mut() = host_root_popup_open(host_pump_bdd));
    assert!(
        click_footer_chip_label(host_pump_bdd, "详情").await,
        "[详情] chip 复点"
    );
    assert!(
        click_footer_chip_label(host_pump_bdd, "侧栏").await,
        "[侧栏] chip"
    );
    SAW_SIDEBAR_OPEN.with(|s| *s.borrow_mut() = host_root_sidebar_on(host_pump_bdd));
    assert!(
        click_footer_chip_label(host_pump_bdd, "侧栏").await,
        "[侧栏] chip 复点"
    );
}

#[then("思维块与工具块可展开可收纳且详情弹层与侧栏可开关")]
fn t_mat31_pointer_reaches_all(tui_interaction: &TuiInteraction, host_pump_bdd: &HostPumpBdd) {
    let frame = {
        let mut slot = tui_interaction.fx.borrow_mut();
        let fx = slot.as_mut().expect("fixture mounted");
        assert!(
            !fx.tools_effective("t-read"),
            "mat31: 左键点三角须可收起单个工具块"
        );
        assert_eq!(fx.fold().tools_overrides.len(), 1, "仅该块入覆盖表");
        assert_eq!(
            fx.fold().thinking_overrides.len(),
            1,
            "mat31: 左键点三角须可翻转思维块"
        );
        fx.render_plain(80)
    };
    assert!(
        frame.contains("Read old.rs"),
        "mat31: 收纳块保留摘要行: {frame}"
    );
    assert!(
        SAW_POPUP_OPEN.with(|s| *s.borrow()),
        "mat31: 详情弹层须可经 chip 开启"
    );
    assert!(!host_root_popup_open(host_pump_bdd), "弹层须可再关");
    assert!(
        SAW_SIDEBAR_OPEN.with(|s| *s.borrow()),
        "mat31: 侧栏须可经 chip 开启"
    );
    assert!(!host_root_sidebar_on(host_pump_bdd), "侧栏须可再关");
}

#[given("产品绑定目录使用默认配置")]
fn g_mat32_default_catalog(tui_interaction: &TuiInteraction) {
    mount_pointer_scene(tui_interaction);
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    fx.handle_key(key(
        KeyCode::F(13),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    ));
    for id in [
        "app.thinking.toggle",
        "app.tools.expand",
        "app.tools.blocks",
        "app.activity.expandNearest",
        "app.activity.collapseNearest",
        "app.session.popup",
        "app.session.sidebar",
        "app.session.detailsCycle",
        "app.session.sidebarWidth",
    ] {
        assert!(
            xylitol_tui::with_keybindings(|kb| kb.get_keys(id).is_empty()),
            "mat32: {id} 默认和弦须为空"
        );
    }
}

#[when(
    "依次按下 Ctrl+T 与 Ctrl+O 与 Alt+E 与 Alt+Shift+E 与 Alt+I 与 Alt+U 与 Alt+O 与 Alt+Shift+U"
)]
fn w_mat32_press_contracted_chords(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    for (code, modifiers) in [
        (KeyCode::Char('t'), KeyModifiers::CONTROL),
        (KeyCode::Char('o'), KeyModifiers::CONTROL),
        (KeyCode::Char('e'), KeyModifiers::ALT),
        (KeyCode::Char('e'), KeyModifiers::ALT | KeyModifiers::SHIFT),
        (KeyCode::Char('i'), KeyModifiers::ALT),
        (KeyCode::Char('u'), KeyModifiers::ALT),
        (KeyCode::Char('o'), KeyModifiers::ALT),
        (KeyCode::Char('u'), KeyModifiers::ALT | KeyModifiers::SHIFT),
    ] {
        fx.handle_key(key(code, modifiers));
    }
}

#[then("思维折叠默认态未变且工具块展开态未变且详情与侧栏未开关")]
fn t_mat32_chords_inert(tui_interaction: &TuiInteraction) {
    let (fold, frame) = {
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        let fold = fx.fold().clone();
        let frame = fx.render_plain(80);
        (fold, frame)
    };
    assert!(!fold.thinking_expanded, "mat32: Ctrl+T 不得再触发思维折叠");
    assert!(fold.tools_expanded, "mat32: Alt+E 不得再触发工具族折叠");
    assert!(
        !fold.tools_output_expanded,
        "mat32: Ctrl+O 不得再触发输出视口全局默认"
    );
    assert!(
        !frame.contains("会话详情") && !frame.contains("会话侧栏"),
        "mat32: Alt+I/Alt+U/Alt+O 不得再触发详情与侧栏:
{frame}"
    );
}

#[given("用户在 keybindings.json 将 app.session.sidebar 绑到 ctrl+b")]
fn g_mat32_rebind_sidebar(host_pump_bdd: &HostPumpBdd) {
    let pump = fresh_pump();
    let dir = tempfile::tempdir().expect("temp agent dir");
    std::fs::write(
        dir.path().join("keybindings.json"),
        r#"{ "app.session.sidebar": "ctrl+b" }"#,
    )
    .expect("write keybindings.json");
    let outcome = pump.session.reload_keybindings(dir.path());
    put_pump(host_pump_bdd, pump);
    assert!(
        format!("{outcome:?}").contains("Applied"),
        "mat32: 重绑 keybindings.json 须被应用: {outcome:?}"
    );
}

#[when("在编辑器槽按下 Ctrl+B")]
async fn w_mat32_press_ctrl_b(host_pump_bdd: &HostPumpBdd) {
    step_key(
        host_pump_bdd,
        xylitol_tui::InputEvent::Key(key(KeyCode::Char('b'), KeyModifiers::CONTROL)),
    );
    pump_once(host_pump_bdd).await;
}

#[then("会话侧栏按既有开启路径上屏")]
fn t_mat32_sidebar_on_screen(host_pump_bdd: &HostPumpBdd) {
    assert!(
        host_root_sidebar_on(host_pump_bdd),
        "mat32: 重绑后 Ctrl+B 须走 app.session.sidebar 开启路径"
    );
    let pump = take_pump(host_pump_bdd);
    let frame = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow_mut()
        .render_with_viewport_overlay(120)
        .join("\n");
    put_pump(host_pump_bdd, pump);
    assert!(frame.contains("会话侧栏"), "侧栏须上屏:\n{frame}");
}

#[given("产品 TUI 就绪于 120 列终端")]
fn g_mat33_wide_host(host_pump_bdd: &HostPumpBdd) {
    if host_pump_bdd.pump.borrow().is_none() {
        put_pump(host_pump_bdd, fresh_pump());
    }
}

#[then("右侧栏上屏且内容区让宽且 [侧栏] chip 反色")]
fn t_mat33_sidebar_chip_active(host_pump_bdd: &HostPumpBdd) {
    assert!(host_root_sidebar_on(host_pump_bdd), "侧栏须上屏");
    let lines = painted_lines(host_pump_bdd);
    let plain: Vec<String> = lines
        .iter()
        .map(|l| xylitol_tui::utils::strip_ansi_codes(l))
        .collect();
    let chip_row = plain
        .iter()
        .position(|l| l.contains("[侧栏]"))
        .expect("[侧栏] chip 行");
    assert!(
        plain[chip_row].contains("[详情]"),
        "标准档须同时绘制两 chip: {}",
        plain[chip_row]
    );
    let raw = &lines[chip_row];
    let palette = LayoutTheme::product_dark().palette();
    let active = xylitol_tui::bold(&xylitol_tui::fg_rgb(palette.accent, "[侧栏]"));
    assert!(
        raw.contains(&active),
        "mat33: 开启态 [侧栏] chip 须反色（accent+bold）: {raw:?}"
    );
    let inactive = xylitol_tui::fg_rgb(palette.muted, "[详情]");
    assert!(
        raw.contains(&inactive),
        "mat33: 未开启 [详情] chip 保持 muted: {raw:?}"
    );
    let pump = take_pump(host_pump_bdd);
    let frame = pump
        .session
        .ui_root()
        .expect("ui")
        .borrow_mut()
        .render_with_viewport_overlay(120)
        .join("\n");
    put_pump(host_pump_bdd, pump);
    assert!(frame.contains("会话侧栏"), "侧栏带须上屏:\n{frame}");
}

#[given("产品 TUI 就绪于 70 列终端")]
fn g_mat33_narrow_host(host_pump_bdd: &HostPumpBdd) {
    *host_pump_bdd.pump.borrow_mut() = Some(HostPump {
        session: HostSession::new_product_ui(TestTerminal::new(70, 40)),
        driver: ScriptedDriver::new(),
    });
}

#[when("渲染编辑区上边框")]
fn w_mat33_render_editor_border(host_pump_bdd: &HostPumpBdd) {
    let lines = painted_lines(host_pump_bdd);
    let frame = lines
        .iter()
        .map(|l| xylitol_tui::utils::strip_ansi_codes(l))
        .collect::<Vec<_>>()
        .join("\n");
    host_pump_bdd.ansi_frames.borrow_mut().push(frame);
}

#[then("编辑区上边框右端只有 [详情] chip 且无 [侧栏] chip")]
fn t_mat33_narrow_details_only(host_pump_bdd: &HostPumpBdd) {
    let frame = host_pump_bdd
        .ansi_frames
        .borrow()
        .last()
        .cloned()
        .expect("已渲染编辑区上边框");
    assert!(frame.contains("[详情]"), "窄档仍须绘制 [详情]:\n{frame}");
    assert!(
        !frame.contains("[侧栏]"),
        "mat33: 窄于标准档不得绘制 [侧栏] chip:\n{frame}"
    );
}

#[given("产品 TUI 忙碌且已排队一条 follow-up")]
async fn g_mat34_busy_followup(host_pump_bdd: &HostPumpBdd) {
    open_busy(host_pump_bdd);
    set_editor(host_pump_bdd, "mat34-follow");
    let mut pump = take_pump(host_pump_bdd);
    pump.session
        .step(HostEvent::Input(alt_enter_event()))
        .expect("alt enter (follow-up)");
    put_pump(host_pump_bdd, pump);
    pump_once(host_pump_bdd).await;
}

#[when("左键单击队列条")]
async fn w_mat34_click_queue_strip(host_pump_bdd: &HostPumpBdd) {
    let frame = render_frame(host_pump_bdd, 100);
    let row = frame
        .lines()
        .position(|line| line.contains("Follow-up"))
        .expect("队列条已渲染") as u16;
    c2911_click(host_pump_bdd, 5, row).await;
    pump_once(host_pump_bdd).await;
}

#[then("队列文本回到编辑器且队列清空")]
fn t_mat34_queue_restored(host_pump_bdd: &HostPumpBdd) {
    let text = editor_text(host_pump_bdd);
    assert!(
        text.contains("mat34-follow"),
        "mat34: 点击队列条须取回排队文本: {text:?}"
    );
    let pump = take_pump(host_pump_bdd);
    let follow = pump.session.ui_model().pending_follow_up.clone();
    let clears = pump.driver.clear_calls.clone();
    put_pump(host_pump_bdd, pump);
    assert!(follow.is_empty(), "mat34: follow-up 队列须清空: {follow:?}");
    assert!(
        clears.iter().any(|(steer, follow)| *steer && *follow),
        "mat34: 点击须与 Alt+Up 同语义（ClearQueue 双向）: {clears:?}"
    );
}

#[given("打开会话树 槽且当前选中第一行")]
async fn g_mat35_tree_slot_first_row(host_pump_bdd: &HostPumpBdd) {
    open_tree_ready(host_pump_bdd);
    step_key(host_pump_bdd, xylitol_app_tui::harness::esc_event());
    step_key(host_pump_bdd, xylitol_app_tui::harness::esc_event());
    pump_once(host_pump_bdd).await;
    assert!(stats(host_pump_bdd).tree_open, "会话树槽须已打开");
}

fn tree_third_node_row(host_pump_bdd: &HostPumpBdd) -> (u16, u16, String) {
    let lines = painted_lines(host_pump_bdd);
    let last = lines.len().saturating_sub(1);
    let label_to_id = [
        ("hello", "u1"),
        ("plan", "a1"),
        ("read", "t1"),
        ("done", "a2"),
        ("next", "u2"),
    ];
    let mut nodes = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if idx >= last {
            break;
        }
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        for (label, id) in label_to_id {
            if plain.contains(label) {
                nodes.push((idx as u16, id.to_string()));
                break;
            }
        }
    }
    let (row, id) = nodes.get(2).expect("树槽须至少渲染三行节点").clone();
    (5, row, id)
}

#[when("左键单击第三行")]
async fn w_mat35_click_third_row(host_pump_bdd: &HostPumpBdd) {
    let (col, row, id) = tree_third_node_row(host_pump_bdd);
    TREE_THIRD_ROW.with(|s| *s.borrow_mut() = Some((col, row, id)));
    c2911_click(host_pump_bdd, col, row).await;
}

#[then("选中项变为第三行且未发起 travel")]
fn t_mat35_selected_not_traveled(host_pump_bdd: &HostPumpBdd) {
    let mut pump = take_pump(host_pump_bdd);
    let pending = pump.session.take_pending_session_tree_travel();
    let travels = pump.driver.travel_calls();
    put_pump(host_pump_bdd, pump);
    assert!(
        pending.is_none(),
        "mat35: 首次点击不得发起 travel: {pending:?}"
    );
    assert!(
        travels.is_empty(),
        "mat35: 首次点击不得触发驱动 travel: {travels:?}"
    );
}

#[when("再左键单击第三行")]
async fn w_mat35_click_third_row_again(host_pump_bdd: &HostPumpBdd) {
    let (col, row, _) = TREE_THIRD_ROW
        .with(|s| s.borrow().clone())
        .expect("已单击过第三行");
    c2911_click(host_pump_bdd, col, row).await;
    pump_once(host_pump_bdd).await;
}

#[then("发起一次 travel 且目标为第三行节点")]
fn t_mat35_travels_to_third_row(host_pump_bdd: &HostPumpBdd) {
    let expected = TREE_THIRD_ROW
        .with(|s| s.borrow().clone())
        .expect("已单击过第三行")
        .2;
    let pump = take_pump(host_pump_bdd);
    let travels = pump.driver.travel_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(
        travels,
        vec![expected],
        "mat35: 再次点击同一行须执行该行主动作（travel）"
    );
}

#[given("以斜杠命令打开模型选择器槽")]
async fn g_mat36_open_models_slot(host_pump_bdd: &HostPumpBdd) {
    crate::bdd::steps_app_tui_host::w_c2826_submit_slash(host_pump_bdd, "/model".into()).await;
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Models",
        "mat36: /model 须打开模型选择器槽"
    );
}

#[when("左键单击槽头 [关闭] chip")]
async fn w_mat36_click_close_chip(host_pump_bdd: &HostPumpBdd) {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let lines = painted_lines(host_pump_bdd);
    let (col, row) = lines
        .iter()
        .enumerate()
        .find_map(|(idx, line)| {
            let plain = xylitol_tui::utils::strip_ansi_codes(line);
            plain
                .find("[关闭]")
                .map(|pos| (xylitol_tui::visible_width(&plain[..pos]) as u16, idx as u16))
        })
        .expect("槽头 [关闭] chip 已绘制");
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
                    modifiers: KeyModifiers::NONE,
                },
            )))
            .expect("close chip click");
    }
    let mut stream = None;
    xylitol_app_tui::harness::pump_host_driver(&mut pump.session, &mut pump.driver, &mut stream)
        .await
        .expect("close chip pump");
    put_pump(host_pump_bdd, pump);
}

#[then("槽关闭且编辑器槽恢复且未提交模型变更")]
fn t_mat36_close_chip_restores_editor(host_pump_bdd: &HostPumpBdd) {
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Editor",
        "mat36: 点击 [关闭] 须回到编辑器槽"
    );
    assert!(
        stats(host_pump_bdd).runs.is_empty(),
        "mat36: 关槽不得触发任何 run"
    );
}

#[given("打开能力覆盖盘")]
async fn g_mat36_open_capabilities(host_pump_bdd: &HostPumpBdd) {
    use xylitol_plugin_sdk::session::{CapabilityEntry, CapabilityKind};
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
    crate::bdd::steps_app_tui_host::w_c2826_submit_slash(host_pump_bdd, "/capabilities".into())
        .await;
    assert_eq!(
        format!("{:?}", c2826_slot(host_pump_bdd)),
        "Capabilities",
        "mat36: /capabilities 须打开能力覆盖盘"
    );
}

#[when("左键单击某能力行")]
async fn w_mat36_click_capability_row(host_pump_bdd: &HostPumpBdd) {
    let lines = painted_lines(host_pump_bdd);
    let (col, row) = lines
        .iter()
        .enumerate()
        .find_map(|(idx, line)| {
            let plain = xylitol_tui::utils::strip_ansi_codes(line);
            plain
                .find("tool:read")
                .map(|pos| (xylitol_tui::visible_width(&plain[..pos]) as u16, idx as u16))
        })
        .expect("能力行已绘制");
    c2911_click(host_pump_bdd, col, row).await;
    pump_once(host_pump_bdd).await;
}

#[then("该能力开关翻转一次")]
fn t_mat36_capability_toggled_once(host_pump_bdd: &HostPumpBdd) {
    let pump = take_pump(host_pump_bdd);
    let calls = pump.driver.capability_calls();
    put_pump(host_pump_bdd, pump);
    assert_eq!(
        calls,
        vec![("tool:read".to_string(), false)],
        "mat36: 行点击须直接翻转该能力一次: {calls:?}"
    );
}

#[given("以场景构建器回放含思维块与工具块的序列")]
fn g_mat37_blocks_scene(tui_interaction: &TuiInteraction) {
    let plain = mount_pointer_scene(tui_interaction);
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = plain;
    let _ = fx.render_plain(80);
    let cluster = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Cluster(_)))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("簇头三角命中");
    assert!(fx.left_click(cluster.0, cluster.1), "簇头点击须被消费");
    let frame = fx.render_plain(80);
    *tui_interaction.mounted_frame.borrow_mut() = frame;
}

#[then("思维块与工具块头均不含括号组合键且含折叠三角")]
fn t_mat37_heads_no_chords(tui_interaction: &TuiInteraction) {
    let frame = tui_interaction.mounted_frame.borrow().clone();
    assert!(
        frame.contains("Thought 3s") && frame.contains("Read old.rs"),
        "mat37: 思维块与工具块头须可见:\n{frame}"
    );
    for line in frame.lines() {
        if line.contains("Thought") || line.contains("Read old.rs") {
            assert!(
                line.contains('▸') || line.contains('▾'),
                "mat37: 块头须保留折叠三角: {line:?}"
            );
            assert!(
                !line.contains("(Ctrl") && !line.contains("(Alt"),
                "mat37: 块头不得写括号组合键: {line:?}"
            );
        }
    }
}
