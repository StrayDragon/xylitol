use std::time::{Duration, Instant};

use crate::pointer::{HitId, HitRegionList, apply_hover_tint};
use crate::scroll_gutter::{ScrollGutter, gutter_active, stamp_gutter};
use crate::scroll_view::ScrollView;
use crate::selection::{ClipboardSink, ScreenRect, SelectionController, format_osc52};
use crossterm::event::{MouseButton, MouseEvent};

pub const COPY_NOTICE_TTL: Duration = Duration::from_millis(2000);

pub struct ApplicationOwnedRuntime {
    pub scroll: ScrollView,
    pub selection: SelectionController,
    pub dock_rows: usize,
    pub pending_clipboard: Vec<String>,
    follow_bottom: bool,
    last_dock_lines: Vec<String>,
    pending_copy_notice: bool,
    copy_notice_until: Option<Instant>,
    hover_id: Option<HitId>,
    hover_regions: HitRegionList,
    scroll_gutter: Option<ScrollGutter>,
    gutter_drag_grab: Option<usize>,
    extra_reserved_cols: u16,
}

impl ApplicationOwnedRuntime {
    #[must_use]
    pub fn reserved_cols(&self) -> usize {
        usize::from(self.scroll_gutter.is_some())
    }

    pub fn set_extra_reserved_cols(&mut self, n: u16) {
        self.extra_reserved_cols = n;
    }

    pub fn extra_reserved_cols(&self) -> u16 {
        self.extra_reserved_cols
    }

    fn content_right(&self, term_cols: usize) -> usize {
        term_cols
            .saturating_sub(self.reserved_cols())
            .saturating_sub(usize::from(self.extra_reserved_cols))
    }
}

impl ApplicationOwnedRuntime {
    pub fn new(dock_rows: usize) -> Self {
        Self {
            scroll: ScrollView::new(1),
            selection: SelectionController::new(),
            dock_rows: dock_rows.max(1),
            pending_clipboard: Vec::new(),
            follow_bottom: true,
            last_dock_lines: Vec::new(),
            pending_copy_notice: false,
            copy_notice_until: None,
            hover_id: None,
            hover_regions: HitRegionList::new(),
            scroll_gutter: None,
            gutter_drag_grab: None,
            extra_reserved_cols: 0,
        }
    }

    pub fn set_hover(&mut self, id: Option<HitId>, regions: HitRegionList) {
        self.hover_id = id;
        self.hover_regions = regions;
    }

    pub fn set_scroll_gutter(&mut self, gutter: Option<ScrollGutter>) {
        self.scroll_gutter = gutter;
    }

    pub fn set_dock_rows(&mut self, rows: usize) {
        self.dock_rows = rows.max(1);
    }

    pub fn set_copy_on_release(&mut self, on: bool) {
        self.selection.copy_on_release = on;
    }

    pub fn set_hit_priority(&mut self, hit: Option<crate::selection::HitPriorityFn>) {
        self.selection.set_hit_priority(hit);
    }

    pub fn has_selection(&self) -> bool {
        self.selection.has_selection()
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection.selected_text(self.scroll.lines())
    }

    pub fn session_lines_for_main_scrollback(&self) -> Vec<String> {
        let mut lines = self.scroll.lines().to_vec();
        lines.extend(self.last_dock_lines.iter().cloned());
        lines
    }

    pub fn has_projected_content(&self) -> bool {
        !self.last_dock_lines.is_empty() || self.scroll.content_len() > 0
    }

    pub fn content_len(&self) -> usize {
        self.scroll.content_len()
    }

    pub fn content_lines(&self) -> &[String] {
        self.scroll.lines()
    }

    pub fn dock_line_count(&self) -> usize {
        self.last_dock_lines.len()
    }

    pub fn scroll_top(&self) -> usize {
        self.scroll.scroll_top()
    }

    pub fn scroll_by(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        if !self.scroll.scroll_by(delta) {
            return false;
        }
        self.follow_bottom = self.scroll.at_bottom();
        true
    }

    pub fn scroll_to_end(&mut self) -> bool {
        let was_at_bottom = self.scroll.at_bottom();
        self.scroll.scroll_to_end();
        self.follow_bottom = true;
        !was_at_bottom
    }

    pub fn scroll_home(&mut self) -> bool {
        if !self.scroll.scroll_home() {
            return false;
        }
        self.follow_bottom = self.scroll.at_bottom();
        true
    }

