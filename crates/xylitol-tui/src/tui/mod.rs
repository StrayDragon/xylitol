use crate::application_owned_runtime::ApplicationOwnedRuntime;
use crate::interaction_mode::InteractionMode;
use crate::pointer::{Hit, HitId, HitRegionList, PointerClickPhase, PointerPipeline, PointerView};
use crate::scroll_view::ScrollView;
use crate::selection::ScreenRect;
use crate::terminal::Terminal;
use crossterm::event::{KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

mod diff;
mod overlay;
mod render;

use overlay::{BlockedResume, OverlayFocusRestore, OverlayFocusRestorePolicy, OverlayStackEntry};
use render::{FORCE_SIZE_SENTINEL, MIN_RENDER_INTERVAL_MS};

pub use overlay::{
    FocusTarget, OverlayAnchor, OverlayHandle, OverlayMargin, OverlayOptions,
    OverlayUnfocusOptions, SizeValue,
};
pub use render::RenderPerfSnap;

pub const CURSOR_MARKER: &str = "\x1b_pi:c\x07";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Key(KeyEvent),
    Paste(String),
    Mouse(MouseEvent),
}

impl InputEvent {
    #[inline]
    pub fn is_pointer_motion(&self) -> bool {
        matches!(
            self,
            Self::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                ..
            })
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputReaction {
    #[default]
    None,
    Rerender,
}

impl InputReaction {
    #[inline]
    pub fn rerender_if(cond: bool) -> Self {
        if cond { Self::Rerender } else { Self::None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputListenerResult {
    Continue,
    Consumed,
    ConsumedRerender,
}

#[derive(Debug)]
pub struct RenderError {
    pub width: usize,
    pub line_width: usize,
    pub line_index: usize,
    pub line_preview: String,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {} is {} cols wide but terminal width is {}; use visible_width()/truncate_to_width(). preview: {:?}",
            self.line_index, self.line_width, self.width, self.line_preview
        )
    }
}

impl std::error::Error for RenderError {}

pub trait Component {
    fn render(&mut self, width: usize) -> Vec<String>;
    fn handle_input(&mut self, event: InputEvent);
    fn invalidate(&mut self);
    fn wants_key_release(&self) -> bool {
        false
    }
    fn tick(&mut self) -> bool {
        false
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        !matches!(event, InputEvent::Mouse(_))
    }

    fn take_pending_clipboard(&mut self) -> Vec<String> {
        Vec::new()
    }

    fn dock_rows_hint(&self) -> Option<usize> {
        None
    }

    fn wants_pointer_motion(&self) -> bool {
        false
    }

    fn clear_pointer_selection(&mut self) -> bool {
        false
    }

    fn set_screen_origin(&mut self, _col: u16, _row: u16) {}

    fn offer_pointer(&mut self, _event: &InputEvent) -> bool {
        false
    }

