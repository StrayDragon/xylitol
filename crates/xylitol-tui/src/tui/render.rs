use super::diff::{previous_body, shifted_previous_index};
use super::{CURSOR_MARKER, RenderError, TUI};
use crate::application_owned_runtime::ApplicationOwnedRuntime;
use crate::terminal::Terminal;
use crate::terminal_image::is_image_line;
use crate::utils::{normalize_terminal_output, visible_width};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderPerfSnap {
    pub do_render_us: u64,
    pub component_lines: usize,
    pub paint_lines: usize,
    pub finalize_width_checks: u64,
    pub finalize_line_reuses: u64,
    pub ao_reprojected: bool,
    pub ao_vertical_shifted: bool,
}

pub(super) const MIN_RENDER_INTERVAL_MS: u64 = 16;
pub(super) const BEGIN_RENDER_BATCH: &str = "\x1b[?2026h\x1b[?7l";
pub(super) const END_RENDER_BATCH: &str = "\x1b[?7h\x1b[?2026l";
pub(super) const SEGMENT_RESET: &str = "\x1b[0m\x1b]8;;\x07";
pub(super) const FORCE_SIZE_SENTINEL: usize = usize::MAX;

impl<T: Terminal> TUI<T> {
    pub fn render_frame(&mut self) -> Result<(), RenderError> {
        self.do_render()
    }

    pub fn request_render(&mut self, force: bool) {
        if force {
            self.previous_lines.clear();
            self.previous_width = FORCE_SIZE_SENTINEL;
            self.previous_height = FORCE_SIZE_SENTINEL;
            self.previous_viewport_top = 0;
            self.cursor_row = 0;
            self.hardware_cursor_row = 0;
            self.max_lines_rendered = 0;
            self.ao_components_stale = true;
            self.ao_wheel_render_requested = false;
        }
        self.render_requested = true;
    }

    pub fn try_render(&mut self) -> Result<bool, RenderError> {
        if !self.render_requested {
            return Ok(false);
        }
        let width = self.terminal.columns() as usize;
        let ao_wheel_reproject = self.ao_wheel_render_requested
            && self.can_reproject_application_owned(self.ao_render_width(width));
        let cadence_last = if ao_wheel_reproject {
            self.last_ao_wheel_render_at
        } else {
            self.last_render_at
        };
        if !self.ao_components_stale
            && cadence_last.is_some_and(|last| {
                last.elapsed() < std::time::Duration::from_millis(MIN_RENDER_INTERVAL_MS)
            })
        {
            return Ok(false);
        }
        self.render_requested = false;
        let now = std::time::Instant::now();
        self.last_render_at = Some(now);
        if ao_wheel_reproject {
            self.last_ao_wheel_render_at = Some(now);
        }
        self.do_render()?;
        Ok(true)
    }

    pub fn render_now(&mut self) -> Result<bool, RenderError> {
        let ao_wheel_reproject = self.ao_wheel_render_requested
            && self.can_reproject_application_owned(
                self.ao_render_width(self.terminal.columns() as usize),
            );
        self.render_requested = false;
        let now = std::time::Instant::now();
        self.last_render_at = Some(now);
        if ao_wheel_reproject {
            self.last_ao_wheel_render_at = Some(now);
        }
        self.do_render()?;
        Ok(!self.stopped)
    }

    pub(super) fn can_reproject_application_owned(&self, width: usize) -> bool {
        self.application_session_active
            && !self.ao_components_stale
            && self.previous_width == width
            && self.overlays.is_empty()
            && self
                .application_owned
                .as_ref()
                .is_some_and(ApplicationOwnedRuntime::has_projected_content)
    }

    pub(super) fn ao_render_width(&self, physical: usize) -> usize {
        if self.application_session_active {
            physical.saturating_sub(
                self.application_owned
                    .as_ref()
                    .map(ApplicationOwnedRuntime::reserved_cols)
                    .unwrap_or(0),
            )
        } else {
            physical
        }
    }