    pub fn ingest_wheel_delta(&mut self, delta: isize) -> bool {
        self.scroll_by(delta)
    }

    pub fn wheel_notch(&self) -> isize {
        ScrollView::wheel_notch()
    }

    pub fn motion_step(&self) -> isize {
        ScrollView::motion_step(self.scroll.viewport_height())
    }

    pub fn project_frame(
        &mut self,
        full_lines: &[String],
        term_height: usize,
        term_cols: usize,
    ) -> Vec<String> {
        let dock = self.dock_rows.min(full_lines.len()).min(term_height);
        let content_end = full_lines.len().saturating_sub(dock);
        let content = &full_lines[..content_end];
        let dock_lines = &full_lines[content_end..];
        self.last_dock_lines = dock_lines.to_vec();

        if self.selection.has_selection() && !self.selection.is_dragging() {
            let top_row = self
                .selection
                .bounds()
                .map(|(start, _)| start.row)
                .unwrap_or(0);
            if Self::prefix_changed(self.scroll.lines(), content, top_row) {
                self.selection.clear();
            }
        }

        let viewport_h = term_height.saturating_sub(dock);
        self.scroll.set_viewport_height(viewport_h.max(1));
        self.scroll.set_lines(content.to_vec());
        self.paint_visible(viewport_h, term_height, term_cols)
    }

    fn prefix_changed(old: &[String], new: &[String], upto: usize) -> bool {
        let n = upto.saturating_add(1);
        if old.len() < n || new.len() < n {
            return true;
        }
        old[..n] != new[..n]
    }

    pub fn reproject_frame(&mut self, term_height: usize, term_cols: usize) -> Vec<String> {
        let dock = self
            .dock_rows
            .min(self.last_dock_lines.len())
            .min(term_height);
        let viewport_h = term_height.saturating_sub(dock);
        self.scroll.set_viewport_height(viewport_h.max(1));
        self.paint_visible(viewport_h, term_height, term_cols)
    }

    fn paint_visible(
        &mut self,
        viewport_h: usize,
        term_height: usize,
        term_cols: usize,
    ) -> Vec<String> {
        if self.follow_bottom && !self.selection.is_dragging() {
            self.scroll.scroll_to_end();
        }

        let mut visible = self.scroll.visible_lines().to_vec();
        self.selection
            .apply_highlight(&mut visible, self.scroll.scroll_top());
        if let Some(id) = self.hover_id
            && !self.selection.is_dragging()
        {
            apply_hover_tint(
                &mut visible,
                self.scroll.scroll_top(),
                &self.hover_regions,
                id,
            );
        }
        if self.scroll_gutter.is_some()
            && gutter_active(self.scroll.content_len(), viewport_h, term_cols)
        {
            let top = self.scroll.scroll_top();
            let content = self.scroll.content_len();
            stamp_gutter(
                &mut visible,
                self.content_right(term_cols).saturating_add(1),
                top,
                content,
            );
        }
        while visible.len() < viewport_h {
            visible.push(String::new());
        }
        visible.truncate(viewport_h);
        visible.extend(self.last_dock_lines.iter().cloned());
        if visible.len() > term_height {
            visible.truncate(term_height);
        }
        visible
    }

