use super::{Component, TUI};
use crate::terminal::Terminal;
use crate::utils::visible_width;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAnchor {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    LeftCenter,
    RightCenter,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OverlayMargin {
    pub top: Option<usize>,
    pub right: Option<usize>,
    pub bottom: Option<usize>,
    pub left: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum SizeValue {
    Absolute(usize),
    Percent(f64),
}

impl SizeValue {
    pub fn resolve(&self, reference: usize) -> usize {
        match self {
            SizeValue::Absolute(n) => *n,
            SizeValue::Percent(p) => ((reference as f64) * p / 100.0).floor() as usize,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OverlayOptions {
    pub width: Option<SizeValue>,
    pub min_width: Option<usize>,
    pub max_height: Option<SizeValue>,
    pub anchor: Option<OverlayAnchor>,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub row: Option<SizeValue>,
    pub col: Option<SizeValue>,
    pub margin: Option<OverlayMargin>,
    pub non_capturing: bool,
}

/// Focus routing target: a root child index or an overlay id (c575 / D08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusTarget {
    Root(usize),
    Overlay(u64),
}

/// Options for [`OverlayHandle::unfocus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayUnfocusOptions {
    /// Explicit target after releasing this overlay (may be `None` = clear focus).
    pub target: Option<FocusTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverlayFocusRestorePolicy {
    Clear,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockedResume {
    RestoreOverlay,
    FocusTarget(Option<FocusTarget>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverlayFocusRestore {
    Inactive,
    Eligible {
        overlay_id: u64,
    },
    Blocked {
        overlay_id: u64,
        blocked_by: FocusTarget,
        resume: BlockedResume,
    },
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OverlayStackEntry {
    overlay_id: u64,
    /// Focus target before this overlay opened (root or overlay); restored on hide.
    pub(super) pre_focus: Option<FocusTarget>,
    pub(super) hidden: bool,
    focus_order: u64,
}

/// Handle returned by [`TUI::show_overlay`] for controlling one overlay entry.
///
/// Operations take `&mut TUI` because the overlay component is owned by the
/// TUI. After [`Self::hide`], further calls are no-ops (id no longer on stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayHandle {
    overlay_id: u64,
}

impl OverlayHandle {
    pub fn overlay_id(self) -> u64 {
        self.overlay_id
    }

    /// Permanently remove this overlay (cannot be shown again via this handle).
    pub fn hide<T: Terminal>(self, tui: &mut TUI<T>) {
        tui.hide_overlay(self);
    }

    pub fn set_hidden<T: Terminal>(self, tui: &mut TUI<T>, hidden: bool) {
        tui.set_overlay_hidden(self, hidden);
    }

    pub fn is_focused<T: Terminal>(self, tui: &TUI<T>) -> bool {
        tui.is_overlay_focused(self)
    }

    pub fn focus<T: Terminal>(self, tui: &mut TUI<T>) {
        tui.focus_overlay(self);
    }

    /// Release focus (optional explicit [`OverlayUnfocusOptions::target`]).
    pub fn unfocus<T: Terminal>(self, tui: &mut TUI<T>, options: Option<OverlayUnfocusOptions>) {
        tui.unfocus_overlay(self, options);
    }
}

impl<T: Terminal> TUI<T> {
    /// Show an overlay and return a handle for hide / focus control.
    pub fn show_overlay(
        &mut self,
        component: Box<dyn Component>,
        options: OverlayOptions,
    ) -> OverlayHandle {
        let overlay_id = self.next_overlay_id;
        self.next_overlay_id = self.next_overlay_id.saturating_add(1);
        self.focus_order_counter += 1;
        let entry = OverlayStackEntry {
            overlay_id,
            pre_focus: self.current_focus_target(),
            hidden: false,
            focus_order: self.focus_order_counter,
        };
        let capturing = !options.non_capturing;
        self.overlays.push((component, options, entry));
        if capturing {
            self.set_focus_internal(
                Some(FocusTarget::Overlay(overlay_id)),
                OverlayFocusRestorePolicy::Clear,
            );
        }
        self.terminal.hide_cursor();
        OverlayHandle { overlay_id }
    }

    pub(super) fn overlay_index(&self, overlay_id: u64) -> Option<usize> {
        self.overlays
            .iter()
            .position(|(_, _, e)| e.overlay_id == overlay_id)
    }

    pub(super) fn is_overlay_entry_visible(&self, entry: &OverlayStackEntry) -> bool {
        !entry.hidden
    }

    pub(super) fn topmost_visible_capturing_overlay_id(&self) -> Option<u64> {
        self.overlays
            .iter()
            .filter(|(_, opts, e)| !opts.non_capturing && self.is_overlay_entry_visible(e))
            .max_by_key(|(_, _, e)| e.focus_order)
            .map(|(_, _, e)| e.overlay_id)
    }

    pub(super) fn current_focus_target(&self) -> Option<FocusTarget> {
        if let Some(id) = self.focused_overlay_id {
            return Some(FocusTarget::Overlay(id));
        }
        self.focused_index.map(FocusTarget::Root)
    }

    fn apply_focus_target(&mut self, target: Option<FocusTarget>) {
        match target {
            Some(FocusTarget::Overlay(id)) => {
                self.focused_overlay_id = Some(id);
            }
            Some(FocusTarget::Root(i)) => {
                self.focused_overlay_id = None;
                self.focused_index = Some(i);
            }
            None => {
                self.focused_overlay_id = None;
                self.focused_index = None;
            }
        }
    }

    pub(super) fn clear_overlay_focus_restore(&mut self) {
        self.overlay_focus_restore = OverlayFocusRestore::Inactive;
    }

    fn clear_overlay_focus_restore_for(&mut self, overlay_id: u64) {
        let clear = match self.overlay_focus_restore {
            OverlayFocusRestore::Eligible { overlay_id: id }
            | OverlayFocusRestore::Blocked { overlay_id: id, .. } => id == overlay_id,
            OverlayFocusRestore::Inactive => false,
        };
        if clear {
            self.clear_overlay_focus_restore();
        }
    }

    pub(super) fn get_visible_overlay_focus_restore(&self) -> OverlayFocusRestore {
        match self.overlay_focus_restore {
            OverlayFocusRestore::Inactive => OverlayFocusRestore::Inactive,
            OverlayFocusRestore::Eligible { overlay_id }
            | OverlayFocusRestore::Blocked { overlay_id, .. } => {
                let visible = self
                    .overlay_index(overlay_id)
                    .is_some_and(|i| self.is_overlay_entry_visible(&self.overlays[i].2));
                if visible {
                    self.overlay_focus_restore
                } else {
                    OverlayFocusRestore::Inactive
                }
            }
        }
    }

    fn is_focus_target_mounted(&self, target: FocusTarget) -> bool {
        match target {
            FocusTarget::Root(i) => i < self.components.len(),
            FocusTarget::Overlay(id) => self
                .overlay_index(id)
                .is_some_and(|i| self.is_overlay_entry_visible(&self.overlays[i].2)),
        }
    }

    fn is_overlay_focus_ancestor(&self, overlay_id: u64, target: FocusTarget) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut current = self
            .overlay_index(overlay_id)
            .and_then(|i| self.overlays[i].2.pre_focus);
        while let Some(node) = current {
            if !visited.insert(node) {
                break;
            }
            if node == target {
                return true;
            }
            current = match node {
                FocusTarget::Overlay(id) => self
                    .overlay_index(id)
                    .and_then(|i| self.overlays[i].2.pre_focus),
                FocusTarget::Root(_) => None,
            };
        }
        false
    }

    fn retarget_overlay_pre_focus(&mut self, removed_id: u64, removed_pre: Option<FocusTarget>) {
        let removed_target = FocusTarget::Overlay(removed_id);
        for (_, _, entry) in &mut self.overlays {
            if entry.pre_focus == Some(removed_target) {
                entry.pre_focus = removed_pre;
            }
        }
    }

    fn resolve_blocked_resume(&mut self, state: OverlayFocusRestore) -> Option<FocusTarget> {
        match state {
            OverlayFocusRestore::Blocked {
                overlay_id,
                resume: BlockedResume::RestoreOverlay,
                ..
            } => Some(FocusTarget::Overlay(overlay_id)),
            OverlayFocusRestore::Blocked {
                resume: BlockedResume::FocusTarget(target),
                ..
            } => {
                self.clear_overlay_focus_restore();
                target
            }
            _ => None,
        }
    }

    pub(super) fn set_focus_internal(
        &mut self,
        mut next: Option<FocusTarget>,
        policy: OverlayFocusRestorePolicy,
    ) {
        let previous = self.current_focus_target();
        let previous_focused_overlay = match previous {
            Some(FocusTarget::Overlay(id))
                if self
                    .overlay_index(id)
                    .is_some_and(|i| self.is_overlay_entry_visible(&self.overlays[i].2)) =>
            {
                Some(id)
            }
            _ => None,
        };
        let next_is_overlay = matches!(next, Some(FocusTarget::Overlay(_)));
        let restore_state = self.get_visible_overlay_focus_restore();

        if let Some(next_target) = next
            && !next_is_overlay
        {
            if let OverlayFocusRestore::Blocked {
                blocked_by,
                overlay_id,
                resume,
            } = restore_state
                && Some(blocked_by) == previous
            {
                if matches!(resume, BlockedResume::FocusTarget(_))
                    || !self.is_focus_target_mounted(blocked_by)
                {
                    next = self.resolve_blocked_resume(OverlayFocusRestore::Blocked {
                        overlay_id,
                        blocked_by,
                        resume,
                    });
                } else {
                    self.overlay_focus_restore = OverlayFocusRestore::Blocked {
                        overlay_id,
                        blocked_by: next_target,
                        resume,
                    };
                }
            } else if let Some(prev_overlay) = previous_focused_overlay
                && !matches!(restore_state, OverlayFocusRestore::Inactive)
                && matches!(
                    restore_state,
                    OverlayFocusRestore::Eligible { overlay_id }
                        | OverlayFocusRestore::Blocked { overlay_id, .. }
                        if overlay_id == prev_overlay
                )
                && !self.is_overlay_focus_ancestor(prev_overlay, next_target)
            {
                self.overlay_focus_restore = OverlayFocusRestore::Blocked {
                    overlay_id: prev_overlay,
                    blocked_by: next_target,
                    resume: BlockedResume::RestoreOverlay,
                };
            }
        } else if next.is_none() {
            if let OverlayFocusRestore::Blocked {
                blocked_by,
                overlay_id,
                resume,
            } = restore_state
                && Some(blocked_by) == previous
            {
                next = self.resolve_blocked_resume(OverlayFocusRestore::Blocked {
                    overlay_id,
                    blocked_by,
                    resume,
                });
            } else if policy == OverlayFocusRestorePolicy::Clear {
                self.clear_overlay_focus_restore();
            }
        }

        self.apply_focus_target(next);

        if let Some(FocusTarget::Overlay(id)) = next
            && self
                .overlay_index(id)
                .is_some_and(|i| self.is_overlay_entry_visible(&self.overlays[i].2))
        {
            self.overlay_focus_restore = OverlayFocusRestore::Eligible { overlay_id: id };
        }
    }

    /// Permanently remove the overlay identified by `handle`.
    pub fn hide_overlay(&mut self, handle: OverlayHandle) {
        let Some(index) = self.overlay_index(handle.overlay_id) else {
            return;
        };
        let (_, _, entry) = self.overlays.remove(index);
        self.clear_overlay_focus_restore_for(entry.overlay_id);
        self.retarget_overlay_pre_focus(entry.overlay_id, entry.pre_focus);
        let was_focused = self.focused_overlay_id == Some(handle.overlay_id);
        if was_focused {
            let top = self.topmost_visible_capturing_overlay_id();
            let target = top.map(FocusTarget::Overlay).or(entry.pre_focus);
            self.set_focus_internal(target, OverlayFocusRestorePolicy::Clear);
        }
        if self.overlays.is_empty() {
            self.terminal.hide_cursor();
        }
        self.request_render(false);
    }

    pub fn set_overlay_hidden(&mut self, handle: OverlayHandle, hidden: bool) {
        let Some(index) = self.overlay_index(handle.overlay_id) else {
            return;
        };
        if self.overlays[index].2.hidden == hidden {
            return;
        }
        self.overlays[index].2.hidden = hidden;
        let non_capturing = self.overlays[index].1.non_capturing;
        let pre_focus = self.overlays[index].2.pre_focus;
        if hidden {
            self.clear_overlay_focus_restore_for(handle.overlay_id);
            if self.focused_overlay_id == Some(handle.overlay_id) {
                let top = self.topmost_visible_capturing_overlay_id();
                let target = top.map(FocusTarget::Overlay).or(pre_focus);
                self.set_focus_internal(target, OverlayFocusRestorePolicy::Clear);
            }
        } else if !non_capturing {
            self.focus_order_counter += 1;
            self.overlays[index].2.focus_order = self.focus_order_counter;
            self.set_focus_internal(
                Some(FocusTarget::Overlay(handle.overlay_id)),
                OverlayFocusRestorePolicy::Clear,
            );
        }
        self.request_render(false);
    }

    pub fn is_overlay_hidden(&self, handle: OverlayHandle) -> bool {
        self.overlay_index(handle.overlay_id)
            .map(|i| self.overlays[i].2.hidden)
            .unwrap_or(true)
    }

    pub fn is_overlay_focused(&self, handle: OverlayHandle) -> bool {
        self.focused_overlay_id == Some(handle.overlay_id)
            && self
                .overlay_index(handle.overlay_id)
                .is_some_and(|i| !self.overlays[i].2.hidden)
    }

    /// Bring overlay to the visual front and capture focus (including non_capturing).
    pub fn focus_overlay(&mut self, handle: OverlayHandle) {
        let Some(index) = self.overlay_index(handle.overlay_id) else {
            return;
        };
        if self.overlays[index].2.hidden {
            return;
        }
        self.focus_order_counter += 1;
        self.overlays[index].2.focus_order = self.focus_order_counter;
        self.set_focus_internal(
            Some(FocusTarget::Overlay(handle.overlay_id)),
            OverlayFocusRestorePolicy::Clear,
        );
        self.request_render(false);
    }

    /// Release focus from this overlay (optional explicit target).
    pub fn unfocus_overlay(
        &mut self,
        handle: OverlayHandle,
        options: Option<OverlayUnfocusOptions>,
    ) {
        let Some(index) = self.overlay_index(handle.overlay_id) else {
            return;
        };
        let is_focused = self.focused_overlay_id == Some(handle.overlay_id);
        let has_pending = match self.overlay_focus_restore {
            OverlayFocusRestore::Eligible { overlay_id }
            | OverlayFocusRestore::Blocked { overlay_id, .. } => overlay_id == handle.overlay_id,
            OverlayFocusRestore::Inactive => false,
        };
        if !is_focused && !has_pending {
            return;
        }

        if let OverlayFocusRestore::Blocked {
            overlay_id,
            blocked_by,
            ..
        } = self.overlay_focus_restore
            && overlay_id == handle.overlay_id
            && self.current_focus_target() == Some(blocked_by)
        {
            if let Some(opts) = options {
                self.overlay_focus_restore = OverlayFocusRestore::Blocked {
                    overlay_id,
                    blocked_by,
                    resume: BlockedResume::FocusTarget(opts.target),
                };
            } else {
                self.clear_overlay_focus_restore();
            }
            self.request_render(false);
            return;
        }

        self.clear_overlay_focus_restore_for(handle.overlay_id);
        if is_focused || options.is_some() {
            let pre_focus = self.overlays[index].2.pre_focus;
            let top = self
                .overlays
                .iter()
                .filter(|(_, opts, e)| {
                    e.overlay_id != handle.overlay_id
                        && !opts.non_capturing
                        && self.is_overlay_entry_visible(e)
                })
                .max_by_key(|(_, _, e)| e.focus_order)
                .map(|(_, _, e)| e.overlay_id);
            let fallback = top.map(FocusTarget::Overlay).or(pre_focus);
            let target = match options {
                Some(OverlayUnfocusOptions { target }) => target,
                None => fallback,
            };
            self.set_focus_internal(target, OverlayFocusRestorePolicy::Clear);
        }
        self.request_render(false);
    }

    pub(super) fn composite_overlays(
        &mut self,
        mut lines: Vec<String>,
        term_width: usize,
        term_height: usize,
    ) -> Vec<String> {
        if self.overlays.is_empty() {
            return lines;
        }

        let mut indices: Vec<usize> = (0..self.overlays.len())
            .filter(|&i| !self.overlays[i].2.hidden)
            .collect();
        indices.sort_by_key(|&i| self.overlays[i].2.focus_order);

        // Collect layout info before rendering (avoid borrow conflicts)
        struct LayoutInfo {
            width: usize,
            max_height: Option<usize>,
            opts: OverlayOptions,
        }
        let mut layouts: Vec<LayoutInfo> = Vec::new();
        for &i in &indices {
            let opts = &self.overlays[i].1;
            let mut w = opts
                .width
                .as_ref()
                .map(|sv| sv.resolve(term_width))
                .unwrap_or(term_width.min(80));
            if let Some(mw) = opts.min_width {
                w = w.max(mw);
            }
            w = w.clamp(1, term_width);
            let mh = opts.max_height.as_ref().map(|h| h.resolve(term_height));
            layouts.push(LayoutInfo {
                width: w,
                max_height: mh,
                opts: opts.clone(),
            });
        }

        // Render each overlay
        let mut rendered: Vec<(Vec<String>, usize, usize)> = Vec::new();
        for (idx, layout) in indices.iter().zip(layouts.iter()) {
            let ov_lines = {
                let (comp, _opts, _) = &mut self.overlays[*idx];
                let mut ov = comp.render(layout.width);
                if let Some(mh) = layout.max_height {
                    ov.truncate(mh);
                }
                ov
            };
            let (row, col) = self.resolve_overlay_position(
                &layout.opts,
                ov_lines.len(),
                layout.width,
                term_width,
                term_height,
            );
            rendered.push((ov_lines, row, col));
        }

        // workingHeight = max(content, screen, minLinesNeeded). Deliberately
        // excludes maxLinesRendered — including it historically caused
        // self-reinforcing scrollback growth on width changes (pi tui.ts:1067).
        // minLinesNeeded covers overlays anchored below the content end.
        let min_lines_needed = rendered
            .iter()
            .map(|(ov, row, _)| row + ov.len())
            .max()
            .unwrap_or(0);
        let working_height = lines.len().max(term_height).max(min_lines_needed);
        lines.resize(working_height, String::new());

        // Overlay screen coords are relative to the visible viewport top; map
        // them to absolute buffer rows via viewportStart (pi tui.ts:1074).
        let viewport_start = working_height.saturating_sub(term_height);

        for (ov_lines, row, col) in rendered {
            for (i, ov_line) in ov_lines.iter().enumerate() {
                let abs_row = viewport_start + row + i;
                if let Some(base) = lines.get_mut(abs_row) {
                    *base = self.composite_line(base, ov_line, col, term_width);
                }
            }
        }
        lines
    }

    fn resolve_overlay_position(
        &self,
        _opts: &OverlayOptions,
        _h: usize,
        _w: usize,
        _tw: usize,
        _th: usize,
    ) -> (usize, usize) {
        // Simplified anchor-based positioning
        let anchor = _opts.anchor.unwrap_or(OverlayAnchor::Center);
        let mt = _opts.margin.unwrap_or_default().top.unwrap_or(0);
        let ml = _opts.margin.unwrap_or_default().left.unwrap_or(0);

        let row = match anchor {
            OverlayAnchor::Center
            | OverlayAnchor::TopCenter
            | OverlayAnchor::BottomCenter
            | OverlayAnchor::LeftCenter
            | OverlayAnchor::RightCenter => mt + (_th.saturating_sub(_h)) / 2,
            OverlayAnchor::TopLeft | OverlayAnchor::TopRight => mt,
            OverlayAnchor::BottomLeft | OverlayAnchor::BottomRight => mt + _th.saturating_sub(_h),
        };

        let col = match anchor {
            OverlayAnchor::Center | OverlayAnchor::LeftCenter | OverlayAnchor::RightCenter => {
                ml + (_tw.saturating_sub(_w)) / 2
            }
            OverlayAnchor::TopLeft | OverlayAnchor::BottomLeft => ml,
            OverlayAnchor::TopRight | OverlayAnchor::BottomRight => ml + _tw.saturating_sub(_w),
            OverlayAnchor::TopCenter | OverlayAnchor::BottomCenter => {
                ml + (_tw.saturating_sub(_w)) / 2
            }
        };

        (
            row + _opts.offset_y.unwrap_or(0).max(0) as usize,
            col + _opts.offset_x.unwrap_or(0).max(0) as usize,
        )
    }

    fn composite_line(&self, base: &str, overlay: &str, col: usize, total_width: usize) -> String {
        use crate::utils::{extract_segments, slice_by_column, slice_with_width};

        let reset = "\x1b[0m\x1b]8;;\x07";
        let ow = visible_width(overlay);
        let after_start = col + ow;
        let after_len = total_width.saturating_sub(after_start);

        // Single pass: split the base into before[0,col) and after[afterStart,...).
        // after inherits the SGR active at col so an overlay laid mid-line doesn't
        // leave the trailing content unstyled (pi compositeLineAt via extractSegments).
        let seg = extract_segments(base, col, after_start, after_len, true);

        // Overlay slice (strict — reject wide chars crossing the overlay's right
        // edge). Truncate the overlay to its declared width if it overflows.
        let overlay_slice = if ow > 0 {
            slice_with_width(overlay, 0, ow, true).0
        } else {
            String::new()
        };

        // Pad before up to col, then reset, then overlay, then reset.
        let before_pad = " ".repeat(col.saturating_sub(seg.before_width));
        let before_part = format!("{}{}{}", seg.before, before_pad, reset);

        let overlay_part = if after_len == 0 {
            // No room for after; overlay (truncated) + reset is all that fits.
            format!("{}{}", overlay_slice, reset)
        } else {
            let after_pad = " ".repeat(after_len.saturating_sub(seg.after_width));
            format!(
                "{}{}{}{}{}",
                reset, overlay_slice, reset, seg.after, after_pad
            )
        };

        let mut result = format!("{}{}", before_part, overlay_part);

        // Final safety net (pi compositeLineAt tui.ts:1218): if anything still
        // overflowed (wide-char rounding, unexpected ANSI), hard-slice to width.
        if visible_width(&result) > total_width {
            result = slice_by_column(&result, 0, total_width);
        }
        result
    }
}
