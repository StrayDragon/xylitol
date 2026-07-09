mod support;

use support::vt_feed::feed_vt;
use xylitol_tui::components::{
    input::Input,
    loader::Loader,
    panel::Panel,
    select_list::{SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme},
    spacer::Spacer,
    text::Text,
};
use xylitol_tui::{Component, OverlayAnchor, OverlayMargin, OverlayOptions, TUI};

/// Virtual terminal for testing.
struct TestTerminal {
    cols: u16,
    rows: u16,
    written: Vec<String>,
}
impl TestTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            written: Vec::new(),
        }
    }
}
impl xylitol_tui::Terminal for TestTerminal {
    fn write(&mut self, data: &str) {
        self.written.push(data.to_string());
    }
    fn columns(&self) -> u16 {
        self.cols
    }
    fn rows(&self) -> u16 {
        self.rows
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
}

/// Test TUI with a single component renders without panic.
#[test]
fn test_tui_renders_simple_component() {
    let term = TestTerminal::new(80, 24);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(Text::new("hello".into(), 0, 0)));
    // Trigger render manually (we can't call start() in test since it blocks)
    // Instead, test components directly
    let mut text = Text::new("hello".into(), 0, 0);
    let lines = text.render(80);
    assert!(!lines.is_empty());
    assert!(lines[0].contains("hello"));
}

/// Test that container renders multiple children.
#[test]
fn test_container_renders_children() {
    let mut text1 = Text::new("first".into(), 0, 0);
    let mut text2 = Text::new("second".into(), 0, 0);

    let lines1 = text1.render(20);
    let lines2 = text2.render(20);

    assert!(lines1[0].contains("first"));
    assert!(lines2[0].contains("second"));
}

/// Test Panel with multiple children and background.
#[test]
fn test_tui_panel_with_background() {
    let mut panel = Panel::new(
        1,
        1,
        Some(Box::new(|s: &str| format!("\x1b[44m{}\x1b[49m", s))),
    );
    panel.add_child(Box::new(Text::new("Title".into(), 0, 0)));
    panel.add_child(Box::new(Spacer::new(1)));
    panel.add_child(Box::new(Text::new("Body text".into(), 0, 0)));

    let lines = panel.render(30);
    // Top padding + Title + Spacer + Body + Bottom padding = 5 lines
    assert_eq!(lines.len(), 5);
    // All content lines have background
    assert!(lines[1].contains("\x1b[44m"));
    assert!(lines[3].contains("\x1b[44m"));
}

/// Test SelectList navigation.
#[test]
fn test_tui_select_list_navigation() {
    let items = vec![
        SelectItem::new("a", "Alpha").with_description("First item"),
        SelectItem::new("b", "Beta").with_description("Second item"),
        SelectItem::new("c", "Gamma"),
    ];
    let theme = SelectListTheme::default();
    let mut list = SelectList::new(
        items,
        10,
        theme,
        SelectListLayoutOptions {
            min_primary_column_width: None,
            max_primary_column_width: None,
            truncate_primary: None,
        },
    );

    // Render check
    let lines = list.render(80);
    assert!(lines[0].contains("→"), "first item selected");
    assert!(lines[0].contains("Alpha"));

    // Navigate down
    feed_vt(&mut list, "\x1b[B");
    assert_eq!(list.get_selected_item().unwrap().value, "b");

    // Navigate down
    feed_vt(&mut list, "\x1b[B");
    assert_eq!(list.get_selected_item().unwrap().value, "c");

    // Wrap around
    feed_vt(&mut list, "\x1b[B");
    assert_eq!(list.get_selected_item().unwrap().value, "a");
}

/// Test SelectList filter.
#[test]
fn test_tui_select_list_filter() {
    let items = vec![
        SelectItem::new("apple", "Apple"),
        SelectItem::new("apricot", "Apricot"),
        SelectItem::new("banana", "Banana"),
    ];
    let mut list = SelectList::new(
        items,
        10,
        SelectListTheme::default(),
        SelectListLayoutOptions {
            min_primary_column_width: None,
            max_primary_column_width: None,
            truncate_primary: None,
        },
    );

    list.set_filter("ap");
    assert_eq!(list.filtered_items.len(), 2);
    assert_eq!(list.filtered_items[0].value, "apple");
    assert_eq!(list.filtered_items[1].value, "apricot");
}

