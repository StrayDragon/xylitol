//! Steps for `package-tui-engine` r1946 (c2880 pointer offer).

use crate::bdd::prelude::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::terminal::Terminal;
use xylitol_tui::{Component, InputEvent, TUI, Tabs};

struct DummyTerminal;

impl Terminal for DummyTerminal {
    fn write(&mut self, _data: &str) {}
    fn columns(&self) -> u16 {
        80
    }
    fn rows(&self) -> u16 {
        24
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
}

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
    tui: RefCell<Option<TUI<DummyTerminal>>>,
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

    let mut tui = TUI::new(DummyTerminal);
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
