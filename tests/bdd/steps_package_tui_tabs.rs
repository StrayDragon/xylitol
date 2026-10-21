use crate::bdd::prelude::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use std::rc::Rc;
use xylitol_tui::keybindings::{
    KeybindingsConfig, KeybindingsManager, KeybindingsScope, create_default_definitions,
};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{Component, InputEvent, Tabs};

const LONG: [&str; 5] = ["AlphaAAA", "BravoBBB", "CharlieC", "DeltaDDD", "EchoEEEE"];
const TIGHT: usize = 20;
const WIDE: usize = 40;

pub struct TabsBdd {
    pub tabs: RefCell<Tabs>,
    pub last_offer: Cell<Option<bool>>,
    pub last_vis: RefCell<String>,
    pub kb_scope: RefCell<Option<KeybindingsScope>>,
    pub selected_path: RefCell<Vec<usize>>,
}

#[fixture]
pub fn tabs_bdd() -> TabsBdd {
    TabsBdd {
        tabs: RefCell::new(Tabs::new(Vec::new())),
        last_offer: Cell::new(None),
        last_vis: RefCell::new(String::new()),
        kb_scope: RefCell::new(None),
        selected_path: RefCell::new(Vec::new()),
    }
}

fn bind_arrows() -> KeybindingsScope {
    let mut user = KeybindingsConfig::new();
    user.insert("tui.tabs.prev".into(), vec!["left".into()]);
    user.insert("tui.tabs.next".into(), vec!["right".into()]);
    let kb = Rc::new(RefCell::new(KeybindingsManager::new(
        create_default_definitions(),
        user,
    )));
    KeybindingsScope::enter(kb)
}

fn unbind() -> KeybindingsScope {
    let kb = Rc::new(RefCell::new(KeybindingsManager::new(
        create_default_definitions(),
        HashMap::new(),
    )));
    KeybindingsScope::enter(kb)
}

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn left_down(column: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn wheel(up: bool) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: if up {
            MouseEventKind::ScrollUp
        } else {
            MouseEventKind::ScrollDown
        },
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    })
}

fn paint(bdd: &TabsBdd, width: usize) {
    let mut tabs = bdd.tabs.borrow_mut();
    let lines = tabs.render(width);
    *bdd.last_vis.borrow_mut() = lines
        .iter()
        .map(|l| strip_ansi_codes(l))
        .collect::<Vec<_>>()
        .join("\n");
}

fn three(bdd: &TabsBdd, selected: usize) {
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    tabs.set_selected(selected);
    *bdd.tabs.borrow_mut() = tabs;
}

fn longs(bdd: &TabsBdd, selected: usize) {
    let mut tabs = Tabs::new(LONG.iter().map(|s| (*s).to_string()).collect());
    tabs.set_selected(selected);
    *bdd.tabs.borrow_mut() = tabs;
}

#[given("有三个标题且选中第二个并且上一标题键与下一标题键已绑定")]
fn g_three_bound(tabs_bdd: &TabsBdd) {
    three(tabs_bdd, 1);
    *tabs_bdd.kb_scope.borrow_mut() = Some(bind_arrows());
}

#[given("有三个标题且选中第二个并且上一标题键与下一标题键未绑定")]
fn g_three_unbound(tabs_bdd: &TabsBdd) {
    three(tabs_bdd, 1);
    *tabs_bdd.kb_scope.borrow_mut() = Some(unbind());
}

#[given("有两个标题")]
fn g_two(tabs_bdd: &TabsBdd) {
    *tabs_bdd.tabs.borrow_mut() = Tabs::new(vec!["one".into(), "two".into()]);
}

#[given("有多个长标题且选中靠后的一个")]
fn g_long_late(tabs_bdd: &TabsBdd) {
    longs(tabs_bdd, 4);
}

#[given("有多个长标题且选中其中一个")]
fn g_long_mid_or(tabs_bdd: &TabsBdd) {
    longs(tabs_bdd, 2);
}

#[given("有多个长标题且选中靠中间的一个")]
fn g_long_mid(tabs_bdd: &TabsBdd) {
    longs(tabs_bdd, 2);
}

