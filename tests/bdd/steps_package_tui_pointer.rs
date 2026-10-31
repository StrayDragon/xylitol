use crate::bdd::prelude::*;
use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::{Component, Hit, InputEvent, InteractionMode, NullTerminal, TUI, Terminal};

const COLS: u16 = 40;
const ROWS: u16 = 10;
const DOCK: usize = 2;
const DOCK_HIT: u64 = 77;

struct BodyLines {
    rows: Vec<String>,
}

impl Component for BodyLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.rows.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
}

pub struct PointerDockBdd {
    tui: RefCell<Option<TUI<NullTerminal>>>,
    hit_before: Cell<Option<u64>>,
    hit_after: Cell<Option<u64>>,
    hover_id: Cell<Option<u64>>,
    transcript_before: RefCell<Vec<String>>,
    transcript_after: RefCell<Vec<String>>,
}

#[fixture]
pub fn pointer_dock_bdd() -> PointerDockBdd {
    PointerDockBdd {
        tui: RefCell::new(None),
        hit_before: Cell::new(None),
        hit_after: Cell::new(None),
        hover_id: Cell::new(None),
        transcript_before: RefCell::new(Vec::new()),
        transcript_after: RefCell::new(Vec::new()),
    }
}

fn dock_row() -> u16 {
    ROWS - 1
}

fn transcript_rows() -> u16 {
    ROWS - DOCK as u16
}

fn seeded_tui() -> TUI<NullTerminal> {
    let mut tui = TUI::with_interaction_mode(
        NullTerminal::new(COLS, ROWS),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(DOCK);
    tui.add_child(Box::new(BodyLines {
        rows: (0..24).map(|i| format!("body-{i:02}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    let _ = tui.render_now();
    tui
}

fn transcript_prefix(tui: &TUI<NullTerminal>) -> Vec<String> {
    tui.last_painted_lines()
        .iter()
        .take(transcript_rows() as usize)
        .cloned()
        .collect()
}

fn moved_event(col: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

#[given("登记一个终端绝对行的固定区命中区")]
fn given_dock_hit_region(pointer_dock_bdd: &PointerDockBdd) {
    let mut tui = seeded_tui();
    tui.mark_hit(Hit::at_screen(DOCK_HIT, 2, dock_row()).cols(0, 8));
    pointer_dock_bdd.tui.replace(Some(tui));
}

#[when("视口滚动后按该绝对行查询")]
fn when_scroll_then_query_dock_row(pointer_dock_bdd: &PointerDockBdd) {
    let mut slot = pointer_dock_bdd.tui.borrow_mut();
    let tui = slot.as_mut().expect("dock hit region registered first");
    pointer_dock_bdd
        .hit_before
        .set(tui.hit_id_at(2, dock_row()));
    let _ = tui.application_owned_scroll_by(-4);
    pointer_dock_bdd.hit_after.set(tui.hit_id_at(2, dock_row()));
}

#[then("仍命中同一 id 且该行超出 transcript 行数也不影响")]
fn then_dock_hit_survives_scroll(pointer_dock_bdd: &PointerDockBdd) {
    assert!(
        dock_row() >= transcript_rows(),
        "the dock row must sit outside the transcript band"
    );
    assert_eq!(
        pointer_dock_bdd.hit_before.get(),
        Some(DOCK_HIT),
        "dock row must hit before scrolling"
    );
    assert_eq!(
        pointer_dock_bdd.hit_after.get(),
        Some(DOCK_HIT),
        "the same absolute row must keep hitting after scrolling"
    );
}

#[given("固定区命中区已登记且探针已武装")]
fn given_dock_hit_with_probe_armed(pointer_dock_bdd: &PointerDockBdd) {
    let mut tui = seeded_tui();
    tui.mark_hit(Hit::at_screen(DOCK_HIT, 0, dock_row()).cols(0, 20));
    tui.arm_pointer_probe(true);
    pointer_dock_bdd
        .transcript_before
        .replace(transcript_prefix(&tui));
    pointer_dock_bdd.tui.replace(Some(tui));
}

#[when("悬停落在固定区命中区上")]
fn when_hover_lands_on_dock(pointer_dock_bdd: &PointerDockBdd) {
    let mut slot = pointer_dock_bdd.tui.borrow_mut();
    let tui = slot.as_mut().expect("dock hit region registered first");
    let _ = tui.dispatch_event(moved_event(1, dock_row()));
    pointer_dock_bdd.hover_id.set(tui.pointer_hover_id());
    tui.request_render(true);
    let _ = tui.render_now();
    pointer_dock_bdd
        .transcript_after
        .replace(transcript_prefix(tui));
}

#[then("悬停呈现不修改任何 transcript 行")]
fn then_hover_leaves_transcript_untouched(pointer_dock_bdd: &PointerDockBdd) {
    assert_eq!(
        pointer_dock_bdd.hover_id.get(),
        Some(DOCK_HIT),
        "the probe must report the dock hit so the check is not vacuous"
    );
    let before = pointer_dock_bdd.transcript_before.borrow();
    let after = pointer_dock_bdd.transcript_after.borrow();
    assert!(
        !before.is_empty(),
        "a transcript band must have been painted"
    );
    assert_eq!(
        *before, *after,
        "dock hover must not repaint any transcript row"
    );
}
