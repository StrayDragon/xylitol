use crate::scroll_view::ScrollView;
use crate::selection::ScreenRect;
use crate::tui::{Component, InputEvent, SizeValue};
use crate::utils::truncate_to_width;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPlacement {
    DockLeft,
    DockRight,
    FloatLeft,
    FloatRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelOverflow {
    Clip,
    Scroll,
}

#[derive(Debug, Clone)]
pub struct PanelBounds {
    pub width: SizeValue,
    pub min_width: usize,
    pub max_width: Option<usize>,
    pub min_height: usize,
    pub max_height: SizeValue,
    pub overflow: PanelOverflow,
}

pub struct PanelRegion {
    child: Box<dyn Component>,
    bounds: PanelBounds,
    placement: PanelPlacement,
    view: ScrollView,
    pending_scroll: Option<usize>,
    available: Option<ScreenRect>,
    last_rect: Option<ScreenRect>,
    content_rows: usize,
    last_width: usize,
    origin_col: u16,
    origin_row: u16,
}

impl PanelRegion {
    pub fn new(child: Box<dyn Component>, bounds: PanelBounds, placement: PanelPlacement) -> Self {
        Self {
            child,
            view: ScrollView::new(bounds.min_height.max(1)),
            pending_scroll: None,
            bounds,
            placement,
            available: None,
            last_rect: None,
            content_rows: 0,
            last_width: 0,
            origin_col: 0,
            origin_row: 0,
        }
    }

    pub fn set_bounds(&mut self, bounds: PanelBounds) {
        self.bounds = bounds;
    }

    pub fn bounds(&self) -> &PanelBounds {
        &self.bounds
    }

    pub fn placement(&self) -> PanelPlacement {
        self.placement
    }

    pub fn set_available(&mut self, rect: ScreenRect) {
        self.available = Some(rect);
    }

    pub fn rect(&self) -> Option<ScreenRect> {
        self.last_rect
    }

    pub fn content_rows(&self) -> usize {
        self.content_rows
    }

    pub fn scroll_top(&self) -> usize {
        self.pending_scroll
            .unwrap_or_else(|| self.view.scroll_top())
    }

    pub fn scroll_by(&mut self, delta: isize) -> bool {
        if self.last_rect.is_none() {
            return false;
        }
        self.pending_scroll = None;
        self.view.scroll_by(delta)
    }

    pub fn set_scroll(&mut self, top: usize) -> bool {
        self.pending_scroll = Some(top);
        true
    }

    pub fn scroll_to_top(&mut self) -> bool {
        self.pending_scroll = Some(0);
        self.view.scroll_home()
    }
}

impl Component for PanelRegion {
    fn render(&mut self, width: usize) -> Vec<String> {
        if width == 0 {
            self.last_rect = None;
            self.content_rows = 0;
            return Vec::new();
        }
        let max_w = self.bounds.max_width.unwrap_or(width).min(width).max(1);
        let panel_w = width
            .min(max_w)
            .max(self.bounds.min_width.min(width))
            .max(1);
        let avail_h = self
            .available
            .map(|rect| rect.height as usize)
            .unwrap_or(usize::MAX);
        let max_h = self.bounds.max_height.resolve(avail_h).min(avail_h).max(1);
        let min_h = self.bounds.min_height.min(max_h).max(1);

        let child_lines = self.child.render(panel_w);
        self.content_rows = child_lines.len();
        let target_h = child_lines.len().clamp(min_h, max_h);

        let mut visible = match self.bounds.overflow {
            PanelOverflow::Scroll => {
                self.view.set_viewport_height(target_h);
                self.view.set_lines(child_lines);
                if let Some(top) = self.pending_scroll.take() {
                    self.view.set_scroll_top(top);
                }
                self.view.visible_lines().to_vec()
            }
            PanelOverflow::Clip => {
                self.pending_scroll = None;
                let mut lines = child_lines;
                lines.truncate(target_h);
                lines
            }
        };
        while visible.len() < target_h {
            visible.push(String::new());
        }
        visible.truncate(target_h);
        for line in &mut visible {
            *line = truncate_to_width(line, panel_w, "", false);
        }

        let (row, col) = self
            .available
            .map(|rect| (rect.row, rect.col))
            .unwrap_or((0, 0));
        self.last_rect = Some(ScreenRect {
            row,
            col,
            height: target_h.min(u16::MAX as usize) as u16,
            width: panel_w.min(u16::MAX as usize) as u16,
        });
        self.last_width = panel_w;
        visible
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.child.handle_input(event);
    }

    fn invalidate(&mut self) {
        self.child.invalidate();
        self.view.set_lines(Vec::new());
        self.content_rows = 0;
    }

    fn set_screen_origin(&mut self, col: u16, row: u16) {
        self.origin_col = col;
        self.origin_row = row;
        self.child.set_screen_origin(col, row);
    }

    fn offer_pointer(&mut self, event: &InputEvent) -> bool {
        let InputEvent::Mouse(mouse) = event else {
            return false;
        };
        let Some(rect) = self.last_rect else {
            return false;
        };
        if mouse.row < self.origin_row
            || mouse.row >= self.origin_row.saturating_add(rect.height)
            || mouse.column < self.origin_col
            || mouse.column >= self.origin_col.saturating_add(rect.width)
        {
            return false;
        }
        match mouse.kind {
            crossterm::event::MouseEventKind::ScrollUp => {
                self.scroll_by(-ScrollView::wheel_notch());
                true
            }
            crossterm::event::MouseEventKind::ScrollDown => {
                self.scroll_by(ScrollView::wheel_notch());
                true
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

    struct Lines(usize);

    impl Component for Lines {
        fn render(&mut self, _width: usize) -> Vec<String> {
            (0..self.0).map(|i| format!("line{i}")).collect()
        }

        fn handle_input(&mut self, _event: InputEvent) {}
        fn invalidate(&mut self) {}
    }

    fn bounds(
        min_w: usize,
        max_w: usize,
        min_h: usize,
        max_h: usize,
        overflow: PanelOverflow,
    ) -> PanelBounds {
        PanelBounds {
            width: SizeValue::Absolute(min_w),
            min_width: min_w,
            max_width: Some(max_w),
            min_height: min_h,
            max_height: SizeValue::Absolute(max_h),
            overflow,
        }
    }

    #[test]
    fn clamps_width_and_height() {
        let mut panel = PanelRegion::new(
            Box::new(Lines(20)),
            bounds(24, 40, 3, 10, PanelOverflow::Clip),
            PanelPlacement::DockRight,
        );
        panel.set_available(ScreenRect {
            row: 0,
            col: 10,
            height: 30,
            width: 80,
        });
        let lines = panel.render(30);
        assert_eq!(lines.len(), 10);
        let rect = panel.rect().expect("rect");
        assert_eq!(rect.width, 30);
        assert_eq!(rect.height, 10);
    }

    #[test]
    fn pads_short_content_to_min_height() {
        let mut panel = PanelRegion::new(
            Box::new(Lines(1)),
            bounds(24, 40, 3, 10, PanelOverflow::Clip),
            PanelPlacement::DockRight,
        );
        panel.set_available(ScreenRect {
            row: 0,
            col: 0,
            height: 30,
            width: 80,
        });
        let lines = panel.render(30);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "");
    }

    #[test]
    fn scrolls_overflow_content() {
        let mut panel = PanelRegion::new(
            Box::new(Lines(10)),
            bounds(24, 40, 3, 3, PanelOverflow::Scroll),
            PanelPlacement::DockRight,
        );
        panel.set_available(ScreenRect {
            row: 0,
            col: 0,
            height: 30,
            width: 80,
        });
        let lines = panel.render(30);
        assert_eq!(lines.len(), 3);
        assert_eq!(panel.content_rows(), 10);
        assert!(panel.scroll_by(1));
        assert_eq!(panel.scroll_top(), 1);
    }

    #[test]
    fn zero_width_is_safe() {
        let mut panel = PanelRegion::new(
            Box::new(Lines(3)),
            bounds(24, 40, 3, 10, PanelOverflow::Clip),
            PanelPlacement::FloatRight,
        );
        assert!(panel.render(0).is_empty());
    }
    #[test]
    fn scroll_by_clears_pending_scroll() {
        let mut panel = PanelRegion::new(
            Box::new(Lines(20)),
            bounds(24, 40, 3, 10, PanelOverflow::Scroll),
            PanelPlacement::DockRight,
        );
        panel.set_available(ScreenRect {
            row: 0,
            col: 10,
            height: 30,
            width: 80,
        });
        panel.render(30);
        assert!(panel.set_scroll(5));
        assert_eq!(panel.scroll_top(), 5);
        assert!(panel.scroll_by(2));
        assert_eq!(panel.scroll_top(), 2, "scroll_by must clear pending_scroll");
    }

    #[test]
    fn clip_mode_discards_pending_scroll_on_render() {
        let mut panel = PanelRegion::new(
            Box::new(Lines(20)),
            bounds(24, 40, 3, 10, PanelOverflow::Clip),
            PanelPlacement::DockRight,
        );
        panel.set_available(ScreenRect {
            row: 0,
            col: 10,
            height: 30,
            width: 80,
        });
        assert!(panel.set_scroll(5));
        assert_eq!(panel.scroll_top(), 5);
        panel.render(30);
        assert_eq!(
            panel.scroll_top(),
            0,
            "clip mode must discard pending scroll on render"
        );
    }
}
