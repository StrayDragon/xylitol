use std::cell::RefCell;
use std::rc::Rc;

use crate::components::input::Input;
use crate::fuzzy::fuzzy_match;
use crate::keybindings::with_keybindings;
use crate::tui::Component;
use crate::utils::{truncate_to_width, visible_width, wrap_text_with_ansi};

// ── types ───────────────────────────────────────────────────────────────────

pub type SubmenuFactory = Box<dyn Fn(String, Box<dyn Fn(Option<String>)>) -> Box<dyn Component>>;

pub struct SettingItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub current_value: String,
    /// If set, Enter/Space cycles through these values in order.
    pub values: Option<Vec<String>>,
    /// If set, Enter opens a submenu component via this factory.
    pub submenu: Option<SubmenuFactory>,
}

#[allow(clippy::type_complexity)]
pub struct SettingsListTheme {
    pub label: Box<dyn Fn(&str, bool) -> String>,
    pub value: Box<dyn Fn(&str, bool) -> String>,
    pub description: Box<dyn Fn(&str) -> String>,
    pub cursor: String,
    pub hint: Box<dyn Fn(&str) -> String>,
}

#[derive(Default)]
pub struct SettingsListOptions {
    pub enable_search: bool,
}

// ── SettingsList ─────────────────────────────────────────────────────────────

#[allow(clippy::type_complexity)]
/// A scrollable, keyboard-navigable settings list with optional fuzzy search.
///
/// Ported from pi's `components/settings-list.ts`.
pub struct SettingsList {
    items: Vec<SettingItem>,
    /// Indices into `self.items` of currently-filtered items.
    filtered_indices: Vec<usize>,
    theme: SettingsListTheme,
    selected_index: usize,
    max_visible: usize,
    on_change: Box<dyn Fn(&str, &str)>,
    on_cancel: Box<dyn Fn()>,

    search_input: Option<Input>,
    search_enabled: bool,

    // Submenu state
    submenu: Option<Box<dyn Component>>,
    /// Shared signal — the submenu's `done` callback writes here.
    submenu_close: Option<Rc<RefCell<Option<String>>>>,
    submenu_item_index: Option<usize>,
}

impl SettingsList {
    pub fn new(
        items: Vec<SettingItem>,
        max_visible: usize,
        theme: SettingsListTheme,
        on_change: impl Fn(&str, &str) + 'static,
        on_cancel: impl Fn() + 'static,
        options: SettingsListOptions,
    ) -> Self {
        let search_enabled = options.enable_search;
        let filtered_indices = (0..items.len()).collect();
        Self {
            items,
            filtered_indices,
            theme,
            selected_index: 0,
            max_visible,
            on_change: Box::new(on_change),
            on_cancel: Box::new(on_cancel),
            search_input: if search_enabled {
                Some(Input::new())
            } else {
                None
            },
            search_enabled,
            submenu: None,
            submenu_close: None,
            submenu_item_index: None,
        }
    }

    pub fn update_value(&mut self, id: &str, new_value: String) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.current_value = new_value;
        }
    }
}

impl Component for SettingsList {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.poll_submenu_close();

        if let Some(ref mut sub) = self.submenu {
            return sub.render(width);
        }

        // Pre-render the search bar so render_main_list doesn't need &mut.
        let mut search_lines: Vec<String> = Vec::new();
        if self.search_enabled
            && let Some(ref mut input) = self.search_input
        {
            search_lines = input.render(width);
            if !search_lines.is_empty() {
                search_lines.push(String::new());
            }
        }