    pub fn handle_mouse(&mut self, event: &MouseEvent, term_cols: u16, term_rows: u16) -> bool {
        use crossterm::event::MouseEventKind;

        let dock = self.dock_rows.min(term_rows as usize) as u16;
        let transcript_h = term_rows.saturating_sub(dock);
        let content_w = self.content_right(term_cols as usize) as u16;
        let transcript = ScreenRect {
            row: 0,
            col: 0,
            height: transcript_h,
            width: content_w,
        };
        let wheel_delta = match event.kind {
            MouseEventKind::ScrollUp => Some(-self.wheel_notch()),
            MouseEventKind::ScrollDown => Some(self.wheel_notch()),
            _ => None,
        };
        if let Some(delta) = wheel_delta {
            let content_right = self.content_right(usize::from(term_cols)) as u16;
            if event.row >= transcript_h
                || event.column >= term_cols
                || event.column > content_right
            {
                return false;
            }
            return self.ingest_wheel_delta(delta);
        }

        if self.scroll_gutter.is_some() {
            let gutter_col = self.content_right(term_cols as usize) as u16;
            let on_track =
                event.column == gutter_col && (event.row as usize) < transcript_h as usize;
            let dragging = self.gutter_drag_grab.is_some();
            if event.kind == MouseEventKind::Up(MouseButton::Left) && dragging {
                self.gutter_drag_grab = None;
            } else if on_track || dragging {
                let row = event.row as usize;
                let inner = crate::scroll_gutter::gutter_inner_rows(transcript_h as usize);
                let inner_row = row.saturating_sub(1).min(inner.saturating_sub(1));
                let content = self.scroll.content_len();
                match event.kind {
                    MouseEventKind::Down(MouseButton::Left)
                        if on_track && row > 0 && row < transcript_h as usize - 1 && inner > 0 =>
                    {
                        let vh = transcript_h as usize;
                        let thumb_len = crate::scroll_gutter::gutter_thumb_len(content, vh);
                        let thumb_off = crate::scroll_gutter::gutter_thumb_offset(
                            self.scroll.scroll_top(),
                            content,
                            vh,
                        );
                        let on_thumb = inner_row >= thumb_off && inner_row < thumb_off + thumb_len;
                        let grab = if on_thumb {
                            inner_row - thumb_off
                        } else {
                            thumb_len / 2
                        };
                        let target = crate::scroll_gutter::gutter_scroll_for_pointer(
                            inner_row, grab, content, vh,
                        );
                        let delta = target as isize - self.scroll.scroll_top() as isize;
                        self.scroll_by(delta);
                        self.clear_selection();
                        self.gutter_drag_grab = Some(grab);
                        return true;
                    }
                    MouseEventKind::Drag(MouseButton::Left) if dragging => {
                        let grab = self.gutter_drag_grab.expect("dragging");
                        let target = crate::scroll_gutter::gutter_scroll_for_pointer(
                            inner_row,
                            grab,
                            content,
                            transcript_h as usize,
                        );
                        let delta = target as isize - self.scroll.scroll_top() as isize;
                        if self.scroll_by(delta) {
                            self.clear_selection();
                            return true;
                        }
                        return true;
                    }
                    _ => {}
                }
            }
        }

        let dock_rect = ScreenRect {
            row: transcript_h,
            col: 0,
            height: dock,
            width: term_cols,
        };
        let mut sink = CollectOsc52Sink {
            out: &mut self.pending_clipboard,
        };
        let dirty =
            self.selection
                .handle_mouse(event, &mut self.scroll, transcript, dock_rect, &mut sink);
        if self.selection.last_event_copied() {
            self.signal_copy_notice();
        }
        if dirty {
            self.follow_bottom = self.scroll.at_bottom();
        }
        dirty
    }

    pub fn tick_autoscroll(&mut self, term_cols: u16, term_rows: u16) -> bool {
        let dock = self.dock_rows.min(term_rows as usize) as u16;
        let transcript_h = term_rows.saturating_sub(dock);
        self.scroll
            .set_viewport_height((transcript_h as usize).max(1));
        let transcript = ScreenRect {
            row: 0,
            col: 0,
            height: transcript_h,
            width: self.content_right(term_cols as usize) as u16,
        };
        let dirty = self.selection.tick_autoscroll(&mut self.scroll, transcript);
        if dirty {
            self.follow_bottom = self.scroll.at_bottom();
        }
        dirty
    }

    pub fn tick_copy_notice(&mut self) -> bool {
        let Some(until) = self.copy_notice_until else {
            return false;
        };
        if Instant::now() < until {
            return false;
        }
        self.copy_notice_until = None;
        self.pending_copy_notice = false;
        true
    }

    pub fn take_copy_notice(&mut self) -> bool {
        std::mem::take(&mut self.pending_copy_notice)
    }

    pub fn copy_notice_active(&self) -> bool {
        self.copy_notice_until
            .is_some_and(|until| Instant::now() < until)
    }

    fn signal_copy_notice(&mut self) {
        self.pending_copy_notice = true;
        self.copy_notice_until = Some(Instant::now() + COPY_NOTICE_TTL);
    }

    pub fn enqueue_clipboard_seqs(&mut self, seqs: impl IntoIterator<Item = String>) {
        let mut any = false;
        for seq in seqs {
            if !seq.is_empty() {
                self.pending_clipboard.push(seq);
                any = true;
            }
        }
        if any {
            self.signal_copy_notice();
        }
    }

    pub fn take_pending_clipboard(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_clipboard)
    }
}

