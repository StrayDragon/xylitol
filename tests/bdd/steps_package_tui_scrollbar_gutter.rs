use crate::bdd::prelude::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{Component, InteractionMode, NullTerminal, ScrollGutter, TUI};

const COLS: u16 = 40;
const ROWS: u16 = 8;
const DOCK: usize = 2;
const GUTTER_COL: usize = 39;

fn long_body() -> Vec<String> {
    (0..20)
        .map(|i| format!("L{i:0>3}{}", "x".repeat(35)))
        .collect()
}

struct ScriptLines {
    rows: Vec<String>,
}

impl Component for ScriptLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.rows.clone()
    }
    fn handle_input(&mut self, _event: xylitol_tui::InputEvent) {}
    fn invalidate(&mut self) {}
}

fn long_component() -> Box<dyn Component> {
    Box::new(ScriptLines { rows: long_body() })
}

fn short_component() -> Box<dyn Component> {
    Box::new(ScriptLines {
        rows: vec![
            "alpha".into(),
            "beta".into(),
            "dock-a".into(),
            "dock-b".into(),
        ],
    })
}

pub struct GutterBdd {
    tui: RefCell<Option<TUI<NullTerminal>>>,
}

#[fixture]
pub fn gutter_bdd() -> GutterBdd {
    GutterBdd {
        tui: RefCell::new(None),
    }
}

fn seed(component: Box<dyn Component>) -> TUI<NullTerminal> {
    let mut tui = TUI::with_interaction_mode(
        NullTerminal::new(COLS, ROWS),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(DOCK);
    tui.add_child(component);
    tui.begin_application_owned_session();
    tui.set_scroll_gutter(Some(ScrollGutter));
    tui.render_frame().expect("seed paint");
    tui
}

fn seeded(gutter_bdd: &GutterBdd) -> std::cell::RefMut<'_, Option<TUI<NullTerminal>>> {
    gutter_bdd.tui.borrow_mut()
}

fn plain(line: &str) -> String {
    strip_ansi_codes(line).to_string()
}

#[given("ApplicationOwned 会话开启滚动槽")]
fn g_enabled(gutter_bdd: &GutterBdd) {
    *seeded(gutter_bdd) = Some(seed(long_component()));
}

fn consume_gutter_click(tui: &mut TUI<NullTerminal>) {
    match tui.take_pointer_click() {
        Some(xylitol_tui::GUTTER_TOP_HIT_ID) => {
            tui.application_owned_scroll_home();
        }
        Some(xylitol_tui::GUTTER_BOTTOM_HIT_ID) => {
            tui.application_owned_scroll_to_end();
        }
        _ => {}
    }
}

#[given("ApplicationOwned 会话开启滚动槽且内容超过一屏")]
fn g_overflow(gutter_bdd: &GutterBdd) {
    *seeded(gutter_bdd) = Some(seed(long_component()));
}

#[given("ApplicationOwned 会话开启滚动槽但内容不超过一屏")]
fn g_short(gutter_bdd: &GutterBdd) {
    *seeded(gutter_bdd) = Some(seed(short_component()));
}

#[given("ApplicationOwned 会话开启滚动槽且已向下滚动离开顶部")]
fn g_scrolled_away(gutter_bdd: &GutterBdd) {
    let mut tui = seed(long_component());
    assert!(tui.application_owned_scroll_by(-3));
    *seeded(gutter_bdd) = Some(tui);
}

#[given("ApplicationOwned 会话开启滚动槽且已向上滚动离开底部")]
fn g_scrolled_up(gutter_bdd: &GutterBdd) {
    let mut tui = seed(long_component());
    assert!(tui.application_owned_scroll_by(-3));
    *seeded(gutter_bdd) = Some(tui);
}

#[when("渲染任一帧")]
fn w_render(gutter_bdd: &GutterBdd) {
    gutter_bdd
        .tui
        .borrow_mut()
        .as_mut()
        .expect("seeded")
        .render_frame()
        .expect("repaint");
}

#[when("渲染")]
fn w_render_any(gutter_bdd: &GutterBdd) {
    w_render(gutter_bdd);
}

fn dispatch_click(tui: &mut TUI<NullTerminal>, kind: MouseEventKind, col: u16, row: u16) {
    let _ = tui.dispatch_event(xylitol_tui::InputEvent::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }));
}

#[when("左键单击跳转顶部按钮字符")]
fn w_click_top(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    tui.replace_hits(tui.scroll_gutter_hit_set());
    let col = COLS - 1;
    dispatch_click(tui, MouseEventKind::Down(MouseButton::Left), col, 0);
    dispatch_click(tui, MouseEventKind::Up(MouseButton::Left), col, 0);
    consume_gutter_click(tui);
}

#[when("左键单击跳转底部按钮字符")]
fn w_click_bottom(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    let row = ROWS - DOCK as u16 - 1;
    let col = COLS - 1;
    tui.replace_hits(tui.scroll_gutter_hit_set());
    dispatch_click(tui, MouseEventKind::Down(MouseButton::Left), col, row);
    dispatch_click(tui, MouseEventKind::Up(MouseButton::Left), col, row);
    consume_gutter_click(tui);
}

#[when("在最右列按住左键拖动")]
fn w_drag_gutter(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    tui.replace_hits(tui.scroll_gutter_hit_set());
    let col = COLS - 1;
    dispatch_click(tui, MouseEventKind::Down(MouseButton::Left), col, 0);
    dispatch_click(tui, MouseEventKind::Drag(MouseButton::Left), col, 2);
    dispatch_click(tui, MouseEventKind::Up(MouseButton::Left), col, 2);
}

