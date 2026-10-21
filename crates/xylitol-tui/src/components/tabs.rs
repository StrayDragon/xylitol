use crate::keybindings::with_keybindings;
use crate::tui::{Component, Focusable, InputEvent};
use crate::utils::{strip_ansi_codes, truncate_to_width, visible_width};
use crossterm::event::{MouseButton, MouseEventKind};

#[allow(clippy::type_complexity)]
pub struct TabsTheme {
    pub title: Box<dyn Fn(&str, bool) -> String>,
    pub divider: Box<dyn Fn(&str) -> String>,
    pub overflow: Box<dyn Fn(&str) -> String>,
}

impl Default for TabsTheme {
    fn default() -> Self {
        Self {
            title: Box::new(|s, selected| {
                if selected {
                    format!("\x1b[7m{s}\x1b[27m")
                } else {
                    s.to_string()
                }
            }),
            divider: Box::new(|s| s.to_string()),
            overflow: Box::new(|s| s.to_string()),
        }
    }
}

pub struct Tabs {
    titles: Vec<String>,
    selected: usize,
    origin: (u16, u16),
    focused: bool,
    theme: TabsTheme,
    on_change: Option<Box<dyn FnMut(usize)>>,
    title_ranges: Vec<(usize, usize, usize)>,
    overflow_ranges: Vec<(usize, usize, usize, usize)>,
    rendered: bool,
    last_width: usize,
    pointer_changed: bool,
}

impl Tabs {
    pub fn new(titles: Vec<String>) -> Self {
        Self {
            titles,
            selected: 0,
            origin: (0, 0),
            focused: false,
            theme: TabsTheme::default(),
            on_change: None,
            title_ranges: Vec::new(),
            overflow_ranges: Vec::new(),
            rendered: false,
            last_width: 0,
            pointer_changed: false,
        }
    }

    pub fn with_theme(mut self, theme: TabsTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_on_change(mut self, cb: impl FnMut(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(cb));
        self
    }

    pub fn titles(&self) -> &[String] {
        &self.titles
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn set_titles(&mut self, titles: Vec<String>) {
        self.titles = titles;
        self.clamp_selected();
        self.invalidate();
    }

    pub fn set_selected(&mut self, index: usize) {
        if self.titles.is_empty() {
            self.selected = 0;
            return;
        }
        let next = index.min(self.titles.len() - 1);
        self.apply_selected(next);
    }

    pub fn set_screen_origin(&mut self, col: u16, row: u16) {
        self.origin = (col, row);
    }

    pub fn title_col_ranges(&self) -> Vec<(usize, usize, usize)> {
        self.title_ranges.clone()
    }

    pub fn overflow_col_ranges(&self) -> Vec<(usize, usize, usize, usize)> {
        self.overflow_ranges.clone()
    }

    pub fn offer_pointer(&mut self, event: &InputEvent) -> bool {
        self.pointer_changed = false;
        let InputEvent::Mouse(mouse) = event else {
            return false;
        };
        if !self.rendered || self.titles.is_empty() {
            return false;
        }
        if mouse.row != self.origin.1 {
            return false;
        }
        let local_col = (mouse.column as i32) - (self.origin.0 as i32);
        if local_col < 0 || (local_col as usize) >= self.last_width {
            return false;
        }
        let col = local_col as usize;
        match mouse.kind {
            MouseEventKind::Moved => false,
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(index) = self.hit_index(col) {
                    self.pointer_changed = self.apply_selected(index);
                }
                true
            }
            MouseEventKind::ScrollUp => {
                self.pointer_changed = self.cycle(1);
                true
            }
            MouseEventKind::ScrollDown => {
                self.pointer_changed = self.cycle(-1);
                true
            }
            _ => false,
        }
    }

    fn clamp_selected(&mut self) {
        if self.titles.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.titles.len() {
            self.selected = self.titles.len() - 1;
        }
    }

    fn apply_selected(&mut self, next: usize) -> bool {
        if next == self.selected || self.titles.is_empty() {
            return false;
        }
        self.selected = next;
        if let Some(cb) = self.on_change.as_mut() {
            cb(self.selected);
        }
        true
    }

    fn cycle(&mut self, dir: i32) -> bool {
        let n = self.titles.len();
        if n == 0 {
            return false;
        }
        let next = (self.selected as i32 + dir).rem_euclid(n as i32) as usize;
        self.apply_selected(next)
    }

    fn hit_index(&self, col: usize) -> Option<usize> {
        for &(index, start, end) in &self.title_ranges {
            if col >= start && col < end {
                return Some(index);
            }
        }
        for &(index, start, end, _) in &self.overflow_ranges {
            if col >= start && col < end {
                return Some(index);
            }
        }
        None
    }
}

fn left_marker(count: usize) -> String {
    format!(" ← +{count} ")
}

fn right_marker(count: usize) -> String {
    format!(" +{count} → ")
}

fn marker_width(count: usize, left: bool) -> usize {
    if count == 0 {
        0
    } else if left {
        visible_width(&left_marker(count))
    } else {
        visible_width(&right_marker(count))
    }
}

fn window_used(widths: &[usize], start: usize, end: usize) -> usize {
    let n = widths.len();
    if start >= end || end > n {
        return usize::MAX;
    }
    let titles: usize = widths[start..end].iter().sum::<usize>() + (end - start).saturating_sub(1);
    let hidden_l = start;
    let hidden_r = n - end;
    marker_width(hidden_l, true) + titles + marker_width(hidden_r, false)
}

fn fit_window(widths: &[usize], selected: usize, width: usize) -> (usize, usize) {
    let n = widths.len();
    if n == 0 || width == 0 {
        return (0, 0);
    }
    let sel = selected.min(n - 1);
    let total: usize = widths.iter().sum::<usize>() + n.saturating_sub(1);
    if total <= width {
        return (0, n);
    }

    let mut start = sel;
    let mut end = sel + 1;
    let mut prefer_left = true;
    loop {
        let can_left = start > 0 && window_used(widths, start - 1, end) <= width;
        let can_right = end < n && window_used(widths, start, end + 1) <= width;
        if !can_left && !can_right {
            break;
        }
        if prefer_left && can_left {
            start -= 1;
        } else if can_right {
            end += 1;
        } else if can_left {
            start -= 1;
        } else {
            break;
        }
        prefer_left = !prefer_left;
    }
    (start, end - start)
}

impl Component for Tabs {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.rendered = true;
        self.last_width = width;
        self.title_ranges.clear();
        self.overflow_ranges.clear();
        if self.titles.is_empty() {
            return Vec::new();
        }
        if width == 0 {
            return vec![String::new()];
        }