#[given("有三个标题且选中第一个并已按固定宽度渲染")]
fn g_three_first_wide(tabs_bdd: &TabsBdd) {
    three(tabs_bdd, 0);
    paint(tabs_bdd, WIDE);
}

#[given("有多个长标题且选中靠中间的一个并已按不足宽度渲染")]
fn g_long_mid_tight(tabs_bdd: &TabsBdd) {
    longs(tabs_bdd, 2);
    paint(tabs_bdd, TIGHT);
}

#[given("有三个标题且选中第二个并已渲染")]
fn g_three_mid_rendered(tabs_bdd: &TabsBdd) {
    three(tabs_bdd, 1);
    paint(tabs_bdd, WIDE);
}

#[given("有三个标题且原点在第 0 行并已渲染")]
fn g_three_origin0(tabs_bdd: &TabsBdd) {
    three(tabs_bdd, 1);
    tabs_bdd.tabs.borrow_mut().set_screen_origin(0, 0);
    paint(tabs_bdd, WIDE);
}

#[given("有三个标题且选中第一个、原点在第 0 行并已渲染")]
fn g_three_first_origin0(tabs_bdd: &TabsBdd) {
    three(tabs_bdd, 0);
    tabs_bdd.tabs.borrow_mut().set_screen_origin(0, 0);
    paint(tabs_bdd, WIDE);
}

#[when("依次收到下一标题键与上一标题键")]
fn w_next_then_prev(tabs_bdd: &TabsBdd) {
    let mut tabs = tabs_bdd.tabs.borrow_mut();
    let mut path = tabs_bdd.selected_path.borrow_mut();
    tabs.handle_input(key(KeyCode::Right));
    path.push(tabs.selected());
    tabs.handle_input(key(KeyCode::Left));
    path.push(tabs.selected());
}

#[when("收到左方向键")]
fn w_left(tabs_bdd: &TabsBdd) {
    tabs_bdd.tabs.borrow_mut().handle_input(key(KeyCode::Left));
}

#[when("按固定宽度渲染")]
fn w_wide(tabs_bdd: &TabsBdd) {
    paint(tabs_bdd, WIDE);
}

#[when("按不足以放下全部标题的宽度渲染")]
fn w_tight(tabs_bdd: &TabsBdd) {
    paint(tabs_bdd, TIGHT);
}

#[when("在第二个标题的列上左键按下")]
fn w_click_second(tabs_bdd: &TabsBdd) {
    let col = {
        let tabs = tabs_bdd.tabs.borrow();
        let (_, start, end) = tabs.title_col_ranges()[1];
        ((start + end) / 2) as u16
    };
    let ev = left_down(col, 0);
    let claimed = tabs_bdd.tabs.borrow_mut().offer_pointer(&ev);
    tabs_bdd.last_offer.set(Some(claimed));
}

#[when("在右侧溢出标记的列上左键按下")]
fn w_click_right_marker(tabs_bdd: &TabsBdd) {
    let col = {
        let tabs = tabs_bdd.tabs.borrow();
        let right = tabs
            .overflow_col_ranges()
            .into_iter()
            .find(|r| r.0 > tabs.selected())
            .expect("right overflow");
        ((right.1 + right.2) / 2) as u16
    };
    tabs_bdd.tabs.borrow_mut().handle_input(left_down(col, 0));
}

#[when("收到滚轮向上")]
fn w_wheel_up(tabs_bdd: &TabsBdd) {
    tabs_bdd.tabs.borrow_mut().handle_input(wheel(true));
}

#[when("收到滚轮向下")]
fn w_wheel_down(tabs_bdd: &TabsBdd) {
    tabs_bdd.tabs.borrow_mut().handle_input(wheel(false));
}

#[when("在第 2 行左键按下")]
fn w_click_row2(tabs_bdd: &TabsBdd) {
    let claimed = tabs_bdd.tabs.borrow_mut().offer_pointer(&left_down(0, 2));
    tabs_bdd.last_offer.set(Some(claimed));
}

#[when("在第 0 行且落在第二个标题列上左键按下")]
fn w_click_row0_second(tabs_bdd: &TabsBdd) {
    w_click_second(tabs_bdd);
}

