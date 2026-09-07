use crate::application_owned_runtime::ApplicationOwnedRuntime;
use crate::interaction_mode::InteractionMode;
use crate::scroll_view::ScrollView;
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

/// Decoded input delivered to components. Runtime path is crossterm-native —
/// no KeyEvent→VT re-encode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Key(KeyEvent),
    Paste(String),
    /// Pointer / wheel report. Prefer filtering [`Self::is_pointer_motion`] at
    /// fan-in so crossterm `1003` any-event floods never reach the host paint path.
    Mouse(MouseEvent),
}

impl InputEvent {
    /// `MouseEventKind::Moved` only — safe to drop at the edge (no presentation change).
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

/// Whether routing an [`InputEvent`] should schedule a frame.
///
/// Keys/paste default to [`Self::Rerender`] (today's host always-paint). Mouse
/// defaults to [`Self::None`] until a component opts in via
/// [`Component::input_wants_rerender`] (e.g. future fold-hit handlers).
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

/// Result of an input listener invoked before focus routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputListenerResult {
    /// Pass the event to later listeners and (if none consume) the focused target.
    Continue,
    /// Stop the listener chain and do not deliver to overlay/focus.
    ///
    /// Paint policy: Key/Paste → [`InputReaction::Rerender`]; Mouse →
    /// [`InputReaction::None`]. Mouse handlers that dirtied UI MUST return
    /// [`Self::ConsumedRerender`] (c2040 fold click).
    Consumed,
    /// Stop routing and schedule a frame (intentional presentation change).
    ConsumedRerender,
}