        let widths: Vec<usize> = self.titles.iter().map(|t| visible_width(t)).collect();
        let n = widths.len();
        let selected = self.selected.min(n - 1);
        let (start, count) = fit_window(&widths, selected, width);
        let end = start + count;
        let hidden_l = start;
        let hidden_r = n.saturating_sub(end);

        let mut col = 0usize;
        let mut out = String::new();

        if hidden_l > 0 {
            let marker = left_marker(hidden_l);
            let mw = visible_width(&marker);
            self.overflow_ranges
                .push((start.saturating_sub(1), col, col + mw, hidden_l));
            out.push_str(&(self.theme.overflow)(&marker));
            col += mw;
        }

        for i in start..end {
            if i > start {
                out.push_str(&(self.theme.divider)(" "));
                col += 1;
            }
            let remaining_after = {
                let mr = marker_width(hidden_r, false);
                width.saturating_sub(col).saturating_sub(mr)
            };
            let raw = if widths[i] > remaining_after {
                truncate_to_width(&self.titles[i], remaining_after, "", false)
            } else {
                self.titles[i].clone()
            };
            let tw = visible_width(&raw);
            self.title_ranges.push((i, col, col + tw));
            out.push_str(&(self.theme.title)(&raw, i == selected));
            col += tw;
        }

        if hidden_r > 0 {
            let marker = right_marker(hidden_r);
            let mw = visible_width(&marker);
            self.overflow_ranges
                .push((start + count, col, col + mw, hidden_r));
            out.push_str(&(self.theme.overflow)(&marker));
            col += mw;
        }

        if col < width {
            out.push_str(&" ".repeat(width - col));
        } else if visible_width(&strip_ansi_codes(&out)) > width {
            out = truncate_to_width(&strip_ansi_codes(&out), width, "", false);
        }

        vec![out]
    }

    fn handle_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key(ref key) => {
                let prev = with_keybindings(|kb| kb.matches_event(key, "tui.tabs.prev"));
                let next = with_keybindings(|kb| kb.matches_event(key, "tui.tabs.next"));
                if prev {
                    self.cycle(-1);
                } else if next {
                    self.cycle(1);
                }
            }
            InputEvent::Mouse(_) => {
                let _ = Tabs::offer_pointer(self, &event);
            }
            InputEvent::Paste(_) => {}
        }
    }

    fn invalidate(&mut self) {
        self.title_ranges.clear();
        self.overflow_ranges.clear();
        self.rendered = false;
        self.last_width = 0;
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        if matches!(event, InputEvent::Mouse(_)) {
            self.pointer_changed
        } else {
            true
        }
    }

    fn set_screen_origin(&mut self, col: u16, row: u16) {
        self.origin = (col, row);
    }

    fn offer_pointer(&mut self, event: &InputEvent) -> bool {
        Tabs::offer_pointer(self, event)
    }
}

impl Focusable for Tabs {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }
}
