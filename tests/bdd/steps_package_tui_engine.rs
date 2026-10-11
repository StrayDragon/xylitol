//! Steps for `package-tui-engine` r1946 (c2880 pointer offer).

use crate::bdd::prelude::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::{Component, InputEvent, InteractionMode, NullTerminal, TUI, Tabs};

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