    pub(super) fn do_render(&mut self) -> Result<(), RenderError> {
        if self.stopped {
            return Ok(());
        }
        self.frame_count = self.frame_count.saturating_add(1);
        let width = self.terminal.columns() as usize;
        let height = self.terminal.rows() as usize;
        let render_w = self.ao_render_width(width);
        if width == 0 {
            return Ok(());
        }

        let render_started = std::time::Instant::now();
        let checks_before = self.finalize_width_checks;
        let reuses_before = self.finalize_line_reuses;

        let can_reproject = self.can_reproject_application_owned(render_w);
        self.refresh_pointer_hover_for_paint(can_reproject);

        let (component_lines, mut new_lines, ao_reprojected) = if can_reproject {
            let runtime = self
                .application_owned
                .as_mut()
                .expect("can_reproject checked runtime");
            let cached_len = runtime
                .content_len()
                .saturating_add(runtime.dock_line_count());
            let painted = runtime.reproject_frame(height, width);
            self.ao_reproject_frames = self.ao_reproject_frames.saturating_add(1);
            self.previous_viewport_top = 0;
            (cached_len, painted, true)
        } else {
            let mut new_lines = Vec::new();
            let dock_before = self
                .application_owned
                .as_ref()
                .map(|runtime| runtime.dock_rows);
            for comp in &mut self.components {
                new_lines.extend(comp.render(render_w));
                if let Some(dock) = comp.dock_rows_hint() {
                    self.dock_rows = dock.max(1);
                    if let Some(runtime) = self.application_owned.as_mut() {
                        runtime.set_dock_rows(self.dock_rows);
                    }
                }
            }
            if self.application_session_active && dock_before.is_some_and(|d| d != self.dock_rows) {
                self.previous_width = FORCE_SIZE_SENTINEL;
                self.previous_height = FORCE_SIZE_SENTINEL;
            }

            let component_lines = new_lines.len();

            if self.application_session_active {
                if let Some(runtime) = self.application_owned.as_mut() {
                    new_lines = runtime.project_frame(&new_lines, height, width);
                }
                self.previous_viewport_top = 0;
            }
            self.ao_components_stale = false;
            (component_lines, new_lines, false)
        };

        new_lines = self.apply_viewport_overlay(new_lines, width, height);

        let cursor_pos = self.extract_cursor_position(&mut new_lines, height);

        let ao_dock_rows = self
            .application_owned
            .as_ref()
            .map(ApplicationOwnedRuntime::dock_line_count)
            .unwrap_or(0)
            .min(new_lines.len());
        let ao_wheel_shift = (ao_reprojected && self.ao_wheel_render_requested)
            .then(|| self.detect_application_owned_vertical_shift(&new_lines, ao_dock_rows))
            .flatten();

        let reuse_ok = self.previous_width == width;
        let paint_rows = new_lines.len();
        for (i, line) in new_lines.iter_mut().enumerate() {
            if is_image_line(line) {
                continue;
            }
            if reuse_ok {
                let shifted_prev = ao_wheel_shift
                    .and_then(|shift| shifted_previous_index(i, paint_rows, ao_dock_rows, shift));
                let previous_index = shifted_prev.unwrap_or(i);
                if let Some(prev) = self.previous_lines.get(previous_index)
                    && previous_body(prev) == line.as_str()
                {
                    *line = prev.clone();
                    self.finalize_line_reuses = self.finalize_line_reuses.saturating_add(1);
                    continue;
                }
            }

            let mut finalized = normalize_terminal_output(line);
            finalized.push_str(SEGMENT_RESET);
            if !finalized.contains(CURSOR_MARKER) {
                self.finalize_width_checks = self.finalize_width_checks.saturating_add(1);
                let lw = visible_width(&finalized);
                if lw > width {
                    return Err(RenderError {
                        width,
                        line_width: lw,
                        line_index: i,
                        line_preview: finalized.chars().take(40).collect(),
                    });
                }
            }
            *line = finalized;
        }

        let width_changed = self.previous_width != 0 && self.previous_width != render_w;
        let height_changed = self.previous_height != 0 && self.previous_height != height;

        let (first_changed, last_changed, appended) = self.compute_line_diff(&new_lines);

        let deletions_need_full = first_changed >= new_lines.len() as isize
            && self.previous_lines.len() > new_lines.len()
            && {
                let target = new_lines.len().saturating_sub(1);
                let extra = self.previous_lines.len() - new_lines.len();
                target < self.previous_viewport_top || extra > height
            };

        let do_full = self.previous_lines.is_empty()
            || width_changed
            || height_changed
            || deletions_need_full
            || (self.clear_on_shrink
                && new_lines.len() < self.max_lines_rendered
                && self.overlays.is_empty())
            || (first_changed >= 0 && (first_changed as usize) < self.previous_viewport_top)
            || (first_changed >= new_lines.len() as isize
                && !new_lines.is_empty()
                && new_lines.len() <= self.previous_viewport_top);

        let ao_vertical_shifted = if do_full {
            let clear = !(self.previous_lines.is_empty() && !width_changed && !height_changed);
            self.full_render(&new_lines, clear, height);
            false
        } else if let Some(shift) = ao_wheel_shift {
            self.application_owned_vertical_shift_render(&new_lines, ao_dock_rows, shift);
            true
        } else if first_changed < 0 {
            false
        } else {
            self.differential_render(
                &new_lines,
                width,
                height,
                first_changed,
                last_changed,
                appended,
            );
            false
        };

        self.position_cursor(cursor_pos, new_lines.len());
        self.last_render_perf = RenderPerfSnap {
            do_render_us: render_started.elapsed().as_micros().min(u64::MAX as u128) as u64,
            component_lines,
            paint_lines: new_lines.len(),
            finalize_width_checks: self.finalize_width_checks.saturating_sub(checks_before),
            finalize_line_reuses: self.finalize_line_reuses.saturating_sub(reuses_before),
            ao_reprojected,
            ao_vertical_shifted,
        };
        self.previous_lines = new_lines;
        self.previous_width = render_w;
        self.previous_height = height;
        if self.application_session_active {
            self.previous_viewport_top = 0;
        }
        self.terminal.flush();
        if let Some(runtime) = self.application_owned.as_mut() {
            for seq in runtime.take_pending_clipboard() {
                self.terminal.write(&seq);
            }
            self.terminal.flush();
        }
        self.ao_wheel_render_requested = false;
        Ok(())
    }

