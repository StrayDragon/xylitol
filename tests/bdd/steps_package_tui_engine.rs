use crate::bdd::prelude::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::{
    Component, InputEvent, InteractionMode, NullTerminal, OverlayAnchor, OverlayHandle,
    OverlayOptions, PanelBounds, PanelOverflow, PanelPlacement, PanelRegion, ScreenRect, SizeValue,
    TUI, Tabs, Text,
};

struct Recorder {
    events: std::sync::Arc<std::sync::Mutex<Vec<InputEvent>>>,
}

impl Component for Recorder {
    fn render(&mut self, _width: usize) -> Vec<String> {
        Vec::new()
    }
    fn handle_input(&mut self, event: InputEvent) {
        self.events.lock().unwrap().push(event);
    }
    fn invalidate(&mut self) {}
}

pub struct EngineOfferBdd {
    events: std::sync::Arc<std::sync::Mutex<Vec<InputEvent>>>,
    tui: RefCell<Option<TUI<NullTerminal>>>,
    click: RefCell<Option<InputEvent>>,
}

#[fixture]
pub fn engine_offer_bdd() -> EngineOfferBdd {
    EngineOfferBdd {
        events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        tui: RefCell::new(None),
        click: RefCell::new(None),
    }
}

#[given("根 child 会认领指针且另一组件持有焦点")]
fn g_tabs_claims_editor_focused(engine_offer_bdd: &EngineOfferBdd) {
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    let _ = tabs.render(80);
    let (_, start, end) = tabs.title_col_ranges()[1];
    let col = ((start + end) / 2) as u16;
    *engine_offer_bdd.click.borrow_mut() = Some(InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }));

    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(tabs));
    tui.add_child(Box::new(Recorder {
        events: engine_offer_bdd.events.clone(),
    }));
    tui.set_focus(Some(1));
    *engine_offer_bdd.tui.borrow_mut() = Some(tui);
}

#[when("dispatch_event 收到该指针")]
fn w_dispatch(engine_offer_bdd: &EngineOfferBdd) {
    let ev = engine_offer_bdd
        .click
        .borrow()
        .clone()
        .expect("click armed");
    engine_offer_bdd
        .tui
        .borrow_mut()
        .as_mut()
        .expect("tui")
        .dispatch_event(ev);
}

#[then("焦点组件收不到该事件")]
fn t_focus_missed(engine_offer_bdd: &EngineOfferBdd) {
    assert!(
        engine_offer_bdd.events.lock().unwrap().is_empty(),
        "focused child must not receive a claimed pointer"
    );
}

struct ViewportMarkLines {
    lines: Vec<String>,
}

impl Component for ViewportMarkLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.lines.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
    fn dock_rows_hint(&self) -> Option<usize> {
        Some(1)
    }
    fn overlay_viewport(&self, lines: &mut [String], _width: usize, transcript_rows: usize) {
        if transcript_rows == 0 || lines.is_empty() {
            return;
        }
        lines[0] = format!("{} VP-MARK", lines[0]);
    }
}

pub struct EngineViewportBdd {
    tui: RefCell<Option<TUI<NullTerminal>>>,
}

#[fixture]
pub fn engine_viewport_bdd() -> EngineViewportBdd {
    EngineViewportBdd {
        tui: RefCell::new(None),
    }
}

#[given("ApplicationOwned 已 ingest 长内容且根组件 overlay_viewport 会在可见顶行写标记")]
fn g_ao_ingest_mark(engine_viewport_bdd: &EngineViewportBdd) {
    let mut body: Vec<String> = (0..80).map(|i| format!("L{i:02}")).collect();
    body.push("dock".into());
    let mut tui =
        TUI::with_interaction_mode(NullTerminal::new(40, 12), InteractionMode::ApplicationOwned);
    tui.add_child(Box::new(ViewportMarkLines { lines: body }));
    tui.begin_application_owned_session();
    tui.render_frame().expect("ingest");
    *engine_viewport_bdd.tui.borrow_mut() = Some(tui);
}

#[when("只 reproject_frame 且 scroll_top 大于 0")]
fn w_reproject_scrolled(engine_viewport_bdd: &EngineViewportBdd) {
    let mut tui = engine_viewport_bdd.tui.borrow_mut();
    let tui = tui.as_mut().expect("tui");
    let notch = tui.application_owned_wheel_notch();
    assert!(tui.application_owned_scroll_by(-(notch * 4)));
    tui.clear_ao_reproject_frames_for_test();
    tui.render_frame().expect("reproject");
    assert!(tui.last_render_perf().ao_reprojected);
    assert!(tui.application_owned_scroll_top() > 0);
}

#[then("可见帧顶行含该标记且 ScrollView 内容第 0 行不含")]
fn t_mark_pins_not_content(engine_viewport_bdd: &EngineViewportBdd) {
    let tui = engine_viewport_bdd.tui.borrow();
    let tui = tui.as_ref().expect("tui");
    let painted0 = &tui.last_painted_lines()[0];
    let content0 = tui
        .application_owned_content_lines()
        .and_then(|c| c.first())
        .cloned()
        .unwrap_or_default();
    assert!(
        painted0.contains("VP-MARK"),
        "visible top must carry overlay mark: {painted0:?}"
    );
    assert!(
        !content0.contains("VP-MARK"),
        "ScrollView content[0] must stay unmarked: {content0:?}"
    );
}

struct PanelLines {
    rows: Vec<String>,
}

impl Component for PanelLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.rows.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
}

pub struct EnginePanelBdd {
    panel: RefCell<Option<PanelRegion>>,
    lines: RefCell<Vec<String>>,
    rect: RefCell<Option<ScreenRect>>,
}