    fn overlay_viewport(&self, _lines: &mut [String], _width: usize, _transcript_rows: usize) {}
}

pub trait Focusable: Component {
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
}

type InputListenerFn = Box<dyn FnMut(InputEvent) -> InputListenerResult + 'static>;

struct RegisteredInputListener {
    id: u64,
    callback: InputListenerFn,
}

pub struct TUI<T: Terminal> {
    pub terminal: T,
    components: Vec<Box<dyn Component>>,
    overlays: Vec<(Box<dyn Component>, OverlayOptions, OverlayStackEntry)>,
    previous_lines: Vec<String>,
    previous_width: usize,
    previous_height: usize,
    previous_viewport_top: usize,
    focused_index: Option<usize>,
    focused_overlay_id: Option<u64>,
    overlay_focus_restore: OverlayFocusRestore,
    stopped: bool,
    cursor_row: usize,
    hardware_cursor_row: usize,
    show_hardware_cursor: bool,
    clear_on_shrink: bool,
    max_lines_rendered: usize,
    full_redraw_count: u64,
    frame_count: u64,
    finalize_width_checks: u64,
    finalize_line_reuses: u64,
    last_render_perf: RenderPerfSnap,
    ao_components_stale: bool,
    ao_wheel_render_requested: bool,
    ao_reproject_frames: u64,
    focus_order_counter: u64,
    next_overlay_id: u64,
    next_input_listener_id: u64,
    input_listeners: Vec<RegisteredInputListener>,
    #[allow(clippy::type_complexity)]
    after_dispatch_hook: Option<Box<dyn FnMut(&mut Self)>>,
    render_requested: bool,
    last_render_at: Option<std::time::Instant>,
    last_ao_wheel_render_at: Option<std::time::Instant>,
    interaction_mode: InteractionMode,
    application_session_active: bool,
    application_owned: Option<ApplicationOwnedRuntime>,
    dock_rows: usize,
    transcript_copy_on_release: bool,
    scroll_gutter: Option<crate::scroll_gutter::ScrollGutter>,
    extra_reserved_cols: u16,
    transcript_hit_priority: Option<crate::selection::HitPriorityFn>,
    pointer: PointerPipeline,
    append_session_to_main_scrollback_on_exit: bool,
}

impl<T: Terminal> TUI<T> {
    pub fn new(terminal: T) -> Self {
        Self {
            terminal,
            components: Vec::new(),
            overlays: Vec::new(),
            previous_lines: Vec::new(),
            previous_width: 0,
            previous_height: 0,
            previous_viewport_top: 0,
            focused_index: None,
            focused_overlay_id: None,
            overlay_focus_restore: OverlayFocusRestore::Inactive,
            stopped: false,
            cursor_row: 0,
            hardware_cursor_row: 0,
            show_hardware_cursor: false,
            clear_on_shrink: false,
            max_lines_rendered: 0,
            full_redraw_count: 0,
            frame_count: 0,
            finalize_width_checks: 0,
            finalize_line_reuses: 0,
            last_render_perf: RenderPerfSnap::default(),
            ao_components_stale: true,
            ao_wheel_render_requested: false,
            ao_reproject_frames: 0,
            focus_order_counter: 0,
            next_overlay_id: 1,
            next_input_listener_id: 1,
            input_listeners: Vec::new(),
            after_dispatch_hook: None,
            render_requested: false,
            last_render_at: None,
            last_ao_wheel_render_at: None,
            interaction_mode: InteractionMode::Inline,
            application_session_active: false,
            application_owned: None,
            dock_rows: 3,
            transcript_copy_on_release: true,
            scroll_gutter: None,
            extra_reserved_cols: 0,
            transcript_hit_priority: None,
            pointer: PointerPipeline::default(),
            append_session_to_main_scrollback_on_exit: true,
        }
    }

    pub fn with_interaction_mode(terminal: T, mode: InteractionMode) -> Self {
        let mut tui = Self::new(terminal);
        tui.interaction_mode = mode;
        tui
    }

    pub fn interaction_mode(&self) -> InteractionMode {
        self.interaction_mode
    }

    pub fn begin_application_owned_session(&mut self) {
        if !self.interaction_mode.is_application_owned() || self.application_session_active {
            return;
        }
        self.terminal.enter_alternate_screen();
        self.terminal.clear_screen();
        self.enable_mouse_capture();
        self.application_session_active = true;
        let mut runtime = ApplicationOwnedRuntime::new(self.dock_rows);
        runtime.set_copy_on_release(self.transcript_copy_on_release);
        runtime.set_scroll_gutter(self.scroll_gutter);
        runtime.set_extra_reserved_cols(self.extra_reserved_cols);
        self.application_owned = Some(runtime);
        self.ao_components_stale = true;
        self.last_ao_wheel_render_at = None;
        self.previous_width = FORCE_SIZE_SENTINEL;
        self.previous_height = FORCE_SIZE_SENTINEL;
        self.previous_lines.clear();
        self.previous_viewport_top = 0;
    }

    pub fn end_application_owned_session(&mut self) {
        if !self.application_session_active {
            return;
        }
        self.disable_mouse_capture();
        self.terminal.leave_alternate_screen();
        self.application_session_active = false;
        self.application_owned = None;
        self.last_ao_wheel_render_at = None;
        self.previous_lines.clear();
        self.previous_viewport_top = 0;
        self.previous_width = FORCE_SIZE_SENTINEL;
        self.previous_height = FORCE_SIZE_SENTINEL;
    }

    pub fn application_session_active(&self) -> bool {
        self.application_session_active
    }

