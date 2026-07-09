use crate::keybindings::with_keybindings;
use crate::tui::Component;
use crate::utils::{truncate_to_width, visible_width};

const DEFAULT_PRIMARY_COLUMN_WIDTH: usize = 32;
const PRIMARY_COLUMN_GAP: usize = 2;
const MIN_DESCRIPTION_WIDTH: usize = 10;

fn normalize_to_single_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ").trim().to_string()
}

fn clamp(value: usize, min: usize, max: usize) -> usize {
    value.max(min).min(max)
}

#[derive(Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        let value = value.into();
        let label = label.into();
        Self {
            value,
            label,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Theme for SelectList styling.
// Clone not derived — Box<dyn Fn> is not Clone
pub struct SelectListTheme {
    pub selected_prefix: Box<dyn Fn(&str) -> String>,
    pub selected_text: Box<dyn Fn(&str) -> String>,
    pub description: Box<dyn Fn(&str) -> String>,
    pub scroll_info: Box<dyn Fn(&str) -> String>,
    pub no_match: Box<dyn Fn(&str) -> String>,
}

impl Default for SelectListTheme {
    fn default() -> Self {
        Self {
            selected_prefix: Box::new(|s| s.to_string()),
            selected_text: Box::new(|s| format!("\x1b[7m{}\x1b[27m", s)),
            description: Box::new(|s| format!("\x1b[2m{}\x1b[22m", s)),
            scroll_info: Box::new(|s| format!("\x1b[2m{}\x1b[22m", s)),
            no_match: Box::new(|s| s.to_string()),
        }
    }
}

pub struct SelectListLayoutOptions {
    pub min_primary_column_width: Option<usize>,
    pub max_primary_column_width: Option<usize>,
    pub truncate_primary: Option<Box<dyn Fn(SelectListTruncatePrimaryContext) -> String>>,
}

#[derive(Clone)]
pub struct SelectListTruncatePrimaryContext {
    pub text: String,
    pub max_width: usize,
    pub column_width: usize,
    pub item: SelectItem,
    pub is_selected: bool,
}

pub struct SelectList {
    pub items: Vec<SelectItem>,
    pub filtered_items: Vec<SelectItem>,
    pub selected_index: usize,
    pub max_visible: usize,
    theme: SelectListTheme,
    layout: SelectListLayoutOptions,
    pub on_select: Option<Box<dyn FnMut(SelectItem) + Send>>,
    pub on_cancel: Option<Box<dyn Fn() + Send>>,
    pub on_selection_change: Option<Box<dyn FnMut(SelectItem) + Send>>,
}

impl SelectList {
    pub fn new(
        items: Vec<SelectItem>,
        max_visible: usize,
        theme: SelectListTheme,
        layout: SelectListLayoutOptions,
    ) -> Self {
        Self {
            filtered_items: items.clone(),
            items,
            selected_index: 0,
            max_visible,
            theme,
            layout,
            on_select: None,
            on_cancel: None,
            on_selection_change: None,
        }
    }

    pub fn set_filter(&mut self, filter: &str) {
        let filter_lower = filter.to_lowercase();
        self.filtered_items = self
            .items
            .iter()
            .filter(|item| item.value.to_lowercase().starts_with(&filter_lower))
            .cloned()
            .collect();
        self.selected_index = 0;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        let max = self.filtered_items.len().saturating_sub(1);
        self.selected_index = index.min(max);
    }

    pub fn get_selected_item(&self) -> Option<&SelectItem> {
        self.filtered_items.get(self.selected_index)
    }

    fn get_primary_column_width(&self) -> usize {
        let (min, max) = self.get_primary_column_bounds();
        let widest = self
            .filtered_items
            .iter()
            .map(|item| visible_width(&self.get_display_value(item)) + PRIMARY_COLUMN_GAP)
            .max()
            .unwrap_or(0);
        clamp(widest, min, max)
    }

    fn get_primary_column_bounds(&self) -> (usize, usize) {
        let raw_min = self
            .layout
            .min_primary_column_width
            .or(self.layout.max_primary_column_width)
            .unwrap_or(DEFAULT_PRIMARY_COLUMN_WIDTH);
        let raw_max = self
            .layout
            .max_primary_column_width
            .or(self.layout.min_primary_column_width)
            .unwrap_or(DEFAULT_PRIMARY_COLUMN_WIDTH);
        let m = raw_min.min(raw_max).max(1);
        let mx = raw_min.max(raw_max).max(1);
        (m, mx)
    }

    fn truncate_primary(
        &self,
        item: &SelectItem,
        is_selected: bool,
        max_width: usize,
        column_width: usize,
    ) -> String {
        let display = self.get_display_value(item);
        if let Some(ref f) = self.layout.truncate_primary {
            let ctx = SelectListTruncatePrimaryContext {
                text: display,
                max_width,
                column_width,
                item: item.clone(),
                is_selected,
            };
            let result = f(ctx);
            truncate_to_width(&result, max_width, "", false)
        } else {
            truncate_to_width(&display, max_width, "", false)
        }
    }

    fn get_display_value(&self, item: &SelectItem) -> String {
        if item.label.is_empty() {
            item.value.clone()
        } else {
            item.label.clone()
        }
    }

    fn notify_selection_change(&mut self) {
        if let Some(item) = self.filtered_items.get(self.selected_index)
            && let Some(ref mut cb) = self.on_selection_change
        {
            cb(item.clone());
        }
    }
}

impl Component for SelectList {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        if self.filtered_items.is_empty() {
            lines.push((self.theme.no_match)("  No matching items"));
            return lines;
        }

        let primary_column_width = self.get_primary_column_width();

        let start_index = (self.selected_index.saturating_sub(self.max_visible / 2))
            .min(self.filtered_items.len().saturating_sub(self.max_visible));
        let end_index = (start_index + self.max_visible).min(self.filtered_items.len());

        for i in start_index..end_index {
            let item = &self.filtered_items[i];
            let is_selected = i == self.selected_index;
            let desc = item
                .description
                .as_ref()
                .map(|d| normalize_to_single_line(d));
            lines.push(self.render_item(
                item,
                is_selected,
                width,
                desc.as_deref(),
                primary_column_width,
            ));
        }

        if start_index > 0 || end_index < self.filtered_items.len() {
            let scroll = format!(
                "  ({}/{})",
                self.selected_index + 1,
                self.filtered_items.len()
            );
            lines.push((self.theme.scroll_info)(&truncate_to_width(
                &scroll,
                width.saturating_sub(2),
                "",
                false,
            )));
        }

        lines
    }

    fn handle_input(&mut self, event: crate::tui::InputEvent) {
        use crate::tui::InputEvent;
        let InputEvent::Key(ref key) = event else {
            return;
        };
        let up = with_keybindings(|kb| kb.matches_event(key, "tui.select.up"));
        let down = with_keybindings(|kb| kb.matches_event(key, "tui.select.down"));
        let confirm = with_keybindings(|kb| kb.matches_event(key, "tui.select.confirm"));
        let cancel = with_keybindings(|kb| kb.matches_event(key, "tui.select.cancel"));

        if up {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_items.len().saturating_sub(1)
            } else {
                self.selected_index - 1
            };
            self.notify_selection_change();
        } else if down {
            self.selected_index =
                if self.selected_index >= self.filtered_items.len().saturating_sub(1) {
                    0
                } else {
                    self.selected_index + 1
                };
            self.notify_selection_change();
        } else if confirm {
            if let Some(item) = self.filtered_items.get(self.selected_index).cloned()
                && let Some(ref mut cb) = self.on_select
            {
                cb(item);
            }
        } else if cancel && let Some(ref mut cb) = self.on_cancel {
            cb();
        }
    }

    fn invalidate(&mut self) {}
}