        self.render_main_list(width, &search_lines)
    }

    fn handle_input(&mut self, event: crate::tui::InputEvent) {
        use crate::keys::printable_from_key_event;
        use crate::tui::InputEvent;
        use crossterm::event::KeyCode;

        if let Some(ref mut sub) = self.submenu {
            sub.handle_input(event);
            self.poll_submenu_close();
            return;
        }

        let item_count = if self.search_enabled {
            self.filtered_indices.len()
        } else {
            self.items.len()
        };

        let InputEvent::Key(ref key) = event else {
            return;
        };

        let up = with_keybindings(|kb| kb.matches_event(key, "tui.select.up"));
        let down = with_keybindings(|kb| kb.matches_event(key, "tui.select.down"));
        let confirm = with_keybindings(|kb| kb.matches_event(key, "tui.select.confirm"))
            || matches!(key.code, KeyCode::Char(' '));
        let cancel = with_keybindings(|kb| kb.matches_event(key, "tui.select.cancel"));

        if up {
            if item_count == 0 {
                return;
            }
            if self.selected_index == 0 {
                self.selected_index = item_count.saturating_sub(1);
            } else {
                self.selected_index -= 1;
            }
        } else if down {
            if item_count == 0 {
                return;
            }
            if self.selected_index >= item_count.saturating_sub(1) {
                self.selected_index = 0;
            } else {
                self.selected_index += 1;
            }
        } else if confirm {
            self.activate_item();
        } else if cancel {
            (self.on_cancel)();
        } else if self.search_enabled {
            // Space is confirm above; other printable chars feed the search input.
            let Some(ch) = printable_from_key_event(key) else {
                return;
            };
            if ch == " " {
                return;
            }
            if let Some(ref mut input) = self.search_input {
                input.handle_input(InputEvent::Key(*key));
                let query = input.value().to_string();
                self.apply_filter(&query);
            }
        }
    }

    fn invalidate(&mut self) {
        self.poll_submenu_close();
        if let Some(ref mut sub) = self.submenu {
            sub.invalidate();
        }
    }
}

// ── private ─────────────────────────────────────────────────────────────────

impl SettingsList {
    fn render_main_list(&self, width: usize, search_lines: &[String]) -> Vec<String> {
        let mut lines: Vec<String> = search_lines
            .iter()
            .map(|l| {
                if visible_width(l) <= width {
                    l.clone()
                } else {
                    truncate_to_width(l, width, "", false)
                }
            })
            .collect();
        let display_indices = &self.filtered_indices;

        if self.items.is_empty() {
            lines.push(truncate_to_width(
                &(self.theme.hint)("  No settings available"),
                width,
                "",
                false,
            ));
            if self.search_enabled {
                self.add_hint_line(&mut lines, width);
            }
            return lines;
        }

        if display_indices.is_empty() {
            lines.push(truncate_to_width(
                &(self.theme.hint)("  No matching settings"),
                width,
                "",
                false,
            ));
            self.add_hint_line(&mut lines, width);
            return lines;
        }

        // Visible range with scrolling.
        let half = self.max_visible / 2;
        let start = self
            .selected_index
            .saturating_sub(half)
            .min(display_indices.len().saturating_sub(self.max_visible));
        let start = start.min(display_indices.len().saturating_sub(1));
        let end = (start + self.max_visible).min(display_indices.len());

        let max_label_width = self
            .items
            .iter()
            .map(|it| visible_width(&it.label))
            .max()
            .unwrap_or(0)
            .min(30);

        let selected_real = display_indices
            .get(self.selected_index)
            .copied()
            .unwrap_or(usize::MAX);

        for &idx in &display_indices[start..end] {
            let item = &self.items[idx];
            let is_selected = idx == selected_real;

            let prefix = if is_selected {
                self.theme.cursor.clone()
            } else {
                "  ".to_string()
            };
            let prefix_width = visible_width(&prefix);

            let lw = visible_width(&item.label);
            let pad = max_label_width.saturating_sub(lw);
            let label_padded = format!("{}{}", item.label, " ".repeat(pad));
            let label_text = (self.theme.label)(&label_padded, is_selected);

            let separator = "  ";
            let used = prefix_width + max_label_width + visible_width(separator);
            let value_max = width.saturating_sub(used + 2);
            let value_text = (self.theme.value)(
                &truncate_to_width(&item.current_value, value_max, "", false),
                is_selected,
            );

            lines.push(truncate_to_width(
                &format!("{}{}{}{}", prefix, label_text, separator, value_text),
                width,
                "",
                false,
            ));
        }

        // Scroll indicator.
        if start > 0 || end < display_indices.len() {
            let txt = format!("  ({}/{})", self.selected_index + 1, display_indices.len());
            lines.push(truncate_to_width(
                &(self.theme.hint)(&txt),
                width,
                "",
                false,
            ));
        }

        // Description.
        if let Some(item) = display_indices
            .get(self.selected_index)
            .and_then(|&i| self.items.get(i))
            && let Some(ref desc) = item.description
        {
            lines.push(String::new());
            let body_w = width.saturating_sub(2).max(1);
            for line in wrap_text_with_ansi(desc, body_w) {
                lines.push(truncate_to_width(
                    &(self.theme.description)(&format!("  {line}")),
                    width,
                    "",
                    false,
                ));
            }
        }

        self.add_hint_line(&mut lines, width);
        lines
    }