    pub fn set_dock_rows(&mut self, rows: usize) {
        let rows = rows.max(1);
        if rows != self.dock_rows {
            self.ao_components_stale = true;
            self.clear_pointer_hover();
        }
        self.dock_rows = rows;
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.set_dock_rows(self.dock_rows);
        }
    }

    pub fn mark_ao_components_stale(&mut self) {
        self.ao_components_stale = true;
        self.ao_wheel_render_requested = false;
    }

    pub fn ao_reproject_frames_for_test(&self) -> u64 {
        self.ao_reproject_frames
    }

    pub fn clear_ao_reproject_frames_for_test(&mut self) {
        self.ao_reproject_frames = 0;
    }

    pub fn ao_components_stale(&self) -> bool {
        self.ao_components_stale
    }

    pub fn application_owned_scroll_by(&mut self, delta: isize) -> bool {
        let Some(runtime) = self.application_owned.as_mut() else {
            return false;
        };
        if !runtime.ingest_wheel_delta(delta) {
            return false;
        }
        self.ao_wheel_render_requested = true;
        self.request_render(false);
        true
    }

    pub fn application_owned_scroll_to_end(&mut self) -> bool {
        let Some(runtime) = self.application_owned.as_mut() else {
            return false;
        };
        let changed = runtime.scroll_to_end();
        if changed {
            self.request_render(false);
        }
        changed
    }

    pub fn application_owned_scroll_home(&mut self) -> bool {
        let Some(runtime) = self.application_owned.as_mut() else {
            return false;
        };
        let changed = runtime.scroll_home();
        if changed {
            self.request_render(false);
        }
        changed
    }

    pub fn application_owned_wheel_notch(&self) -> isize {
        self.application_owned
            .as_ref()
            .map(ApplicationOwnedRuntime::wheel_notch)
            .unwrap_or(ScrollView::WHEEL_NOTCH)
    }

    pub fn application_owned_scroll_top(&self) -> usize {
        self.application_owned
            .as_ref()
            .map(ApplicationOwnedRuntime::scroll_top)
            .unwrap_or(0)
    }

    pub fn dock_rows(&self) -> usize {
        self.dock_rows
    }

    pub fn last_painted_lines(&self) -> &[String] {
        &self.previous_lines
    }

    pub fn application_owned_content_lines(&self) -> Option<&[String]> {
        self.application_owned
            .as_ref()
            .map(ApplicationOwnedRuntime::content_lines)
    }

    pub fn set_transcript_hit_priority(&mut self, hit: Option<crate::selection::HitPriorityFn>) {
        self.transcript_hit_priority = hit;
    }

    pub fn set_hit_regions(&mut self, regions: HitRegionList) {
        self.pointer.set_regions(regions);
    }

    pub fn hit_regions(&self) -> &HitRegionList {
        self.pointer.regions()
    }

    pub fn hit_regions_mut(&mut self) -> &mut HitRegionList {
        self.pointer.regions_mut()
    }

    pub fn clear_hits(&mut self) {
        self.pointer.regions_mut().clear();
    }

    pub fn mark_hit(&mut self, hit: Hit) {
        self.pointer.regions_mut().mark(hit);
    }

    pub fn replace_hits(&mut self, hits: impl IntoIterator<Item = Hit>) {
        self.clear_hits();
        for hit in hits {
            self.mark_hit(hit);
        }
    }

    pub fn replace_hover_hits(&mut self, hits: impl IntoIterator<Item = Hit>) {
        let mut list = HitRegionList::new();
        for hit in hits {
            list.mark(hit);
        }
        self.pointer.set_hover_paint(list);
    }

    pub fn set_pointer_click_phase(&mut self, phase: PointerClickPhase) {
        self.pointer.set_phase(phase);
    }

    pub fn pointer_click_phase(&self) -> PointerClickPhase {
        self.pointer.phase()
    }

    pub fn arm_pointer_probe(&mut self, armed: bool) {
        self.pointer.arm_probe(armed);
    }

    pub fn pointer_probe_armed(&self) -> bool {
        self.pointer.probe_armed()
    }

    pub fn pointer_hover_id(&self) -> Option<HitId> {
        self.pointer.hover_id()
    }