    fn apply_viewport_overlay(
        &mut self,
        mut lines: Vec<String>,
        width: usize,
        height: usize,
    ) -> Vec<String> {
        if !self.overlays.is_empty() {
            lines = self.composite_overlays(lines, width, height);
        }
        if lines.is_empty() || height == 0 {
            return lines;
        }
        let viewport_start = lines.len().saturating_sub(height.min(lines.len()));
        let dock = if self.application_session_active {
            self.application_owned
                .as_ref()
                .map(ApplicationOwnedRuntime::dock_line_count)
                .unwrap_or(self.dock_rows)
        } else {
            self.dock_rows
        };
        {
            let visible = &mut lines[viewport_start..];
            let dock = dock.min(visible.len());
            let transcript_rows = visible.len().saturating_sub(dock);
            for comp in &self.components {
                comp.overlay_viewport(visible, width, transcript_rows);
            }
        }
        lines
    }

    fn full_render(&mut self, new_lines: &[String], clear: bool, height: usize) {
        self.full_redraw_count += 1;
        let mut buf = String::from(BEGIN_RENDER_BATCH);
        if clear {
            buf.push_str("\x1b[2J\x1b[H\x1b[3J");
        }
        for (i, line) in new_lines.iter().enumerate() {
            if i > 0 {
                buf.push_str("\r\n");
            }
            buf.push_str(line);
        }
        buf.push_str(END_RENDER_BATCH);
        self.terminal.write(&buf);
        let len = new_lines.len();
        self.cursor_row = len.saturating_sub(1);
        self.hardware_cursor_row = self.cursor_row;
        if clear {
            self.max_lines_rendered = len;
        } else {
            self.max_lines_rendered = self.max_lines_rendered.max(len);
        }
        self.previous_viewport_top = height.max(len).saturating_sub(height);
    }

    fn extract_cursor_position(
        &self,
        lines: &mut [String],
        height: usize,
    ) -> Option<(usize, usize)> {
        let vp = lines.len().saturating_sub(height);
        for row in (vp..lines.len()).rev() {
            if let Some(mi) = lines[row].find(CURSOR_MARKER) {
                let col = visible_width(&lines[row][..mi]);
                lines[row] = format!(
                    "{}{}",
                    &lines[row][..mi],
                    &lines[row][mi + CURSOR_MARKER.len()..]
                );
                return Some((row, col));
            }
        }
        None
    }

    fn position_cursor(&mut self, cursor_pos: Option<(usize, usize)>, total_lines: usize) {
        if cursor_pos.is_none() || total_lines == 0 {
            self.terminal.hide_cursor();
            return;
        }
        let (row, col) = cursor_pos.expect("checked above");
        let target_row = row.min(total_lines.saturating_sub(1));
        let target_col = col;

        let row_delta = target_row as isize - self.hardware_cursor_row as isize;
        let mut buf = String::new();
        if row_delta > 0 {
            buf.push_str(&format!("\x1b[{}B", row_delta));
        } else if row_delta < 0 {
            buf.push_str(&format!("\x1b[{}A", -row_delta));
        }
        buf.push_str(&format!("\x1b[{}G", target_col + 1));
        if !buf.is_empty() {
            self.terminal.write(&buf);
        }
        self.hardware_cursor_row = target_row;

        if self.show_hardware_cursor {
            self.terminal.show_cursor();
        } else {
            self.terminal.hide_cursor();
        }
    }
}
