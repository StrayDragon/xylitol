mod support;

use insta::assert_snapshot;
use support::TuiTestHarness;
use xylitol_tui::components::panel::Panel;
use xylitol_tui::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};
use xylitol_tui::components::text::Text;

fn snap(name: &str, harness: &TuiTestHarness) {
    assert_snapshot!(name, support::viewport_snapshot(harness));
}

#[test]
fn panel_renders_bordered_layout() {
    let mut panel = Panel::new(1, 1, None);
    panel.add_child(Box::new(Text::new("hello".into(), 0, 0)));
    let mut h = TuiTestHarness::new(20, 5);
    h.mount(Box::new(panel));
    h.render();
    snap("panel_bordered", &h);
}

#[test]
fn select_list_renders_highlighted_first_item() {
    let items = vec![
        SelectItem::new("apple", "apple"),
        SelectItem::new("banana", "banana"),
        SelectItem::new("cherry", "cherry"),
    ];
    let list = SelectList::new(
        items,
        5,
        SelectListTheme::default(),
        SelectListLayoutOptions {
            min_primary_column_width: None,
            max_primary_column_width: None,
            truncate_primary: None,
        },
    );
    let mut h = TuiTestHarness::new(20, 5);
    h.mount(Box::new(list));
    h.render();
    snap("select_list", &h);
}

#[test]
fn text_wraps_at_terminal_width() {
    let mut h = TuiTestHarness::new(15, 6);
    h.mount(Box::new(Text::new(
        "the quick brown fox jumps".into(),
        0,
        0,
    )));
    h.render();
    snap("text_wrap", &h);
}