    pub fn take_pointer_click(&mut self) -> Option<HitId> {
        self.pointer.take_click()
    }

    pub fn hit_id_at(&self, col: u16, row: u16) -> Option<HitId> {
        self.pointer.hit_at(col, row, self.pointer_view(false))
    }

    pub fn set_append_session_to_main_scrollback_on_exit(&mut self, on: bool) {
        self.append_session_to_main_scrollback_on_exit = on;
    }

    pub fn append_session_to_main_scrollback_on_exit(&self) -> bool {
        self.append_session_to_main_scrollback_on_exit
    }

    pub fn take_copy_notice(&mut self) -> bool {
        self.application_owned
            .as_mut()
            .is_some_and(|runtime| runtime.take_copy_notice())
    }

    pub fn enqueue_clipboard_sequences(&mut self, seqs: impl IntoIterator<Item = String>) {
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.enqueue_clipboard_seqs(seqs);
        }
    }

    pub fn set_scroll_gutter(&mut self, gutter: Option<crate::scroll_gutter::ScrollGutter>) {
        self.scroll_gutter = gutter;
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.set_scroll_gutter(gutter);
        }
    }

    pub fn set_extra_reserved_cols(&mut self, n: u16) {
        self.extra_reserved_cols = n;
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.set_extra_reserved_cols(n);
        }
    }

    pub fn scroll_gutter_hit_set(&self) -> Vec<crate::pointer::Hit> {
        use crate::scroll_gutter::{gutter_active, gutter_hit_set};
        let Some(runtime) = self.application_owned.as_ref() else {
            return Vec::new();
        };
        if self.scroll_gutter.is_none() {
            return Vec::new();
        }
        let cols = self.terminal.columns();
        let rows = self.terminal.rows() as usize;
        let transcript_rows = rows.saturating_sub(self.dock_rows.min(rows));
        let boundary_cols = cols.saturating_sub(runtime.extra_reserved_cols());
        let active = gutter_active(
            runtime.content_len(),
            transcript_rows,
            usize::from(boundary_cols),
        );
        gutter_hit_set(boundary_cols, transcript_rows as u16, active)
    }

    pub fn copy_notice_active(&self) -> bool {
        self.application_owned
            .as_ref()
            .is_some_and(|runtime| runtime.copy_notice_active())
    }

    pub fn application_owned_has_selection(&self) -> bool {
        self.application_owned
            .as_ref()
            .is_some_and(|runtime| runtime.has_selection())
    }

    pub fn application_owned_is_dragging(&self) -> bool {
        self.application_owned
            .as_ref()
            .is_some_and(|runtime| runtime.selection.is_dragging())
    }

    fn capturing_overlay_contains(&self, col: u16, row: u16) -> bool {
        self.overlays.iter().any(|(_comp, opts, entry)| {
            !opts.non_capturing
                && self.is_overlay_entry_visible(entry)
                && entry
                    .last_screen_rect
                    .is_some_and(|rect: ScreenRect| rect.contains(col, row))
        })
    }

    fn pointer_view(&self, overlay_capturing: bool) -> PointerView {
        let rows = self.terminal.rows();
        let transcript_rows = rows.saturating_sub(self.dock_rows.min(rows as usize) as u16);
        PointerView {
            scroll_top: self.application_owned_scroll_top(),
            transcript_rows,
            overlay_capturing,
        }
    }

    fn refresh_pointer_hover_for_paint(&mut self, can_reproject: bool) {
        if !self.application_session_active {
            return;
        }
        if self.hover_reprobe_allowed(can_reproject) {
            self.pointer.reprobe_hover(self.pointer_view(false));
        } else {
            self.pointer.clear_hover();
        }
        self.sync_pointer_hover_into_runtime();
    }

    fn hover_reprobe_allowed(&self, can_reproject: bool) -> bool {
        can_reproject
            || (self.overlays.is_empty()
                && self.previous_width == self.ao_render_width(self.terminal.columns() as usize))
    }

    pub fn clear_pointer_hover(&mut self) {
        self.pointer.forget_hover();
        self.sync_pointer_hover_into_runtime();
    }

    pub fn reprobe_pointer_hover(&mut self) {
        self.pointer.reprobe_hover(self.pointer_view(false));
        self.sync_pointer_hover_into_runtime();
    }

    fn sync_pointer_hover_into_runtime(&mut self) {
        let hover_id = self.pointer.hover_id();
        let regions = self.pointer.hover_paint().clone();
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.set_hover(hover_id, regions);
        }
    }

    pub fn add_input_listener(
        &mut self,
        listener: impl FnMut(InputEvent) -> InputListenerResult + 'static,
    ) -> u64 {
        let id = self.next_input_listener_id;
        self.next_input_listener_id = self.next_input_listener_id.saturating_add(1);
        self.input_listeners.push(RegisteredInputListener {
            id,
            callback: Box::new(listener),
        });
        id
    }

    pub fn remove_input_listener(&mut self, id: u64) {
        self.input_listeners.retain(|l| l.id != id);
    }

    pub fn full_redraws(&self) -> u64 {
        self.full_redraw_count
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn stop_requested(&self) -> bool {
        self.stopped
    }

    pub fn finalize_width_checks_for_test(&self) -> u64 {
        self.finalize_width_checks
    }

    pub fn finalize_line_reuses_for_test(&self) -> u64 {
        self.finalize_line_reuses
    }

    pub fn clear_finalize_counters_for_test(&mut self) {
        self.finalize_width_checks = 0;
        self.finalize_line_reuses = 0;
    }

    pub fn last_render_perf(&self) -> RenderPerfSnap {
        self.last_render_perf
    }

    pub fn is_render_requested(&self) -> bool {
        self.render_requested
    }

    pub fn application_owned_wheel_render_deadline(&self) -> Option<std::time::Instant> {
        if !self.render_requested
            || !self.ao_wheel_render_requested
            || !self.can_reproject_application_owned(
                self.ao_render_width(self.terminal.columns() as usize),
            )
        {
            return None;
        }
        Some(
            self.last_ao_wheel_render_at
                .map_or_else(std::time::Instant::now, |last| {
                    last + std::time::Duration::from_millis(MIN_RENDER_INTERVAL_MS)
                }),
        )
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }
    pub fn remove_child(&mut self, index: usize) {
        if index < self.components.len() {
            self.components.remove(index);
        }
    }
    pub fn clear_children(&mut self) {
        self.components.clear();
    }
    pub fn set_focus(&mut self, index: Option<usize>) {
        self.set_focus_internal(
            index.map(FocusTarget::Root),
            OverlayFocusRestorePolicy::Clear,
        );
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start_impl(None)
    }

    pub fn start_with_flag(
        &mut self,
        quit_flag: &Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_impl(Some(quit_flag))
    }

    fn start_impl(
        &mut self,
        quit_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crossterm::event::{self, Event};

        self.stopped = false;
        self.terminal.hide_cursor();
        self.terminal.start();
        if self.interaction_mode.is_application_owned() {
            self.begin_application_owned_session();
        }
        self.do_render()?;

        while !self.stopped {
            if let Some(flag) = quit_flag
                && flag.load(std::sync::atomic::Ordering::SeqCst)
            {
                break;
            }
            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if !self.should_dispatch_key_event(&key_event) {
                            continue;
                        }
                        let reaction = self.dispatch_event(InputEvent::Key(key_event));
                        if reaction == InputReaction::Rerender {
                            self.run_after_dispatch_hook();
                            self.do_render()?;
                        }
                    }
                    Event::Resize(_, _) => {
                        self.terminal.refresh_size();
                        self.do_render()?;
                    }
                    Event::Paste(data) => {
                        let reaction = self.dispatch_event(InputEvent::Paste(data));
                        if reaction == InputReaction::Rerender {
                            self.run_after_dispatch_hook();
                            self.do_render()?;
                        }
                    }
                    Event::Mouse(mouse) => {
                        let event = InputEvent::Mouse(mouse);
                        if event.is_pointer_motion() && !self.application_session_active {
                            continue;
                        }
                        let reaction = self.dispatch_event(event);
                        if reaction == InputReaction::Rerender {
                            self.request_render(false);
                        }
                        if self.idle_tick() {
                            self.request_render(false);
                        }
                        if self.render_requested {
                            self.run_after_dispatch_hook();
                            let _ = self.try_render()?;
                        }
                    }
                    _ => {}
                }
            } else {
                let wants_render = self.idle_tick();
                if wants_render {
                    self.do_render()?;
                }
            }
        }

        self.finish();
        Ok(())
    }

    pub fn finish(&mut self) {
        if self.application_session_active || self.interaction_mode.is_application_owned() {
            self.finish_application_owned();
        } else {
            self.finish_inline();
        }
    }

    pub fn finish_inline(&mut self) {
        self.stopped = true;
        if !self.previous_lines.is_empty() {
            let target_row = self.previous_lines.len();
            if target_row > self.hardware_cursor_row {
                self.terminal
                    .write(&format!("\x1b[{}B", target_row - self.hardware_cursor_row));
            }
            self.terminal.write("\r\n");
        }
        self.terminal.flush();
        self.terminal.stop();
    }

    pub fn finish_application_owned(&mut self) {
        self.stopped = true;
        let scrollback_lines = if self.append_session_to_main_scrollback_on_exit {
            self.application_owned
                .as_ref()
                .map(|runtime| runtime.session_lines_for_main_scrollback())
                .filter(|lines| !lines.is_empty())
                .unwrap_or_else(|| self.previous_lines.clone())
        } else {
            Vec::new()
        };
        let pending_clipboard = self
            .application_owned
            .as_mut()
            .map(ApplicationOwnedRuntime::take_pending_clipboard)
            .unwrap_or_default();
        self.end_application_owned_session();
        if self.append_session_to_main_scrollback_on_exit {
            for line in &scrollback_lines {
                self.terminal.write(line);
                self.terminal.write("\r\n");
            }
        }
        for sequence in pending_clipboard {
            self.terminal.write(&sequence);
        }
        self.terminal.flush();
        self.terminal.stop();
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }

    pub fn with_terminal_suspended<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let restore_application_owned = self.application_session_active;
        self.terminal.stop();
        if restore_application_owned {
            self.application_session_active = true;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        self.terminal.hide_cursor();
        self.terminal.start();
        if restore_application_owned {
            self.terminal.enter_alternate_screen();
            self.terminal.clear_screen();
            self.enable_mouse_capture();
            if self.application_owned.is_none() {
                let mut runtime = ApplicationOwnedRuntime::new(self.dock_rows);
                runtime.set_copy_on_release(self.transcript_copy_on_release);
                runtime.set_scroll_gutter(self.scroll_gutter);
                runtime.set_extra_reserved_cols(self.extra_reserved_cols);
                self.application_owned.replace(runtime);
            }
            self.previous_width = FORCE_SIZE_SENTINEL;
            self.previous_height = FORCE_SIZE_SENTINEL;
            self.previous_lines.clear();
            self.previous_viewport_top = 0;
        }
        self.terminal.refresh_size();
        self.request_render(restore_application_owned);
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub fn set_after_dispatch_hook(&mut self, hook: impl FnMut(&mut Self) + 'static) {
        self.after_dispatch_hook = Some(Box::new(hook));
    }

    fn run_after_dispatch_hook(&mut self) {
        let mut hook = self.after_dispatch_hook.take();
        if let Some(ref mut h) = hook {
            h(self);
        }
        self.after_dispatch_hook = hook;
    }

    fn offer_pointer_tree(&mut self, event: &InputEvent) -> Option<bool> {
        if !matches!(event, InputEvent::Mouse(_)) {
            return None;
        }
        for i in (0..self.overlays.len()).rev() {
            if self.overlays[i].2.hidden {
                continue;
            }
            if self.overlays[i].0.offer_pointer(event) {
                return Some(self.overlays[i].0.input_wants_rerender(event));
            }
        }
        for i in 0..self.components.len() {
            if self.components[i].offer_pointer(event) {
                return Some(self.components[i].input_wants_rerender(event));
            }
        }
        None
    }

    pub fn dispatch_event(&mut self, event: InputEvent) -> InputReaction {
        if matches!(event, InputEvent::Key(_) | InputEvent::Paste(_)) {
            self.ao_components_stale = true;
        }
        let mut force_rerender = false;
        if self.application_session_active
            && let InputEvent::Mouse(mouse) = &event
        {
            let is_wheel = matches!(
                mouse.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            );
            if matches!(
                mouse.kind,
                MouseEventKind::Down(crossterm::event::MouseButton::Left)
            ) {
                force_rerender |= self.clear_focused_pointer_selection();
            }
            let cols = self.terminal.columns();
            let rows = self.terminal.rows();
            let transcript_rows = rows.saturating_sub(self.dock_rows.min(rows as usize) as u16);
            let wheel_in_transcript = is_wheel && mouse.row < transcript_rows;
            let overlay_capturing = self.capturing_overlay_contains(mouse.column, mouse.row);
            let outcome = self
                .pointer
                .on_mouse(mouse, self.pointer_view(overlay_capturing));
            let hit = self.transcript_hit_priority.take();
            let dirty = if let Some(runtime) = self.application_owned.as_mut() {
                runtime.selection.set_hit_priority(hit);
                let dirty = if outcome.skip_selection {
                    runtime.clear_selection();
                    false
                } else {
                    runtime.handle_mouse(mouse, cols, rows)
                };
                self.transcript_hit_priority = runtime.selection.take_hit_priority();
                dirty
            } else {
                self.transcript_hit_priority = hit;
                false
            };
            if outcome.hover_changed {
                self.ao_wheel_render_requested = true;
                self.sync_pointer_hover_into_runtime();
                return InputReaction::Rerender;
            }
            if force_rerender {
                self.ao_components_stale = true;
            }
            if dirty || (outcome.skip_selection && outcome.consume) {
                if is_wheel {
                    self.ao_wheel_render_requested = true;
                }
                return InputReaction::Rerender;
            }
            if wheel_in_transcript {
                return InputReaction::None;
            }
            if matches!(mouse.kind, MouseEventKind::Moved) {
                let wants_motion = self.focused_index.is_some_and(|idx| {
                    self.components
                        .get(idx)
                        .is_some_and(|c| c.wants_pointer_motion())
                }) || self.focused_overlay_id.is_some_and(|oid| {
                    self.overlay_index(oid)
                        .is_some_and(|i| self.overlays[i].0.wants_pointer_motion())
                });
                if !wants_motion {
                    return InputReaction::rerender_if(force_rerender);
                }
            }
        }

        let listener_ids: Vec<u64> = self.input_listeners.iter().map(|l| l.id).collect();
        for id in listener_ids {
            let Some(index) = self.input_listeners.iter().position(|l| l.id == id) else {
                continue;
            };
            let result = (self.input_listeners[index].callback)(event.clone());
            match result {
                InputListenerResult::Continue => {}
                InputListenerResult::ConsumedRerender => {
                    self.ao_components_stale = true;
                    return InputReaction::Rerender;
                }
                InputListenerResult::Consumed => {
                    let paint = force_rerender || !matches!(event, InputEvent::Mouse(_));
                    if paint {
                        self.ao_components_stale = true;
                    }
                    return InputReaction::rerender_if(paint);
                }
            }
        }

        if let Some(overlay_id) = self.focused_overlay_id
            && let Some(index) = self.overlay_index(overlay_id)
            && !self.is_overlay_entry_visible(&self.overlays[index].2)
        {
            let pre_focus = self.overlays[index].2.pre_focus;
            if let Some(top) = self.topmost_visible_capturing_overlay_id() {
                self.set_focus_internal(
                    Some(FocusTarget::Overlay(top)),
                    OverlayFocusRestorePolicy::Clear,
                );
            } else {
                self.set_focus_internal(pre_focus, OverlayFocusRestorePolicy::Preserve);
            }
        }

        let focus_is_overlay = matches!(self.current_focus_target(), Some(FocusTarget::Overlay(_)));
        if !focus_is_overlay {
            match self.get_visible_overlay_focus_restore() {
                OverlayFocusRestore::Eligible { overlay_id } => {
                    self.set_focus_internal(
                        Some(FocusTarget::Overlay(overlay_id)),
                        OverlayFocusRestorePolicy::Clear,
                    );
                }
                OverlayFocusRestore::Blocked {
                    overlay_id,
                    blocked_by,
                    resume,
                } if self.current_focus_target() != Some(blocked_by) => match resume {
                    BlockedResume::RestoreOverlay => {
                        self.set_focus_internal(
                            Some(FocusTarget::Overlay(overlay_id)),
                            OverlayFocusRestorePolicy::Clear,
                        );
                    }
                    BlockedResume::FocusTarget(target) => {
                        self.clear_overlay_focus_restore();
                        self.set_focus_internal(target, OverlayFocusRestorePolicy::Clear);
                    }
                },
                _ => {}
            }
        }

        if matches!(event, InputEvent::Mouse(_))
            && let Some(wants) = self.offer_pointer_tree(&event)
        {
            let paint = wants || force_rerender;
            if paint {
                self.ao_components_stale = true;
            }
            return InputReaction::rerender_if(paint);
        }

        if let Some(overlay_id) = self.focused_overlay_id
            && let Some(index) = self.overlay_index(overlay_id)
            && !self.overlays[index].2.hidden
        {
            self.overlays[index].0.handle_input(event.clone());
            let wants = self.overlays[index].0.input_wants_rerender(&event);
            let seqs = self.overlays[index].0.take_pending_clipboard();
            self.ingest_component_clipboard(seqs);
            let paint = wants || force_rerender;
            if paint {
                self.ao_components_stale = true;
            }
            return InputReaction::rerender_if(paint);
        }
        if let Some(idx) = self.focused_index
            && idx < self.components.len()
        {
            self.components[idx].handle_input(event.clone());
            let wants = self.components[idx].input_wants_rerender(&event);
            let seqs = self.components[idx].take_pending_clipboard();
            self.ingest_component_clipboard(seqs);
            let paint = wants || force_rerender;
            if paint {
                self.ao_components_stale = true;
            }
            return InputReaction::rerender_if(paint);
        }
        if force_rerender {
            self.ao_components_stale = true;
        }
        InputReaction::rerender_if(force_rerender)
    }

    fn clear_focused_pointer_selection(&mut self) -> bool {
        if let Some(overlay_id) = self.focused_overlay_id
            && let Some(index) = self.overlay_index(overlay_id)
        {
            return self.overlays[index].0.clear_pointer_selection();
        }
        if let Some(idx) = self.focused_index
            && let Some(component) = self.components.get_mut(idx)
        {
            return component.clear_pointer_selection();
        }
        false
    }

    fn ingest_component_clipboard(&mut self, seqs: Vec<String>) {
        if seqs.is_empty() {
            return;
        }
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.enqueue_clipboard_seqs(seqs);
        }
    }

    pub fn enable_mouse_capture(&mut self) {
        self.terminal.enable_mouse_capture();
    }

    pub fn disable_mouse_capture(&mut self) {
        self.terminal.disable_mouse_capture();
    }

    pub fn mouse_capture_enabled(&self) -> bool {
        self.terminal.mouse_capture_active()
    }

    pub fn idle_tick(&mut self) -> bool {
        let mut changed = self.components.iter_mut().any(|c| c.tick());
        changed |= self
            .overlays
            .iter_mut()
            .any(|(component, _, _)| component.tick());
        if self.application_session_active {
            let cols = self.terminal.columns();
            let rows = self.terminal.rows();
            if let Some(runtime) = self.application_owned.as_mut() {
                changed |= runtime.tick_autoscroll(cols, rows);
                if runtime.tick_copy_notice() {
                    changed = true;
                }
            }
        }
        changed
    }

    fn should_dispatch_key_event(&self, key: &KeyEvent) -> bool {
        match key.kind {
            KeyEventKind::Press | KeyEventKind::Repeat => true,
            KeyEventKind::Release => {
                if let Some(overlay_id) = self.focused_overlay_id
                    && let Some(index) = self.overlay_index(overlay_id)
                {
                    return self.overlays[index].0.wants_key_release();
                }
                self.focused_index
                    .and_then(|idx| self.components.get(idx))
                    .is_some_and(|component| component.wants_key_release())
            }
        }
    }

    pub fn invalidate(&mut self) {
        for c in &mut self.components {
            c.invalidate();
        }
        for (c, _, _) in &mut self.overlays {
            c.invalidate();
        }
    }
}

#[cfg(test)]
mod tests;