#[then("transcript 最右一列为滚动槽且正文按少一列排版")]
fn t_stamped(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    let painted = tui.last_painted_lines();
    let top = plain(painted.first().expect("viewport row").as_str());
    assert!(
        top.ends_with('^'),
        "top row ends with gutter glyph: {top:?}"
    );
    assert_eq!(
        top.chars().nth(GUTTER_COL - 1),
        Some('x'),
        "body preserved at col 38 (carve): {top:?}"
    );
}

#[then("不绘制滚动槽且正文占满全宽")]
fn t_hidden(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    let painted = tui.last_painted_lines();
    let top = plain(painted.first().expect("viewport row").as_str());
    let last = top.chars().last().unwrap_or_default();
    assert_ne!(last, '^', "no gutter glyph when inactive: {top:?}");
}

#[then("应用视口滚动位置归零")]
fn t_top_zero(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    assert_eq!(tui.application_owned_scroll_top(), 0, "jumped to head");
}

#[then("应用视口滚到底且跟随底部")]
fn t_bottom_follow(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    let transcript_rows = ROWS as usize - DOCK;
    let content = tui
        .application_owned_content_lines()
        .map(<[String]>::len)
        .unwrap_or(0);
    assert_eq!(
        tui.application_owned_scroll_top(),
        content.saturating_sub(transcript_rows),
        "at bottom"
    );
}

#[then("不产生 transcript 选区")]
fn t_no_selection(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    assert!(!tui.application_owned_has_selection(), "no selection");
}

use xylitol_tui::{gutter_scroll_for_pointer, gutter_thumb_len, gutter_thumb_offset};

#[when("向下滚动到中段后渲染")]
fn w_scroll_middle(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    let notch = tui.application_owned_wheel_notch();
    assert!(tui.application_owned_scroll_by(-(notch * 4)));
    tui.render_frame().expect("repaint");
}

#[then("thumb 位置随滚动比例移动且按钮行不被 thumb 占用")]
fn t_thumb_ratio(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    let painted = tui.last_painted_lines();
    let top = plain(painted.first().expect("row").as_str());
    let bottom = plain(painted[ROWS as usize - DOCK - 1].as_str());
    assert!(top.ends_with('^') && bottom.ends_with('v'));
    let content = tui
        .application_owned_content_lines()
        .map(<[String]>::len)
        .unwrap_or(0);
    let vh = (ROWS - DOCK as u16) as usize;
    let scroll = tui.application_owned_scroll_top();
    let start = 1 + gutter_thumb_offset(scroll, content, vh);
    let thumb = plain(painted[start].as_str());
    let ch = thumb.chars().last().unwrap();
    assert_eq!(ch, '\u{2503}', "thumb glyph at ratio pos, got {thumb:?}");
}

#[when("在滑轨内域中段按下")]
fn w_press_mid(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    tui.replace_hits(tui.scroll_gutter_hit_set());
    let lower = ROWS - DOCK as u16 - 2;
    dispatch_click(
        tui,
        MouseEventKind::Down(MouseButton::Left),
        COLS - 1,
        lower,
    );
}

#[then("滚动位置按比例即时改变且进入拖拽")]
fn t_pressed_jump(gutter_bdd: &GutterBdd) {
    let tui = gutter_bdd.tui.borrow();
    let tui = tui.as_ref().expect("seeded");
    let content = tui
        .application_owned_content_lines()
        .map(<[String]>::len)
        .unwrap_or(0);
    let vh = (ROWS - DOCK as u16) as usize;
    let lower_inner = (ROWS - DOCK as u16 - 3) as usize;
    let thumb_len = gutter_thumb_len(content, vh);
    let expect = gutter_scroll_for_pointer(lower_inner, thumb_len / 2, 18, 6);
    assert_eq!(
        tui.application_owned_scroll_top(),
        expect,
        "proportional jump on press"
    );
}

#[given("滑轨拖拽进行中")]
fn g_dragging(gutter_bdd: &GutterBdd) {
    let mut tui = seed(long_component());
    tui.replace_hits(tui.scroll_gutter_hit_set());
    let lower = ROWS - DOCK as u16 - 2;
    dispatch_click(
        &mut tui,
        MouseEventKind::Down(MouseButton::Left),
        COLS - 1,
        lower,
    );
    *seeded(gutter_bdd) = Some(tui);
}

#[when("按住移动到另一内域行")]
fn w_drag_move(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    dispatch_click(tui, MouseEventKind::Drag(MouseButton::Left), COLS - 1, 2);
}

#[then("滚动位置按抓取偏移持续跟随且抬起后不再跟随")]
fn t_drag_follow_and_release(gutter_bdd: &GutterBdd) {
    let mut tui = seeded(gutter_bdd);
    let tui = tui.as_mut().expect("seeded");
    let expect = gutter_scroll_for_pointer(1, 1, 18, 6);
    assert_eq!(tui.application_owned_scroll_top(), expect, "drag follow");
    let before = tui.application_owned_scroll_top();
    dispatch_click(tui, MouseEventKind::Up(MouseButton::Left), COLS - 1, 2);
    let top_before = tui.application_owned_scroll_top();
    dispatch_click(tui, MouseEventKind::Drag(MouseButton::Left), COLS - 1, 5);
    let _ = before;
    assert_eq!(
        tui.application_owned_scroll_top(),
        top_before,
        "no follow after up"
    );
}