#[then("选中下标先到第三个再回到第二个")]
fn t_cycle_then_back(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.selected_path.borrow().as_slice(), &[2, 1]);
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 1);
}

#[then("选中下标仍是第二个")]
fn t_still_second(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 1);
}

#[then("输出只有标题行且不含第二行正文")]
fn t_strip_only(tabs_bdd: &TabsBdd) {
    let vis = tabs_bdd.last_vis.borrow();
    assert_eq!(vis.lines().count(), 1, "{vis:?}");
    assert!(vis.contains("one") && vis.contains("two"), "{vis:?}");
}

#[then("每个可见标题有一段带原下标且落在宽度内、互不重叠的列区间")]
fn t_title_ranges(tabs_bdd: &TabsBdd) {
    let tabs = tabs_bdd.tabs.borrow();
    let ranges = tabs.title_col_ranges();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].0, 0);
    assert_eq!(ranges[1].0, 1);
    assert!(ranges[0].2 <= ranges[1].1);
    assert!(ranges[1].2 <= WIDE);
}

#[then("区间表每段的下标是原标题下标且不含隐藏标题")]
fn t_overflow_source_index(tabs_bdd: &TabsBdd) {
    let tabs = tabs_bdd.tabs.borrow();
    let ranges = tabs.title_col_ranges();
    assert!(ranges.iter().any(|r| r.0 == 4));
    assert!(!ranges.iter().any(|r| r.0 == 0));
    for &(idx, start, end) in &ranges {
        assert!(idx < LONG.len());
        assert!(start < end && end <= TIGHT);
    }
}

#[then("输出含当前标题全文、不含全部隐藏标题、且两侧有隐藏计数")]
fn t_overflow_keeps_selected(tabs_bdd: &TabsBdd) {
    let vis = tabs_bdd.last_vis.borrow();
    assert!(vis.contains("CharlieC"), "{vis:?}");
    assert!(!vis.contains("AlphaAAA"), "{vis:?}");
    assert!(!vis.contains("EchoEEEE"), "{vis:?}");
    assert!(
        vis.contains('+') && (vis.contains('←') || vis.contains('→')),
        "{vis:?}"
    );
}

#[then("有隐藏的每一侧各有一段标记区间且下标是该侧第一个隐藏标题")]
fn t_overflow_markers(tabs_bdd: &TabsBdd) {
    let tabs = tabs_bdd.tabs.borrow();
    let titles: Vec<usize> = tabs.title_col_ranges().iter().map(|r| r.0).collect();
    let markers = tabs.overflow_col_ranges();
    assert!(!markers.is_empty());
    for &(idx, start, end, count) in &markers {
        assert!(start < end && count > 0);
        assert!(!titles.contains(&idx));
    }
    if let Some(min) = titles.iter().copied().min()
        && min > 0
    {
        assert!(markers.iter().any(|m| m.0 == min - 1));
    }
    if let Some(max) = titles.iter().copied().max()
        && max + 1 < LONG.len()
    {
        assert!(markers.iter().any(|m| m.0 == max + 1));
    }
}

#[then("选中下标变为第二个")]
fn t_selected_1(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 1);
}

#[then("选中下标变为右侧第一个隐藏标题")]
fn t_selected_right_hidden(tabs_bdd: &TabsBdd) {
    let tabs = tabs_bdd.tabs.borrow();
    let max_vis = tabs
        .title_col_ranges()
        .iter()
        .map(|r| r.0)
        .max()
        .unwrap_or(0);
    assert_eq!(tabs.selected(), max_vis + 1);
}

#[then("选中下标变为第三个")]
fn t_selected_2(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 2);
}

#[then("选中下标变为第一个")]
fn t_selected_0(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 0);
}

#[then("选中下标不变且 offer_pointer 为假")]
fn t_unchanged_false(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 1);
    assert_eq!(tabs_bdd.last_offer.get(), Some(false));
}

#[then("选中下标变为第二个且 offer_pointer 为真")]
fn t_second_true(tabs_bdd: &TabsBdd) {
    assert_eq!(tabs_bdd.tabs.borrow().selected(), 1);
    assert_eq!(tabs_bdd.last_offer.get(), Some(true));
}