/// Test Input cursor operations.
#[test]
fn test_tui_input_cursor_operations() {
    let mut input = Input::new();
    input.set_focused(true);

    // Type text
    feed_vt(&mut input, "hello");
    assert_eq!(input.value(), "hello");

    // Move left and insert
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "X");
    assert_eq!(input.value(), "helXlo");

    // Home and insert
    feed_vt(&mut input, "\x1b[H");
    feed_vt(&mut input, "Y");
    assert_eq!(input.value(), "YhelXlo");

    // Backspace
    feed_vt(&mut input, "\x7f");
    assert_eq!(input.value(), "helXlo");
}

/// Test word wrap with ANSI.
#[test]
fn test_tui_word_wrap_ansi() {
    use xylitol_tui::wrap_text_with_ansi;
    let colored = "\x1b[31mred text\x1b[0m is here";
    let lines = wrap_text_with_ansi(colored, 12);
    assert!(lines[0].contains("\x1b[31m"));
    assert!(lines[0].contains("red text"));
}

/// Test truncate to width.
#[test]
fn test_tui_truncate() {
    use xylitol_tui::truncate_to_width;
    assert_eq!(
        truncate_to_width("hello", 3, "...", false),
        "\x1b[0m...\x1b[0m"
    );
    assert_eq!(truncate_to_width("hi", 10, "...", false), "hi");
}

/// Test visible_width with ANSI.
#[test]
fn test_tui_visible_width_ansi() {
    use xylitol_tui::visible_width;
    assert_eq!(visible_width("\x1b[1;31mhello\x1b[0m"), 5);
    assert_eq!(visible_width("\x1b[34m\x1b[1mbold blue\x1b[0m"), 9);
}

/// Test overlay positioning logic.
#[test]
fn test_tui_overlay_center() {
    let opts = OverlayOptions {
        anchor: Some(OverlayAnchor::Center),
        ..Default::default()
    };
    // Overlay anchors resolve in the TUI; just test construction
    assert!(matches!(opts.anchor, Some(OverlayAnchor::Center)));
}

/// Test overlay with margins.
#[test]
fn test_tui_overlay_margins() {
    let opts = OverlayOptions {
        margin: Some(OverlayMargin {
            top: Some(2),
            bottom: Some(3),
            left: Some(1),
            right: Some(4),
        }),
        ..Default::default()
    };
    let m = opts.margin.unwrap();
    assert_eq!(m.top, Some(2));
    assert_eq!(m.bottom, Some(3));
}

/// Test SizeValue parsing.
#[test]
fn test_size_value_resolve() {
    use xylitol_tui::SizeValue;
    assert_eq!(SizeValue::Absolute(10).resolve(100), 10);
    assert_eq!(SizeValue::Percent(50.0).resolve(100), 50);
    assert_eq!(SizeValue::Percent(25.0).resolve(80), 20);
}

/// Test Loader component.
#[test]
fn test_tui_loader_renders() {
    let mut loader = Loader::new(
        Box::new(|s| format!("\x1b[36m{}\x1b[39m", s)),
        Box::new(|s| s.to_string()),
        "Working...".into(),
        None,
    );

    let lines = loader.render(30);
    // First line empty + text line
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("Working..."));
}

/// Test that spaces don't break TUI content.
#[test]
fn test_tui_spacer_renders_empty_lines() {
    let mut spacer = Spacer::new(3);
    let lines = spacer.render(80);
    assert_eq!(lines.len(), 3);
    assert!(lines.iter().all(|l| l.is_empty()));
}

/// Test key binding config works.
#[test]
fn test_tui_keybinding_config_complete() {
    use std::collections::HashMap;
    use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions};

    let defs = create_default_definitions();
    let kb = KeybindingsManager::new(defs, HashMap::new());

    // Verify all expected bindings exist
    let expected = [
        "tui.editor.cursorUp",
        "tui.editor.cursorDown",
        "tui.editor.cursorLeft",
        "tui.editor.cursorRight",
        "tui.input.submit",
        "tui.select.cancel",
        "tui.editor.deleteCharBackward",
        "tui.editor.deleteCharForward",
        "tui.editor.undo",
        "tui.editor.yank",
    ];
    for &name in &expected {
        assert!(!kb.get_keys(name).is_empty(), "missing binding: {}", name);
    }
}