struct CollectOsc52Sink<'a> {
    out: &'a mut Vec<String>,
}

impl ClipboardSink for CollectOsc52Sink<'_> {
    fn copy_text(&mut self, text: &str) {
        if let Some(seq) = format_osc52(text) {
            self.out.push(seq);
        } else {
            let _ = text;
        }
    }
}

#[cfg(test)]
pub fn test_runtime(dock_rows: usize) -> ApplicationOwnedRuntime {
    ApplicationOwnedRuntime::new(dock_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn project_frame_caps_height_and_keeps_dock() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let paint = runtime.project_frame(&full, 8, 40);
        assert_eq!(paint.len(), 8);
        assert_eq!(paint[6], "L18");
        assert_eq!(paint[7], "L19");
        assert_eq!(paint[0], "L12");
    }

    #[test]
    fn wheel_scroll_persists_across_project_frame() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 8, 40);
        assert!(runtime.scroll.at_bottom());
        let top_before = runtime.scroll.scroll_top();
        let notch = runtime.wheel_notch();
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&wheel, 40, 8));
        assert_eq!(runtime.scroll.scroll_top(), top_before - notch as usize);
        let top_scrolled = runtime.scroll.scroll_top();
        let paint = runtime.project_frame(&full, 8, 40);
        assert_eq!(runtime.scroll.scroll_top(), top_scrolled);
        assert_eq!(paint[0], format!("L{top_scrolled}"));
    }

    #[test]
    fn wheel_multi_notch_applies_fully_immediately() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..40).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 10, 40);
        let notch = runtime.wheel_notch();
        assert_eq!(notch, 3);
        let top0 = runtime.scroll.scroll_top();
        assert!(runtime.ingest_wheel_delta(-(notch * 3)));
        assert_eq!(runtime.scroll.scroll_top(), top0 - (notch * 3) as usize);
        assert!(
            !runtime.tick_autoscroll(40, 10),
            "normal wheel motion must not drip through the selection tick path"
        );
        assert_eq!(runtime.scroll.scroll_top(), top0 - (notch * 3) as usize);
    }

    #[test]
    fn scroll_to_end_rearms_follow_mode() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 8, 40);
        assert!(runtime.scroll_by(-runtime.wheel_notch()));
        assert!(!runtime.follow_bottom);

        assert!(runtime.scroll_to_end());
        assert!(runtime.scroll.at_bottom());
        assert!(runtime.follow_bottom);
        assert!(!runtime.scroll_to_end());
    }

    #[test]
    fn dock_that_fills_terminal_keeps_dock_and_disables_transcript_selection() {
        let mut runtime = ApplicationOwnedRuntime::new(8);
        let full: Vec<String> = (0..8).map(|i| format!("L{i}")).collect();
        let paint = runtime.project_frame(&full, 4, 40);
        assert_eq!(paint, vec!["L4", "L5", "L6", "L7"]);

        assert!(!runtime.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            40,
            4,
        ));
        assert!(!runtime.selection.is_dragging());
    }

    #[test]
    fn copy_on_release_signals_notice_empty_does_not() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = vec![
            "hello world".into(),
            "second".into(),
            "status".into(),
            "input".into(),
        ];
        let _ = runtime.project_frame(&full, 6, 40);
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 1, 0), 40, 6));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), 40, 6));
        assert!(!runtime.copy_notice_active());
        assert!(!runtime.take_copy_notice());

        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), 40, 6));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Drag(MouseButton::Left), 5, 0), 40, 6));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 5, 0), 40, 6));
        assert!(runtime.copy_notice_active());
        assert!(runtime.take_copy_notice());
        assert!(!runtime.take_copy_notice());
        assert!(runtime.copy_notice_active());
        assert!(!runtime.take_pending_clipboard().is_empty());
    }

    #[test]
    fn copy_disabled_does_not_signal_notice() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_copy_on_release(false);
        let full: Vec<String> = vec!["abcd".into(), "e".into(), "s".into(), "i".into()];
        let _ = runtime.project_frame(&full, 6, 40);
        runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), 40, 6);
        runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 3, 0), 40, 6);
        assert!(!runtime.copy_notice_active());
        assert!(!runtime.take_copy_notice());
        assert!(runtime.take_pending_clipboard().is_empty());
    }

    #[test]
    fn copy_notice_expires_on_tick() {
        let mut runtime = ApplicationOwnedRuntime::new(1);
        runtime.signal_copy_notice();
        runtime.copy_notice_until = Some(Instant::now() - Duration::from_millis(1));
        assert!(!runtime.copy_notice_active());
        assert!(runtime.tick_copy_notice());
        assert!(!runtime.take_copy_notice());
        assert!(!runtime.tick_copy_notice());
    }

    fn drag_select_rows_1_to_2(runtime: &mut ApplicationOwnedRuntime) {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let ev = |kind, col: u16, row: u16| MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Down(MouseButton::Left), 0, 1), 20, 6));
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Drag(MouseButton::Left), 2, 2), 20, 6));
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Up(MouseButton::Left), 2, 2), 20, 6));
    }

    fn reflow_fixture(tail: &[&str], first_row: &str) -> Vec<String> {
        let mut v: Vec<String> = ["alpha", "beta", "gamma", "delta", "echo"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if first_row != "alpha" {
            v[0] = first_row.to_string();
        }
        v.extend(tail.iter().map(|s| s.to_string()));
        v.push("dock".into());
        v
    }

    #[test]
    fn completed_selection_survives_append_below_and_clears_on_reflow_above() {
        let mut runtime = ApplicationOwnedRuntime::new(1);
        let full1 = reflow_fixture(&[], "alpha");
        let _ = runtime.project_frame(&full1, 6, 40);
        drag_select_rows_1_to_2(&mut runtime);
        assert!(runtime.has_selection(), "drag must complete with selection");

        let full2 = reflow_fixture(&["foxtrot"], "alpha");
        let _ = runtime.project_frame(&full2, 7, 40);
        assert!(runtime.has_selection(), "append below must not clear");

        let full3 = reflow_fixture(&["foxtrot"], "alpha wraps differently now");
        let _ = runtime.project_frame(&full3, 7, 40);
        assert!(!runtime.has_selection(), "reflow above must invalidate");
    }

    #[test]
    fn gutter_paint_matches_across_project_and_reproject() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        let full: Vec<String> = (0..20).map(|i| format!("L{i:0>3}")).collect();
        let painted = runtime.project_frame(&full, 8, 40);
        assert!(painted[0].ends_with('^'), "top row, got {:?}", painted[0]);
        let mid = painted[3].chars().last().unwrap();
        assert!(
            mid == '\u{2502}' || mid == '\u{2503}',
            "track/thumb row, got {mid:?}"
        );
        assert_eq!(crate::utils::visible_width(&painted[0]), 40);
        assert!(runtime.scroll_by(-1));
        let repainted = runtime.reproject_frame(8, 40);
        assert!(repainted[0].ends_with('^'));
        assert!(repainted[5].ends_with('v'));
        assert!(
            repainted[6].ends_with("L018"),
            "dock unstamped, got {:?}",
            repainted[6]
        );
    }

    #[test]
    fn gutter_hidden_when_content_fits_or_too_narrow() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        let short: Vec<String> = vec!["aa".into(), "bb".into(), "dock".into()];
        let painted = runtime.project_frame(&short, 8, 40);
        assert_eq!(painted[0], "aa");
        let full: Vec<String> = (0..20).map(|i| format!("L{i:0>3}")).collect();
        let narrow = runtime.project_frame(&full, 8, 19);
        assert_eq!(narrow[0], "L012");
        let wide = runtime.project_frame(&full, 8, 40);
        assert!(wide[0].ends_with('^'), "got {:?}", wide[0]);
    }

    #[test]
    fn reserved_cols_and_carved_selection_copy() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        assert_eq!(runtime.reserved_cols(), 0, "default off");
        runtime.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        assert_eq!(runtime.reserved_cols(), 1);

        let body = "x".repeat(30);
        let full: Vec<String> = (0..20).map(|i| format!("L{i:0>3}{body}")).collect();
        let _ = runtime.project_frame(&full, 8, 40);
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 1), 40, 8));
        assert!(runtime.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 39, 2),
            40,
            8
        ));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 39, 2), 40, 8));
        let text = runtime
            .selection
            .selected_text(runtime.scroll.lines())
            .unwrap_or_default();
        assert!(
            !text.contains('^') && !text.contains('|') && !text.contains('v'),
            "copy must exclude gutter glyphs, got {text:?}"
        );
    }

    #[test]
    fn wheel_over_gutter_column_still_scrolls() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 8, 40);
        let top = runtime.scroll_top();
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 39,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&wheel, 40, 8));
        assert!(runtime.scroll_top() < top);
    }

    #[test]
    fn hover_dim_skipped_while_dragging() {
        use crate::pointer::HitRegion;

        let mut runtime = ApplicationOwnedRuntime::new(1);
        let full = vec!["header-line".into(), "body".into(), "dock".into()];
        let _ = runtime.project_frame(&full, 3, 20);

        let mut regions = HitRegionList::new();
        regions.push(HitRegion::new(1, 0, 0, 11));
        runtime.set_hover(Some(1), regions);
        let hovered = runtime.reproject_frame(3, 40);
        assert!(
            hovered[0].contains("\x1b[2m"),
            "idle hover must dim the header, got {:?}",
            hovered[0]
        );

        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 1), 20, 3));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Drag(MouseButton::Left), 3, 1), 20, 3));
        assert!(runtime.selection.is_dragging());
        let dragging = runtime.reproject_frame(3, 40);
        assert!(
            !dragging[0].contains("\x1b[2m"),
            "drag must suppress hover dim on the header, got {:?}",
            dragging[0]
        );
    }

    #[test]
    fn active_drag_is_not_cancelled_by_reflow_above() {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let mut runtime = ApplicationOwnedRuntime::new(1);
        let full1 = reflow_fixture(&[], "alpha");
        let _ = runtime.project_frame(&full1, 6, 40);
        let ev = |kind, col: u16, row: u16| MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Down(MouseButton::Left), 0, 1), 20, 6));
        assert!(runtime.selection.is_dragging());

        let full2 = reflow_fixture(&[], "alpha wraps differently now");
        let _ = runtime.project_frame(&full2, 6, 40);
        assert!(runtime.selection.is_dragging());
    }

    #[test]
    fn gutter_track_drag_jumps_and_follows() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        let full: Vec<String> = (0..100).map(|i| format!("L{i:0>3}")).collect();
        let _ = runtime.project_frame(&full, 20, 40);
        let vh = 18usize;
        let content = 98usize;

        assert!(runtime.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 39, 9),
            40,
            20
        ));
        let jumped = runtime.scroll_top();
        let expect = crate::scroll_gutter::gutter_scroll_for_pointer(8, 1, content, vh);
        assert_eq!(jumped, expect, "thumb-center grab jump");
        assert!(runtime.gutter_drag_grab.is_some());
        assert!(!runtime.has_selection(), "drag clears selection");

        assert!(runtime.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 39, 3),
            40,
            20
        ));
        assert_eq!(
            runtime.scroll_top(),
            crate::scroll_gutter::gutter_scroll_for_pointer(2, 1, content, vh),
        );

        let _ = runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 39, 3), 40, 20);
        assert!(runtime.gutter_drag_grab.is_none());
    }

    #[test]
    fn gutter_button_rows_not_hijacked_by_drag() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        let full: Vec<String> = (0..100).map(|i| format!("L{i:0>3}")).collect();
        let _ = runtime.project_frame(&full, 20, 40);
        let _ = runtime.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 39, 0),
            40,
            20,
        );
        assert!(runtime.gutter_drag_grab.is_none());
    }
}

#[cfg(test)]
mod c2910_tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn extra_reserved_owns_sidebar_band() {
        let mut rt = ApplicationOwnedRuntime::new(1);
        rt.set_scroll_gutter(Some(crate::scroll_gutter::ScrollGutter));
        rt.set_extra_reserved_cols(24);
        let full: Vec<String> = (0..100).map(|i| format!("L{i:0>3}")).collect();
        let _ = rt.project_frame(&full, 8, 40);
        let up = mouse(MouseEventKind::ScrollUp, 1, 1);
        assert!(rt.handle_mouse(&up, 40, 8));
        let top = rt.scroll_top();
        assert!(top > 0, "seed must be away from head");
        let before = rt.scroll_top();
        let band = mouse(MouseEventKind::ScrollUp, 30, 1);
        assert!(!rt.handle_mouse(&band, 40, 8), "band wheel must not scroll");
        assert_eq!(rt.scroll_top(), before, "band wheel must not move scroll");
        assert!(rt.handle_mouse(&up, 40, 8));
        assert_ne!(rt.scroll_top(), before, "content wheel still scrolls");
        assert_eq!(rt.content_right(40), 40 - 1 - 24);
    }
}
