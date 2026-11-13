use crossterm::event::{MouseButton, MouseEventKind};

use crate::keybindings::{Keybinding, with_keybindings};
use crate::scroll_gutter::{gutter_active, stamp_gutter};
use crate::scroll_view::ScrollView;
use crate::tui::{Component, InputEvent};
use crate::utils::wrap_text_with_ansi;

pub struct ScrollRegion {
    view: ScrollView,
    content: Vec<String>,
    wrapped: Vec<String>,
    height: usize,
    rendered: bool,
    last_width: usize,
    origin_col: u16,
    origin_row: u16,
    page_up: Keybinding,
    page_down: Keybinding,
}

impl ScrollRegion {
    pub fn new(height: usize, page_up: Keybinding, page_down: Keybinding) -> Self {
        Self {
            view: ScrollView::new(height.max(1)),
            content: Vec::new(),
            wrapped: Vec::new(),
            height,
            rendered: false,
            last_width: 0,
            origin_col: 0,
            origin_row: 0,
            page_up,
            page_down,
        }
    }

    pub fn set_content(&mut self, content: Vec<String>) {
        self.content = content;
        self.wrapped.clear();
        self.view.set_lines(Vec::new());
        self.view.set_scroll_top(0);
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
        self.view.set_viewport_height(height.max(1));
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn content_rows(&self) -> usize {
        self.wrapped.len()
    }

    pub fn last_width(&self) -> usize {
        self.last_width
    }

    pub fn scroll_top(&self) -> usize {
        self.view.scroll_top()
    }

    pub fn scroll_by(&mut self, delta: isize) -> bool {
        if self.height == 0 {
            return false;
        }
        self.view.scroll_by(delta)
    }

    pub fn scroll_to_top(&mut self) -> bool {
        self.view.scroll_home()
    }

    pub fn scroll_to_bottom(&mut self) -> bool {
        self.view.set_scroll_top(usize::MAX)
    }

    pub fn page_up(&mut self) -> bool {
        self.scroll_by(-(self.height as isize))
    }

    pub fn page_down(&mut self) -> bool {
        self.scroll_by(self.height as isize)
    }

    fn wrapped_at(&self, width: usize) -> Vec<String> {
        self.content
            .iter()
            .flat_map(|line| wrap_text_with_ansi(line, width.max(1)))
            .collect()
    }

    fn gutter_visible(&self) -> bool {
        gutter_active(self.view.content_len(), self.height, self.last_width)
    }
}

impl Component for ScrollRegion {
    fn render(&mut self, width: usize) -> Vec<String> {
        if width == 0 {
            self.rendered = false;
            self.last_width = 0;
            return Vec::new();
        }
        if self.height == 0 {
            self.rendered = false;
            self.wrapped.clear();
            self.view.set_lines(Vec::new());
            return Vec::new();
        }
        self.rendered = true;
        self.last_width = width;
        let full = self.wrapped_at(width);
        let gutter = gutter_active(full.len(), self.height, width);
        let content_width = if gutter {
            width.saturating_sub(1).max(1)
        } else {
            width
        };
        let wrapped = if gutter {
            self.wrapped_at(content_width)
        } else {
            full
        };
        self.wrapped = wrapped.clone();
        self.view.set_viewport_height(self.height);
        self.view.set_lines(wrapped);
        let mut visible = self.view.visible_lines().to_vec();
        while visible.len() < self.height {
            visible.push(String::new());
        }
        if gutter {
            stamp_gutter(
                &mut visible,
                width,
                self.view.scroll_top(),
                self.view.content_len(),
            );
        }
        visible.truncate(self.height);
        visible
    }

    fn handle_input(&mut self, event: InputEvent) {
        let InputEvent::Key(ref key) = event else {
            return;
        };
        let page_up = with_keybindings(|kb| kb.matches_event(key, self.page_up));
        let page_down = with_keybindings(|kb| kb.matches_event(key, self.page_down));
        if page_up {
            self.page_up();
        }
        if page_down {
            self.page_down();
        }
    }

    fn invalidate(&mut self) {
        self.rendered = false;
        self.wrapped.clear();
        self.view.set_lines(Vec::new());
    }

    fn set_screen_origin(&mut self, col: u16, row: u16) {
        self.origin_col = col;
        self.origin_row = row;
    }