    fn add_hint_line(&self, lines: &mut Vec<String>, width: usize) {
        lines.push(String::new());
        let hint = if self.search_enabled {
            "  Type to search · Enter/Space to change · Esc to cancel"
        } else {
            "  Enter/Space to change · Esc to cancel"
        };
        lines.push(truncate_to_width(
            &(self.theme.hint)(hint),
            width,
            "",
            false,
        ));
    }

    // ── submenu ──────────────────────────────────────────────────────────

    fn poll_submenu_close(&mut self) {
        let value = if let Some(ref signal) = self.submenu_close {
            signal.borrow_mut().take()
        } else {
            None
        };
        if let Some(v) = value {
            self.close_submenu(Some(v));
        }
    }

    fn close_submenu(&mut self, selected_value: Option<String>) {
        if let Some(ref val) = selected_value {
            let idx = self
                .submenu_item_index
                .and_then(|i| {
                    if self.search_enabled {
                        self.filtered_indices.get(i).copied()
                    } else {
                        Some(i)
                    }
                })
                .and_then(|i| self.items.get_mut(i));
            if let Some(item) = idx {
                item.current_value = val.clone();
                (self.on_change)(&item.id, val);
            }
        }

        self.submenu = None;
        self.submenu_close = None;
        if let Some(i) = self.submenu_item_index.take() {
            self.selected_index = i;
        }
    }

    // ── filter / activate ────────────────────────────────────────────────