/// Error from a render pass. The engine hard-errors when a component emits a
/// line wider than the terminal `width` — a widget that overflows desyncs the
/// cursor and corrupts subsequent lines, so we surface it loudly instead of
/// silently truncating (mirrors pi-tui's crash guard in the diff path).
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
    /// Advance any time-driven state (e.g. a spinner frame). Called by the
    /// event loop on its tick. Default no-op so non-animated components ignore it.
    /// Returning true hints that a re-render is wanted (host decides).
    fn tick(&mut self) -> bool {
        false
    }

    /// Whether handling `event` should request a frame after [`Self::handle_input`].
    ///
    /// Default: Key/Paste → yes; Mouse → no (ignored pointer traffic must not
    /// thrash the differential engine). Override when a component actually
    /// mutates presentation for a mouse event (click targets, etc.).
    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        !matches!(event, InputEvent::Mouse(_))
    }

    /// Drain OSC52 (or other clipboard) sequences produced by this component
    /// since the last take (e.g. Editor ApplicationOwned copy-on-release). Default empty.
    fn take_pending_clipboard(&mut self) -> Vec<String> {
        Vec::new()
    }

    /// ApplicationOwned: after `render`, hint how many trailing rows are dock (status +
    /// editor + footer). TUI applies this **before** `project_frame` so a grown
    /// editor does not leave dock content misclassified as transcript.
    fn dock_rows_hint(&self) -> Option<usize> {
        None
    }

    /// True while this component needs bare `MouseEventKind::Moved` (e.g. active
    /// editor selection drag). ApplicationOwned otherwise drops non-dirty Moved.
    fn wants_pointer_motion(&self) -> bool {
        false
    }

    /// Clear any pointer-driven text selection this component owns.
    ///
    /// ApplicationOwned: TUI calls this on left-button Down so transcript and
    /// editor selections stay mutually exclusive (click anywhere cancels the
    /// other surface's highlight). Returns whether a selection was cleared
    /// (caller may force a frame when the Down does not otherwise dirty).
    fn clear_pointer_selection(&mut self) -> bool {
        false
    }
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
    /// Last painted terminal width. `0` = first-frame sentinel (no clear).
    /// [`FORCE_SIZE_SENTINEL`] = pi `previousWidth = -1` (force clearing redraw).
    previous_width: usize,
    /// Last painted terminal height. Same sentinels as [`Self::previous_width`].
    previous_height: usize,
    /// Row index (in the rendered line buffer) of the top of the visible
    /// viewport at the end of the previous frame. Differential rendering uses
    /// this to translate buffer rows to screen rows when content exceeds one
    /// screen. Mirrors pi's `previousViewportTop`.
    previous_viewport_top: usize,
    focused_index: Option<usize>,
    /// When set, input routes to this overlay instead of a root child.
    focused_overlay_id: Option<u64>,
    /// pi-style eligible / blocked / resume (c575 / D08).
    overlay_focus_restore: OverlayFocusRestore,
    stopped: bool,
    /// Logical cursor row = content end (used for viewport math). Distinct from
    /// `hardware_cursor_row` (where the terminal's cursor physically stopped).
    cursor_row: usize,
    hardware_cursor_row: usize,
    show_hardware_cursor: bool,
    clear_on_shrink: bool,
    max_lines_rendered: usize,
    full_redraw_count: u64,
    /// Increments on every successful `do_render` (throttle / skip 不计入).
    frame_count: u64,
    /// Obs/test: lines that ran normalize + `visible_width` in finalize.
    finalize_width_checks: u64,
    /// Obs/test: lines that reused the previous finalized string (A+B fast path).
    finalize_line_reuses: u64,
    /// Last painted frame cost (obs / labs).
    last_render_perf: RenderPerfSnap,
    /// ApplicationOwned: when false, wheel/selection frames may skip
    /// `Component::render` and only reproject the cached transcript+dock.
    ao_components_stale: bool,
    /// A wheel-only AO viewport change is pending. `try_render` may paint this
    /// cached reproject immediately instead of waiting behind the full-frame
    /// throttle.
    ao_wheel_render_requested: bool,
    /// Obs: how many frames took the ApplicationOwned reproject-only path.
    ao_reproject_frames: u64,
    focus_order_counter: u64,
    next_overlay_id: u64,
    next_input_listener_id: u64,
    input_listeners: Vec<RegisteredInputListener>,
    /// Optional hook after each input dispatch inside `start` / `start_with_flag`
    /// (e.g. demo external `$EDITOR` via [`Self::with_terminal_suspended`]).
    #[allow(clippy::type_complexity)]
    after_dispatch_hook: Option<Box<dyn FnMut(&mut Self)>>,
    // ── render scheduling (pi's requestRender/scheduleRender) ──
    /// True when a render has been requested but not yet executed.
    render_requested: bool,
    /// Monotonic instant of the last actual render, for the 16ms throttle.
    last_render_at: Option<std::time::Instant>,
    /// Separate wheel cadence: the first cached AO reproject is independent of
    /// the previous full frame; continuous wheel paints remain capped near 60fps.
    last_ao_wheel_render_at: Option<std::time::Instant>,
    /// Dual interaction mode (c2070). Default [`InteractionMode::Inline`].
    interaction_mode: InteractionMode,
    /// ApplicationOwned session currently holding alt-buffer + mouse (after
    /// [`Self::begin_application_owned_session`]).
    application_session_active: bool,
    /// ApplicationOwned viewport/selection runtime (only while application session active).
    application_owned: Option<ApplicationOwnedRuntime>,
    /// Default dock rows used when beginning an ApplicationOwned session.
    dock_rows: usize,
    /// Persists across begin/end; applied when constructing [`ApplicationOwnedRuntime`].
    transcript_copy_on_release: bool,
    /// Transcript Left-Down priority hook (fold hit). Parked on `TUI` so it
    /// survives ApplicationOwnedRuntime recreate (suspend / begin); swapped
    /// onto the live selection controller for each mouse dispatch.
    transcript_hit_priority: Option<crate::selection::HitPriorityFn>,
    /// When true (default), [`Self::finish_application_owned`] appends the
    /// session transcript (+ last dock) to the **main-screen** terminal
    /// scrollback after leaving the alternate buffer — so the user can still
    /// scroll the session after ApplicationOwned exits.
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
            transcript_hit_priority: None,
            append_session_to_main_scrollback_on_exit: true,
        }
    }

    /// Construct with an explicit interaction mode (c2070). Switching modes at
    /// runtime MUST tear down and rebuild the stack (`end_*` then `begin_*`);
    /// do not mix emulator-owned and application-owned selection on one stack.
    pub fn with_interaction_mode(terminal: T, mode: InteractionMode) -> Self {
        let mut tui = Self::new(terminal);
        tui.interaction_mode = mode;
        tui
    }

    pub fn interaction_mode(&self) -> InteractionMode {
        self.interaction_mode
    }

    /// ApplicationOwned enter: alternate screen (SHOULD) + mouse capture + viewport runtime.
    /// Idempotent. No-op when mode is [`InteractionMode::Inline`].
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
        self.application_owned = Some(runtime);
        self.ao_components_stale = true;
        self.last_ao_wheel_render_at = None;
        // Force a clearing redraw into the alt buffer.
        self.previous_width = FORCE_SIZE_SENTINEL;
        self.previous_height = FORCE_SIZE_SENTINEL;
        self.previous_lines.clear();
        self.previous_viewport_top = 0;
    }

    /// ApplicationOwned leave: disable mouse + leave alt-buffer. Safe if never begun.
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

    /// Rows reserved at the bottom for dock (editor/status/footer). ApplicationOwned
    /// selection excludes this band.
    pub fn set_dock_rows(&mut self, rows: usize) {
        let rows = rows.max(1);
        if rows != self.dock_rows {
            self.ao_components_stale = true;
        }
        self.dock_rows = rows;
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.set_dock_rows(self.dock_rows);
        }
    }

    /// Force the next ApplicationOwned frame to re-run `Component::render`
    /// (entries / chrome / editor changed). Wheel/selection-only paints leave
    /// this clear so they can reproject the cached transcript.
    pub fn mark_ao_components_stale(&mut self) {
        self.ao_components_stale = true;
        self.ao_wheel_render_requested = false;
    }

    /// Obs/test: ApplicationOwned frames that skipped component render.
    pub fn ao_reproject_frames_for_test(&self) -> u64 {
        self.ao_reproject_frames
    }

    pub fn clear_ao_reproject_frames_for_test(&mut self) {
        self.ao_reproject_frames = 0;
    }

    /// Whether the next ApplicationOwned frame must re-run `Component::render`.
    pub fn ao_components_stale(&self) -> bool {
        self.ao_components_stale
    }

    /// Apply a coalesced ApplicationOwned wheel delta immediately and request
    /// a cheap cached reproject paint.
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

    /// Move an active ApplicationOwned viewport to the transcript end and
    /// re-arm follow mode. Returns whether the viewport position changed.
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

    /// Three-row wheel notch for AO host coalescing; edge-drag stays independent.
    pub fn application_owned_wheel_notch(&self) -> isize {
        self.application_owned
            .as_ref()
            .map(ApplicationOwnedRuntime::wheel_notch)
            .unwrap_or(ScrollView::WHEEL_NOTCH)
    }

    /// ApplicationOwned transcript viewport scroll top (content rows above the pane).
    pub fn application_owned_scroll_top(&self) -> usize {
        self.application_owned
            .as_ref()
            .map(ApplicationOwnedRuntime::scroll_top)
            .unwrap_or(0)
    }

    pub fn dock_rows(&self) -> usize {
        self.dock_rows
    }

    /// Transcript Left-Down priority hook (c2040 fold triangle, etc.).
    ///
    /// When the callback returns true, the press is swallowed: transcript
    /// selection is cleared and the event does **not** start a drag. Stored on
    /// `TUI` so it survives ApplicationOwnedRuntime recreate (suspend / begin).
    pub fn set_transcript_hit_priority(&mut self, hit: Option<crate::selection::HitPriorityFn>) {
        self.transcript_hit_priority = hit;
    }

    /// After leaving alt-screen, append the session transcript (+ last dock)
    /// into the terminal's **main** scrollback so history remains readable
    /// (default **on**). Set `false` to tear down without writing those lines.
    pub fn set_append_session_to_main_scrollback_on_exit(&mut self, on: bool) {
        self.append_session_to_main_scrollback_on_exit = on;
    }

    pub fn append_session_to_main_scrollback_on_exit(&self) -> bool {
        self.append_session_to_main_scrollback_on_exit
    }

    /// Consume ApplicationOwned copy-notice edge (ptim15). True once per successful
    /// copy-on-release until taken; empty selection / copy-off never arm it.
    pub fn take_copy_notice(&mut self) -> bool {
        self.application_owned
            .as_mut()
            .is_some_and(|runtime| runtime.take_copy_notice())
    }

    /// Queue OSC52 (or other) clipboard sequences to flush after the next paint
    /// batch — same path as transcript copy-on-release (ptim05 / ptim13).
    /// Non-empty sequences also arm ApplicationOwned copy-notice (ptim15).
    pub fn enqueue_clipboard_sequences(&mut self, seqs: impl IntoIterator<Item = String>) {
        if let Some(runtime) = self.application_owned.as_mut() {
            runtime.enqueue_clipboard_seqs(seqs);
        }
    }

    /// Whether ApplicationOwned copy-notice is still within its TTL (~2s).
    pub fn copy_notice_active(&self) -> bool {
        self.application_owned
            .as_ref()
            .is_some_and(|runtime| runtime.copy_notice_active())
    }

    /// Whether ApplicationOwned transcript selection currently spans text.
    pub fn application_owned_has_selection(&self) -> bool {
        self.application_owned
            .as_ref()
            .is_some_and(|runtime| runtime.has_selection())
    }

    /// Register a pre-focus input listener. Returns an id for
    /// [`remove_input_listener`](Self::remove_input_listener).
    ///
    /// Listeners run in registration order before overlay/root focus routing.
    /// The first [`InputListenerResult::Consumed`] or
    /// [`InputListenerResult::ConsumedRerender`] stops the chain.
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

    /// Remove a previously registered input listener. No-op if `id` is unknown.
    pub fn remove_input_listener(&mut self, id: u64) {
        self.input_listeners.retain(|l| l.id != id);
    }

    pub fn full_redraws(&self) -> u64 {
        self.full_redraw_count
    }

    /// Frames actually painted via `do_render` (excludes throttle skips).
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Lines that ran normalize + `visible_width` in `do_render` (harness / obs).
    pub fn finalize_width_checks_for_test(&self) -> u64 {
        self.finalize_width_checks
    }

    /// Lines that reused the previous finalized string in `do_render` (harness / obs).
    pub fn finalize_line_reuses_for_test(&self) -> u64 {
        self.finalize_line_reuses
    }

    pub fn clear_finalize_counters_for_test(&mut self) {
        self.finalize_width_checks = 0;
        self.finalize_line_reuses = 0;
    }

    /// Cost of the last successful `do_render` (microseconds + line counts).
    pub fn last_render_perf(&self) -> RenderPerfSnap {
        self.last_render_perf
    }

    /// Whether a soft/force `request_render` is pending.
    pub fn is_render_requested(&self) -> bool {
        self.render_requested
    }

    /// Exact due time for a pending cached ApplicationOwned wheel paint.
    ///
    /// Hosts can await this deadline and call [`Self::try_render`] without
    /// restarting a full cadence interval from the latest input event. Other
    /// pending paints return `None` and keep their existing host tick policy.
    pub fn application_owned_wheel_render_deadline(&self) -> Option<std::time::Instant> {
        if !self.render_requested
            || !self.ao_wheel_render_requested
            || !self.can_reproject_application_owned(self.terminal.columns() as usize)
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

    /// Like `start()` but polls an external quit flag in addition to `stopped`.
    /// The TUI cleanly restores the terminal even if the flag is set externally.
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
        // terminal.start() owns: raw mode + bracketed paste + keyboard
        // protocol negotiation (Kitty push / modifyOtherKeys fallback, c410).
        self.terminal.hide_cursor();
        self.terminal.start();
        // ApplicationOwned: enter alt + mouse only after the TTY is started (c2070).
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
                        // Re-query size before rendering so we don't paint with
                        // stale columns/rows (the CrosstermTerminal caches them).
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
                        // Inline: drop bare Moved floods. ApplicationOwned owns mouse in
                        // dispatch_event (selection may need Moved while dragging).
                        if event.is_pointer_motion() && !self.application_session_active {
                            continue;
                        }
                        let reaction = self.dispatch_event(event);
                        if reaction == InputReaction::Rerender {
                            self.request_render(false);
                        }
                        // Mouse floods starve `poll` idle — still advance spinner /
                        // ticks, then coalesce paints @ 16ms.
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
                // Idle tick: advance animated or scripted components. If any
                // wants a repaint, drive a frame.
                let wants_render = self.idle_tick();
                if wants_render {
                    self.do_render()?;
                }
            }
        }

        // Host-driven loops must call the same teardown (`finish`).
        self.finish();
        Ok(())
    }

    /// Teardown matching the current interaction mode / session.
    ///
    /// ApplicationOwned (or an active ApplicationOwned session) →
    /// [`Self::finish_application_owned`];
    /// otherwise → [`Self::finish_inline`]. Prefer this from shared host exit paths.
    pub fn finish(&mut self) {
        if self.application_session_active || self.interaction_mode.is_application_owned() {
            self.finish_application_owned();
        } else {
            self.finish_inline();
        }
    }

    /// Park the cursor below the last rendered frame, emit `\r\n`, then
    /// restore the terminal (raw mode / keyboard protocol).
    ///
    /// Call this from host-driven **Inline** loops (product TUI / trust gate)
    /// instead of bare `terminal.stop()`, so the shell prompt lands under the
    /// leftover frame — same exit shape as `start` / `start_with_flag` /
    /// `agent_demo`.
    ///
    /// ApplicationOwned hosts MUST use [`Self::finish_application_owned`] or
    /// [`Self::finish`] — this method does not append the session or clear the
    /// ApplicationOwned runtime.
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

    /// ApplicationOwned teardown: leave alt-buffer, optionally append the session
    /// transcript (+ last dock) to the **main** terminal scrollback, then stop
    /// the terminal. Controlled by
    /// [`Self::set_append_session_to_main_scrollback_on_exit`] (default on).
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

    /// Temporarily release the terminal (leave raw mode / keyboard protocols)
    /// so a host can run an external process (e.g. `$EDITOR`), then restore.
    /// Does **not** exit the `start` event loop.
    ///
    /// **Paint is deferred** and **previous_lines are kept**: after an
    /// alternate-screen editor exits, the main buffer matches our last frame,
    /// so the next soft render can differential-update (grow/shrink the editor)
    /// without `\x1b[2J` wiping inline scrollback above the TUI (cargo output,
    /// etc.). Callers must `set_text` *after* this returns, then render —
    /// painting here with a stale buffer caused 1-line ghosts.
    ///
    /// If the TTY was resized while suspended, the next `do_render` still sees
    /// a real width/height change and clears as usual.
    ///
    /// Spawning `$EDITOR` / tempfile I/O stays in the application (demo or
    /// `src/app/tui`), not in this crate.
    pub fn with_terminal_suspended<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let restore_application_owned = self.application_session_active;
        self.terminal.stop();
        // `stop` leaves alt-buffer; keep logical ApplicationOwned flag so we can re-enter.
        if restore_application_owned {
            self.application_session_active = true;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        self.terminal.hide_cursor();
        self.terminal.start();
        if restore_application_owned {
            // Re-sync alt + mouse with application_session_active (ptim11).
            self.terminal.enter_alternate_screen();
            self.terminal.clear_screen();
            self.enable_mouse_capture();
            if self.application_owned.is_none() {
                let mut runtime = ApplicationOwnedRuntime::new(self.dock_rows);
                runtime.set_copy_on_release(self.transcript_copy_on_release);
                self.application_owned.replace(runtime);
            }
            self.previous_width = FORCE_SIZE_SENTINEL;
            self.previous_height = FORCE_SIZE_SENTINEL;
            self.previous_lines.clear();
            self.previous_viewport_top = 0;
        }
        // Size may have changed while the editor owned the TTY (pi SIGWINCH).
        self.terminal.refresh_size();
        // Soft pending only — do not wipe previous_lines / do not paint yet.
        self.request_render(restore_application_owned);
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    /// Hook invoked after each input `dispatch_event` inside `start` /
    /// `start_with_flag` (before the following render). Host-driven loops may
    /// call the same logic manually after `dispatch_event`.
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

    /// Route one decoded input event through listeners, then the focused
    /// overlay (if any) or root child.
    ///
    /// Returns whether a frame should be scheduled. Listener
    /// [`InputListenerResult::ConsumedRerender`] always paints;
    /// [`InputListenerResult::Consumed`] paints for Key/Paste only (Mouse stays
    /// quiet unless the focused component opts in via
    /// [`Component::input_wants_rerender`]).
    ///
    /// Public so host loops (and tests) can feed input without going through
    /// the blocking `start()` event loop.
    pub fn dispatch_event(&mut self, event: InputEvent) -> InputReaction {
        // Keys/paste always mutate chrome or editor — invalidate AO component cache.
        // Wheel/selection-only mouse returns early below without setting this.
        if matches!(event, InputEvent::Key(_) | InputEvent::Paste(_)) {
            self.ao_components_stale = true;
        }
        // ApplicationOwned: left Down clears the focused component's pointer
        // selection first so transcript ↔ editor highlights stay one-of
        // (click anywhere cancels the other). Then transcript/wheel may
        // consume; unhandled dock presses fall through to Editor.
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
            let wheel_in_transcript = is_wheel
                && mouse.row < rows.saturating_sub(self.dock_rows.min(rows as usize) as u16);
            // Park hit-priority on the live selection for this dispatch so
            // c2040 fold hooks survive runtime recreate (suspend / begin).
            let hit = self.transcript_hit_priority.take();
            let dirty = if let Some(runtime) = self.application_owned.as_mut() {
                runtime.selection.set_hit_priority(hit);
                let dirty = runtime.handle_mouse(mouse, cols, rows);
                self.transcript_hit_priority = runtime.selection.take_hit_priority();
                dirty
            } else {
                self.transcript_hit_priority = hit;
                false
            };
            if force_rerender {
                // Editor pointer clear needs Component::render for dock highlight.
                self.ao_components_stale = true;
            }
            if dirty {
                // Scroll/selection only: keep component cache, reproject viewport.
                // Copied chrome is armed by the host via take_copy_notice → mark stale.
                if is_wheel {
                    self.ao_wheel_render_requested = true;
                }
                return InputReaction::Rerender;
            }
            if wheel_in_transcript {
                // Consume an in-pane wheel event even at the content edge, but
                // avoid a no-op paint and do not leak it into the dock editor.
                return InputReaction::None;
            }
            if matches!(mouse.kind, MouseEventKind::Moved) {
                // Bare Moved: only continue when a focused child owns an active
                // drag (ptim13 editor). Otherwise drop to avoid motion floods.
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
            // Non-moved (or drag-owned Moved) not consumed by transcript → fall through.
        }

        // Snapshot ids so a listener may remove itself / others mid-dispatch
        // without invalidating the walk; callbacks are invoked by id lookup.
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
                    // Key/Paste: historical always-paint. Mouse: silent consume
                    // so a no-op handler cannot flood frames (c2020 / c2040).
                    let paint = force_rerender || !matches!(event, InputEvent::Mouse(_));
                    if paint {
                        self.ao_components_stale = true;
                    }
                    return InputReaction::rerender_if(paint);
                }
            }
        }

        // If focused overlay became invisible, redirect (preserve restore).
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

        // Reclaim eligible / blocked resume when focus is not on an overlay.
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

        // Route to focused overlay (including explicitly focused non_capturing).
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
            // Query wants *after* handle so mouse_dirty / dragging updates count
            // (Down must schedule a frame for editor selection highlight).
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

    /// Clear pointer selection on the focused overlay or root child.
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

    /// Opt in to crossterm mouse capture (default off). Restored across
    /// [`Self::with_terminal_suspended`] when still desired.
    pub fn enable_mouse_capture(&mut self) {
        self.terminal.enable_mouse_capture();
    }

    /// Opt out of mouse capture. Also runs on `Terminal::stop` / `finish` paths.
    pub fn disable_mouse_capture(&mut self) {
        self.terminal.disable_mouse_capture();
    }

    pub fn mouse_capture_enabled(&self) -> bool {
        self.terminal.mouse_capture_active()
    }

    /// Advance time-driven component state once without reading input.
    ///
    /// Host-driven loops and test harnesses can call this to exercise the same
    /// idle path that `start()` uses for spinner/script progress.
    pub fn idle_tick(&mut self) -> bool {
        let mut changed = self.components.iter_mut().any(|c| c.tick());
        changed |= self
            .overlays
            .iter_mut()
            .any(|(component, _, _)| component.tick());
        if changed {
            // Spinner / chrome animations must rebuild dock lines.
            self.ao_components_stale = true;
        }
        if self.application_session_active {
            let cols = self.terminal.columns();
            let rows = self.terminal.rows();
            if let Some(runtime) = self.application_owned.as_mut() {
                changed |= runtime.tick_autoscroll(cols, rows);
                if runtime.tick_copy_notice() {
                    // Status row lost "Copied" — refresh chrome.
                    self.ao_components_stale = true;
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