    fn offer_pointer(&mut self, event: &InputEvent) -> bool {
        let InputEvent::Mouse(mouse) = event else {
            return false;
        };
        if !self.rendered || self.height == 0 || self.last_width == 0 {
            return false;
        }
        let bottom = self.origin_row.saturating_add(self.height as u16);
        let right = self.origin_col.saturating_add(self.last_width as u16);
        if mouse.row < self.origin_row || mouse.row >= bottom {
            return false;
        }
        if mouse.column < self.origin_col || mouse.column >= right {
            return false;
        }
        let rel = (mouse.row - self.origin_row) as usize;
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_by(-ScrollView::wheel_notch());
                true
            }
            MouseEventKind::ScrollDown => {
                self.scroll_by(ScrollView::wheel_notch());
                true
            }
            MouseEventKind::Down(MouseButton::Left) if self.gutter_visible() => {
                let gutter_col = self
                    .origin_col
                    .saturating_add(self.last_width.saturating_sub(1) as u16);
                if mouse.column != gutter_col {
                    return false;
                }
                if rel == 0 {
                    self.scroll_to_top();
                    true
                } else if rel + 1 == self.height {
                    self.scroll_to_bottom();
                    true
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    fn input_wants_rerender(&self, _event: &InputEvent) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::visible_width;
    use crossterm::event::MouseEvent;

    fn region(height: usize, content: Vec<&str>) -> ScrollRegion {
        let mut region = ScrollRegion::new(height, "app.slot.pageUp", "app.slot.pageDown");
        region.set_content(content.into_iter().map(str::to_string).collect());
        region
    }

    #[test]
    fn wraps_and_clips_to_viewport_height() {
        let long = "x".repeat(40);
        let mut region = region(3, vec![long.as_str()]);
        let lines = region.render(20);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| visible_width(line) <= 20));
        assert_eq!(region.content_rows(), 2);
    }

    #[test]
    fn pads_short_content_to_viewport_height() {
        let mut region = region(5, vec!["a", "b"]);
        let lines = region.render(20);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "a");
        assert_eq!(lines[1], "b");
        assert!(lines[2..].iter().all(|line| line.is_empty()));
    }

    #[test]
    fn page_down_clamps_to_max_scroll() {
        let mut region = region(3, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
        let _ = region.render(20);
        assert!(region.page_down());
        assert_eq!(region.scroll_top(), 3);
        assert!(region.page_down());
        assert_eq!(region.scroll_top(), 5);
        assert!(!region.page_down());
    }

    #[test]
    fn wheel_scrolls_by_notch_inside_rect_only() {
        let mut region = region(3, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
        let _ = region.render(20);
        region.set_screen_origin(0, 4);
        let inside = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 4,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(region.offer_pointer(&inside));
        assert_eq!(region.scroll_top(), ScrollView::wheel_notch() as usize);
        let outside = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 20,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(!region.offer_pointer(&outside));
    }

    #[test]
    fn gutter_top_and_bottom_clicks_jump() {
        let mut region = region(3, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
        let lines = region.render(20);
        assert!(lines[0].ends_with('^'));
        assert!(lines[2].ends_with('v'));
        region.set_screen_origin(0, 0);
        let bottom = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 19,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(region.offer_pointer(&bottom));
        assert_eq!(region.scroll_top(), region.content_rows() - 3);
        let top = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 19,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(region.offer_pointer(&top));
        assert_eq!(region.scroll_top(), 0);
    }

    #[test]
    fn wheel_does_not_move_when_content_fits() {
        let mut region = region(5, vec!["a", "b"]);
        let _ = region.render(20);
        region.set_screen_origin(0, 0);
        let wheel = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(region.offer_pointer(&wheel));
        assert_eq!(region.scroll_top(), 0);
    }

    #[test]
    fn gutter_track_click_consumes_without_action() {
        let mut region = region(3, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
        let _ = region.render(20);
        region.set_screen_origin(0, 0);
        assert!(region.scroll_by(3));
        let track = InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 19,
            row: 1,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(region.offer_pointer(&track));
        assert_eq!(region.scroll_top(), 3);
    }

    #[test]
    fn no_gutter_when_content_fits() {
        let mut region = region(5, vec!["a", "b"]);
        let lines = region.render(20);
        assert!(
            lines
                .iter()
                .all(|line| !line.ends_with('^') && !line.ends_with('v'))
        );
    }

    #[test]
    fn zero_width_is_safe() {
        let mut region = region(3, vec!["a"]);
        assert!(region.render(0).is_empty());
    }
}