    fn apply_filter(&mut self, query: &str) {
        self.filtered_indices.clear();
        for (i, item) in self.items.iter().enumerate() {
            if fuzzy_match(query, &item.label).is_some() {
                self.filtered_indices.push(i);
            }
        }
        // Re-sort by match quality (lower score = better).
        self.filtered_indices.sort_by(|&a, &b| {
            let sa = fuzzy_match(query, &self.items[a].label)
                .map(|m| m.score)
                .unwrap_or(0.0);
            let sb = fuzzy_match(query, &self.items[b].label)
                .map(|m| m.score)
                .unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
        self.selected_index = 0;
    }

    fn activate_item(&mut self) {
        let real_idx = if self.search_enabled {
            self.filtered_indices.get(self.selected_index).copied()
        } else {
            Some(self.selected_index)
        };

        let item_idx = match real_idx {
            Some(i) => i,
            None => return,
        };

        // Check if submenu or value cycling is available.
        let (has_submenu, has_values) = {
            let item = &self.items[item_idx];
            (
                item.submenu.is_some(),
                item.values.as_ref().is_some_and(|v| !v.is_empty()),
            )
        };

        if has_submenu {
            self.open_submenu(item_idx);
        } else if has_values {
            let values_len = self.items[item_idx].values.as_ref().unwrap().len();
            let current = &self.items[item_idx].current_value;
            let values = self.items[item_idx].values.as_ref().unwrap();
            let pos = values.iter().position(|v| *v == *current).unwrap_or(0);
            let next = (pos + 1) % values_len;
            let new_value = values[next].clone();
            self.items[item_idx].current_value = new_value.clone();
            let id = self.items[item_idx].id.clone();
            (self.on_change)(&id, &new_value);
        }
    }

    fn open_submenu(&mut self, item_idx: usize) {
        let current_val = self.items[item_idx].current_value.clone();
        self.submenu_item_index = Some(self.selected_index);

        let signal: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let signal_clone = signal.clone();

        let done: Box<dyn Fn(Option<String>)> = Box::new(move |value: Option<String>| {
            *signal_clone.borrow_mut() = value;
        });

        // Call the submenu factory. Box<dyn Fn> implements Fn, so we can
        // call it via reference without moving out of the Option.
        let factory = self.items[item_idx].submenu.as_ref().unwrap();
        let component = factory(current_val, done);
        self.submenu = Some(component);
        self.submenu_close = Some(signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_theme() -> SettingsListTheme {
        SettingsListTheme {
            label: Box::new(|s: &str, _sel: bool| s.to_string()),
            value: Box::new(|s: &str, _sel: bool| s.to_string()),
            description: Box::new(|s: &str| s.to_string()),
            cursor: "> ".to_string(),
            hint: Box::new(|s: &str| s.to_string()),
        }
    }

    fn make_item(id: &str, label: &str, value: &str) -> SettingItem {
        SettingItem {
            id: id.to_string(),
            label: label.to_string(),
            description: None,
            current_value: value.to_string(),
            values: None,
            submenu: None,
        }
    }

    #[test]
    fn renders_items() {
        let mut list = SettingsList::new(
            vec![make_item("a", "Alpha", "on"), make_item("b", "Beta", "off")],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions::default(),
        );
        let lines = list.render(40);
        assert!(lines.iter().any(|l| l.contains("Alpha")));
        assert!(lines.iter().any(|l| l.contains("Beta")));
        assert!(lines.iter().any(|l| l.contains("on")));
        assert!(lines.iter().any(|l| l.contains("off")));
    }

    #[test]
    fn empty_items_shows_hint() {
        let mut list = SettingsList::new(
            vec![],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions::default(),
        );
        let lines = list.render(40);
        assert!(lines.iter().any(|l| l.contains("No settings")));
    }

    fn feed(list: &mut SettingsList, code: crossterm::event::KeyCode) {
        use crate::tui::InputEvent;
        use crossterm::event::{KeyEvent, KeyModifiers};
        list.handle_input(InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn feed_char(list: &mut SettingsList, c: char) {
        feed(list, crossterm::event::KeyCode::Char(c));
    }

    #[test]
    fn up_down_wrap() {
        use crossterm::event::KeyCode;
        let mut list = SettingsList::new(
            vec![make_item("a", "A", "1"), make_item("b", "B", "2")],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions::default(),
        );
        // Up from 0 wraps to last.
        feed(&mut list, KeyCode::Up);
        assert_eq!(list.selected_index, 1);
        // Down from last wraps to 0.
        feed(&mut list, KeyCode::Down);
        assert_eq!(list.selected_index, 0);
    }

    #[test]
    fn confirm_cycles_values() {
        use crossterm::event::KeyCode;
        let on_change = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
        let oc = on_change.clone();
        let mut list = SettingsList::new(
            vec![SettingItem {
                id: "x".into(),
                label: "X".into(),
                description: None,
                current_value: "a".into(),
                values: Some(vec!["a".into(), "b".into(), "c".into()]),
                submenu: None,
            }],
            10,
            default_theme(),
            move |id, val| {
                *oc.borrow_mut() = format!("{}=", val);
                let _ = id;
            },
            || {},
            SettingsListOptions::default(),
        );
        feed(&mut list, KeyCode::Enter); // Enter → cycles a→b
        assert_eq!(*on_change.borrow(), "b=");
        assert_eq!(list.items[0].current_value, "b");
    }

    #[test]
    fn cancel_calls_on_cancel() {
        use crossterm::event::KeyCode;
        let cancelled = std::rc::Rc::new(std::cell::RefCell::new(false));
        let cc = cancelled.clone();
        let mut list = SettingsList::new(
            vec![make_item("a", "A", "x")],
            10,
            default_theme(),
            |_, _| {},
            move || *cc.borrow_mut() = true,
            SettingsListOptions::default(),
        );
        feed(&mut list, KeyCode::Esc);
        assert!(*cancelled.borrow());
    }

    #[test]
    fn search_filters_items() {
        let mut list = SettingsList::new(
            vec![make_item("a", "Alpha", "x"), make_item("b", "Bravo", "y")],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions {
                enable_search: true,
            },
        );
        // Type "alp"
        feed_char(&mut list, 'a');
        feed_char(&mut list, 'l');
        feed_char(&mut list, 'p');
        let lines = list.render(40);
        assert!(
            lines.iter().any(|l| l.contains("Alpha")),
            "should show Alpha"
        );
        assert!(
            !lines.iter().any(|l| l.contains("Bravo")),
            "should not show Bravo"
        );
    }

    #[test]
    fn submenu_open_and_close() {
        use crossterm::event::KeyCode;
        let mut list = SettingsList::new(
            vec![SettingItem {
                id: "s".into(),
                label: "Sub".into(),
                description: None,
                current_value: "old".into(),
                values: None,
                submenu: Some(Box::new(|_current, _done| {
                    Box::new(crate::components::text::Text::new(
                        "submenu content".into(),
                        0,
                        0,
                    ))
                })),
            }],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions::default(),
        );
        feed(&mut list, KeyCode::Enter); // open submenu
        assert!(list.submenu.is_some());
        let lines = list.render(40);
        assert!(lines.iter().any(|l| l.contains("submenu content")));

        // Submenu done callback → close and update value.
        // We trigger by rendering (which polls submenu_close).
        // We need to write to the signal first, which the real submenu would do.
        if let Some(ref signal) = list.submenu_close {
            *signal.borrow_mut() = Some("new".to_string());
        }
        list.render(40); // poll_submenu_close fires, applies value, closes
        assert!(list.submenu.is_none());
        assert_eq!(list.items[0].current_value, "new");
    }

    #[test]
    fn narrow_width_clamps_empty_and_rows() {
        let mut empty = SettingsList::new(
            vec![],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions::default(),
        );
        for w in [0usize, 1, 8, 20] {
            for line in empty.render(w) {
                assert!(
                    visible_width(&line) <= w,
                    "empty list width={w}: visible {} > budget; line={line:?}",
                    visible_width(&line)
                );
            }
        }

        let mut list = SettingsList::new(
            vec![SettingItem {
                id: "a".into(),
                label: "x".repeat(40),
                description: Some("y".repeat(60)),
                current_value: "z".repeat(40),
                values: None,
                submenu: None,
            }],
            10,
            default_theme(),
            |_, _| {},
            || {},
            SettingsListOptions {
                enable_search: true,
                ..SettingsListOptions::default()
            },
        );
        for w in [1usize, 8, 16, 32] {
            for line in list.render(w) {
                assert!(
                    visible_width(&line) <= w,
                    "rows width={w}: visible {} > budget; line={line:?}",
                    visible_width(&line)
                );
            }
        }

        feed_char(&mut list, 'q');
        feed_char(&mut list, 'q');
        feed_char(&mut list, 'q');
        for w in [1usize, 10, 18] {
            for line in list.render(w) {
                assert!(
                    visible_width(&line) <= w,
                    "no-match width={w}: visible {} > budget; line={line:?}",
                    visible_width(&line)
                );
            }
        }
        let joined = list.render(40).join("\n");
        assert!(
            joined.contains("No matching"),
            "filter miss should show empty hint:\n{joined}"
        );
    }
}