#[fixture]
pub fn engine_panel_bdd() -> EnginePanelBdd {
    EnginePanelBdd {
        panel: RefCell::new(None),
        lines: RefCell::new(Vec::new()),
        rect: RefCell::new(None),
    }
}

#[given("PanelRegion 的 bounds 为 min_width 24 / max_width 40 / min_height 3 / max_height 10")]
fn g_panel_bounds(engine_panel_bdd: &EnginePanelBdd) {
    let panel = PanelRegion::new(
        Box::new(PanelLines {
            rows: (0..20).map(|i| format!("row-{i:02}")).collect(),
        }),
        PanelBounds {
            width: SizeValue::Absolute(30),
            min_width: 24,
            max_width: Some(40),
            min_height: 3,
            max_height: SizeValue::Absolute(10),
            overflow: PanelOverflow::Scroll,
        },
        PanelPlacement::DockRight,
    );
    engine_panel_bdd.panel.replace(Some(panel));
}

#[when("以宽度 30 渲染内容 20 行")]
fn w_panel_render(engine_panel_bdd: &EnginePanelBdd) {
    let mut slot = engine_panel_bdd.panel.borrow_mut();
    let panel = slot.as_mut().expect("panel bounds first");
    panel.set_available(ScreenRect {
        row: 0,
        col: 0,
        height: 10,
        width: 30,
    });
    let lines = panel.render(30);
    engine_panel_bdd.lines.replace(lines);
    engine_panel_bdd.rect.replace(panel.rect());
}

#[then("输出宽度不超过 40 且高度不超过 10 且不少于 3")]
fn t_panel_clamp(engine_panel_bdd: &EnginePanelBdd) {
    let lines = engine_panel_bdd.lines.borrow();
    assert!(
        (3..=10).contains(&lines.len()),
        "height must clamp to [3, 10], got {}",
        lines.len()
    );
    for line in lines.iter() {
        assert!(
            xylitol_tui::visible_width(line) <= 40,
            "width must clamp to 40, got {line:?}"
        );
    }
    let rect = engine_panel_bdd.rect.borrow().expect("rect after render");
    assert!(rect.width <= 40, "rect width must clamp to 40");
    assert!(
        (3..=10).contains(&rect.height),
        "rect height must clamp to [3, 10]"
    );
}

pub struct EngineOverlayBdd {
    tui: RefCell<Option<TUI<NullTerminal>>>,
    handle: RefCell<Option<OverlayHandle>>,
    clip: RefCell<Option<ScreenRect>>,
    painted: RefCell<Vec<String>>,
}

#[fixture]
pub fn engine_overlay_bdd() -> EngineOverlayBdd {
    EngineOverlayBdd {
        tui: RefCell::new(None),
        handle: RefCell::new(None),
        clip: RefCell::new(None),
        painted: RefCell::new(Vec::new()),
    }
}

#[given("overlay 锚点为 TopRight 且 clip 为 transcript 矩形")]
fn g_overlay_clip(engine_overlay_bdd: &EngineOverlayBdd) {
    let mut tui = TUI::new(NullTerminal::new(40, 20));
    tui.add_child(Box::new(PanelLines {
        rows: (0..20).map(|i| format!("line-{i:02}")).collect(),
    }));
    let clip = ScreenRect {
        row: 2,
        col: 5,
        height: 4,
        width: 10,
    };
    let handle = tui.show_overlay(
        Box::new(Text::new("OVERLAY".into(), 0, 0)),
        OverlayOptions {
            width: Some(SizeValue::Absolute(10)),
            max_width: Some(10),
            max_height: Some(SizeValue::Absolute(5)),
            min_height: Some(1),
            anchor: Some(OverlayAnchor::TopRight),
            clip: Some(clip),
            ..Default::default()
        },
    );
    engine_overlay_bdd.clip.replace(Some(clip));
    engine_overlay_bdd.handle.replace(Some(handle));
    engine_overlay_bdd.tui.replace(Some(tui));
}

#[when("composite_overlays 合成")]
fn w_overlay_composite(engine_overlay_bdd: &EngineOverlayBdd) {
    let mut slot = engine_overlay_bdd.tui.borrow_mut();
    let tui = slot.as_mut().expect("overlay first");
    tui.render_frame().expect("render");
    engine_overlay_bdd
        .painted
        .replace(tui.last_painted_lines().to_vec());
}

#[then("overlay 只出现在 clip 内且 last_screen_rect 为裁剪后矩形")]
fn t_overlay_clip(engine_overlay_bdd: &EngineOverlayBdd) {
    let clip = engine_overlay_bdd.clip.borrow().expect("clip");
    let painted = engine_overlay_bdd.painted.borrow();
    let mut found = false;
    for row in clip.row as usize..(clip.row as usize + clip.height as usize) {
        if painted
            .get(row)
            .is_some_and(|line| line.contains("OVERLAY"))
        {
            found = true;
        }
    }
    assert!(found, "overlay must be painted inside clip");
    for row in 0..clip.row as usize {
        assert!(
            !painted
                .get(row)
                .is_some_and(|line| line.contains("OVERLAY")),
            "overlay must not paint above clip"
        );
    }
    let tui = engine_overlay_bdd.tui.borrow();
    let tui = tui.as_ref().expect("tui");
    let rect = tui
        .overlay_screen_rect(engine_overlay_bdd.handle.borrow().expect("overlay handle"))
        .expect("clipped rect");
    assert!(rect.row >= clip.row && rect.col >= clip.col);
    assert!(rect.row + rect.height <= clip.row + clip.height);
    assert!(rect.col + rect.width <= clip.col + clip.width);
}