impl SelectList {
    fn render_item(
        &self,
        item: &SelectItem,
        is_selected: bool,
        width: usize,
        desc: Option<&str>,
        primary_column_width: usize,
    ) -> String {
        let prefix = if is_selected { "→ " } else { "  " };
        let prefix_width = visible_width(prefix);

        if let Some(desc) = desc
            && width > 40
        {
            let effective_primary = primary_column_width
                .min(width.saturating_sub(prefix_width + 4))
                .max(1);
            let max_primary = (effective_primary.saturating_sub(PRIMARY_COLUMN_GAP)).max(1);
            let truncated_value =
                self.truncate_primary(item, is_selected, max_primary, effective_primary);
            let truncated_width = visible_width(&truncated_value);
            let spacing = " ".repeat((effective_primary.saturating_sub(truncated_width)).max(1));
            let desc_start = prefix_width + truncated_width + spacing.len();
            let remaining = width.saturating_sub(desc_start + 2);

            if remaining > MIN_DESCRIPTION_WIDTH {
                let truncated_desc = truncate_to_width(desc, remaining, "", false);
                if is_selected {
                    return (self.theme.selected_text)(&format!(
                        "{}{}{}{}",
                        prefix, truncated_value, spacing, truncated_desc
                    ));
                }
                let desc_text = (self.theme.description)(&format!("{}{}", spacing, truncated_desc));
                return format!("{}{}{}", prefix, truncated_value, desc_text);
            }
        }

        let max_width = width.saturating_sub(prefix_width + 2);
        let truncated_value = self.truncate_primary(item, is_selected, max_width, max_width);
        if is_selected {
            (self.theme.selected_text)(&format!("{}{}", prefix, truncated_value))
        } else {
            format!("{}{}", prefix, truncated_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> SelectListTheme {
        SelectListTheme {
            selected_prefix: Box::new(|s| format!("\x1b[36m{}\x1b[39m", s)),
            selected_text: Box::new(|s| format!("\x1b[7m{}\x1b[27m", s)),
            description: Box::new(|s| format!("\x1b[2m{}\x1b[22m", s)),
            scroll_info: Box::new(|s| s.to_string()),
            no_match: Box::new(|s| s.to_string()),
        }
    }

    fn items() -> Vec<SelectItem> {
        vec![
            SelectItem::new("apple", "Apple").with_description("A sweet fruit"),
            SelectItem::new("banana", "Banana").with_description("A curved fruit"),
            SelectItem::new("cherry", "Cherry"),
        ]
    }

    #[test]
    fn test_select_list_renders_items() {
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        let lines = list.render(80);
        assert!(!lines.is_empty());
        assert!(
            lines[0].contains("Apple"),
            "first item should be Apple: {}",
            lines[0]
        );
    }

    #[test]
    fn test_select_list_selection_indicator() {
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        let lines = list.render(80);
        // First item (index 0) should have "→" prefix
        assert!(
            lines[0].contains("→"),
            "expected selection arrow: {}",
            lines[0]
        );
    }

    fn feed(list: &mut SelectList, code: crossterm::event::KeyCode) {
        use crate::tui::InputEvent;
        use crossterm::event::{KeyEvent, KeyModifiers};
        list.handle_input(InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    #[test]
    fn test_select_list_moves_selection_down() {
        use crossterm::event::KeyCode;
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        feed(&mut list, KeyCode::Down);
        assert_eq!(list.selected_index, 1);
    }

    #[test]
    fn test_select_list_wraps_selection_up() {
        use crossterm::event::KeyCode;
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        feed(&mut list, KeyCode::Up); // up wraps to bottom
        assert_eq!(list.selected_index, 2);
    }

    #[test]
    fn test_select_list_wraps_selection_down() {
        use crossterm::event::KeyCode;
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        feed(&mut list, KeyCode::Down); // 0->1
        feed(&mut list, KeyCode::Down); // 1->2
        feed(&mut list, KeyCode::Down); // wrap to 0
        assert_eq!(list.selected_index, 0);
    }

    #[test]
    fn test_select_list_confirm_fires_callback() {
        use crossterm::event::KeyCode;
        use std::sync::{Arc, Mutex};
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        let selected = Arc::new(Mutex::new(None));
        let s = selected.clone();
        list.on_select = Some(Box::new(move |item| {
            *s.lock().unwrap() = Some(item);
        }));
        feed(&mut list, KeyCode::Enter);
        let result = selected.lock().unwrap();
        assert!(result.is_some());
        assert_eq!(result.as_ref().unwrap().value, "apple");
    }

    #[test]
    fn test_select_list_cancel() {
        use crossterm::event::KeyCode;
        use std::sync::{Arc, Mutex};
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        let cancelled = Arc::new(Mutex::new(false));
        let c = cancelled.clone();
        list.on_cancel = Some(Box::new(move || {
            *c.lock().unwrap() = true;
        }));
        feed(&mut list, KeyCode::Esc);
        assert!(*cancelled.lock().unwrap());
    }

    #[test]
    fn test_select_list_filter() {
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        list.set_filter("b");
        assert_eq!(list.filtered_items.len(), 1);
        assert_eq!(list.filtered_items[0].value, "banana");
    }

    #[test]
    fn test_select_list_filter_no_match() {
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        list.set_filter("zzz");
        assert_eq!(list.filtered_items.len(), 0);
        let lines = list.render(80);
        assert!(lines.iter().any(|l| l.contains("No matching")));
    }

    #[test]
    fn test_select_list_get_selected() {
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        feed(&mut list, crossterm::event::KeyCode::Down); // move to 1
        let selected = list.get_selected_item().unwrap();
        assert_eq!(selected.value, "banana");
    }

    #[test]
    fn test_select_list_descriptions_rendered() {
        let mut list = SelectList::new(
            items(),
            10,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        let lines = list.render(100);
        assert!(
            lines[0].contains("sweet fruit"),
            "should contain description: {}",
            lines[0]
        );
    }

    #[test]
    fn test_select_list_scroll_indicator() {
        let many_items: Vec<SelectItem> = (0..20)
            .map(|i| SelectItem::new(format!("item{}", i), format!("Item {}", i)))
            .collect();
        let mut list = SelectList::new(
            many_items,
            5,
            theme(),
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            },
        );
        let lines = list.render(80);
        assert!(
            lines.iter().any(|l| l.contains("/20")),
            "should have scroll indicator"
        );
    }
}
