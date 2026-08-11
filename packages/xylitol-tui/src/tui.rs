use crate::interaction_mode::InteractionMode;
use crate::mode_b::ModeBRuntime;
use crate::terminal::Terminal;
use crate::utils::{normalize_terminal_output, visible_width};
use crossterm::event::{KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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

/// Heuristic for Kitty inline-image lines (APC `<_...ST`). Their visible width
/// is 0 (all escape bytes), so they're exempt from the width invariant.
fn is_image_line(line: &str) -> bool {
    line.contains("\x1b_G") || line.contains("\x1b]1337;File")
}

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
    /// since the last take (e.g. Editor Mode B copy-on-release). Default empty.
    fn take_pending_clipboard(&mut self) -> Vec<String> {
        Vec::new()
    }
}

pub trait Focusable: Component {
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
}

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
enum OverlayFocusRestorePolicy {
    Clear,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockedResume {
    RestoreOverlay,
    FocusTarget(Option<FocusTarget>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayFocusRestore {
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
struct OverlayStackEntry {
    overlay_id: u64,
    /// Focus target before this overlay opened (root or overlay); restored on hide.
    pre_focus: Option<FocusTarget>,
    hidden: bool,
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

    pub fn is_hidden<T: Terminal>(self, tui: &TUI<T>) -> bool {
        tui.is_overlay_hidden(self)
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
    /// Dual interaction mode (c2070). Default [`InteractionMode::Inline`].
    interaction_mode: InteractionMode,
    /// Mode B session currently holding alt-buffer + mouse (after
    /// [`Self::begin_application_owned_session`]).
    application_session_active: bool,
    /// Mode B viewport/selection runtime (only while application session active).
    mode_b: Option<ModeBRuntime>,
    /// Default dock rows used when beginning a Mode B session.
    mode_b_dock_rows: usize,
    /// Persists across begin/end; applied when constructing [`ModeBRuntime`].
    mode_b_copy_on_release: bool,
}

/// Minimum spacing between throttled frames (~60fps). Mirrors pi's
/// MIN_RENDER_INTERVAL_MS. `render_frame` bypasses this; `try_render` honors it.
const MIN_RENDER_INTERVAL_MS: u64 = 16;
const BEGIN_RENDER_BATCH: &str = "\x1b[?2026h\x1b[?7l";
const END_RENDER_BATCH: &str = "\x1b[?7h\x1b[?2026l";
/// pi `requestRender(true)` sets `previousWidth = -1` so `widthChanged` is true
/// and the next frame takes `fullRender(true)` (`2J`/`H`/`3J`). `usize` has no
/// `-1`; use `MAX` as the same non-zero ≠ real-width sentinel.
const FORCE_SIZE_SENTINEL: usize = usize::MAX;

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
            focus_order_counter: 0,
            next_overlay_id: 1,
            next_input_listener_id: 1,
            input_listeners: Vec::new(),
            after_dispatch_hook: None,
            render_requested: false,
            last_render_at: None,
            interaction_mode: InteractionMode::Inline,
            application_session_active: false,
            mode_b: None,
            mode_b_dock_rows: 3,
            mode_b_copy_on_release: true,
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

    /// Set the mode flag without entering/leaving alt-buffer. Prefer
    /// [`Self::with_interaction_mode`] at construction; for a live switch,
    /// call [`Self::end_application_owned_session`] / [`Self::finish_inline`]
    /// then rebuild the TUI.
    ///
    /// If an application session is active and `mode` is Inline, this ends the
    /// session so flag and TTY state stay aligned.
    pub fn set_interaction_mode(&mut self, mode: InteractionMode) {
        if mode.is_inline() && self.application_session_active {
            self.end_application_owned_session();
        }
        self.interaction_mode = mode;
    }

    /// Mode B enter: alternate screen (SHOULD) + mouse capture + viewport runtime.
    /// Idempotent. No-op when mode is [`InteractionMode::Inline`].
    pub fn begin_application_owned_session(&mut self) {
        if !self.interaction_mode.is_application_owned() || self.application_session_active {
            return;
        }
        self.terminal.enter_alternate_screen();
        self.terminal.clear_screen();
        self.enable_mouse_capture();
        self.application_session_active = true;
        let mut mb = ModeBRuntime::new(self.mode_b_dock_rows);
        mb.set_copy_on_release(self.mode_b_copy_on_release);
        self.mode_b = Some(mb);
        // Force a clearing redraw into the alt buffer.
        self.previous_width = FORCE_SIZE_SENTINEL;
        self.previous_height = FORCE_SIZE_SENTINEL;
        self.previous_lines.clear();
        self.previous_viewport_top = 0;
    }

    /// Mode B leave: disable mouse + leave alt-buffer. Safe if never begun.
    pub fn end_application_owned_session(&mut self) {
        if !self.application_session_active {
            return;
        }
        self.disable_mouse_capture();
        self.terminal.leave_alternate_screen();
        self.application_session_active = false;
        self.mode_b = None;
        self.previous_lines.clear();
        self.previous_viewport_top = 0;
        self.previous_width = FORCE_SIZE_SENTINEL;
        self.previous_height = FORCE_SIZE_SENTINEL;
    }

    pub fn application_session_active(&self) -> bool {
        self.application_session_active
    }

    /// Rows reserved at the bottom for dock (editor/status/footer). Mode B
    /// selection excludes this band.
    pub fn set_mode_b_dock_rows(&mut self, rows: usize) {
        self.mode_b_dock_rows = rows.max(1);
        if let Some(mb) = self.mode_b.as_mut() {
            mb.set_dock_rows(self.mode_b_dock_rows);
        }
    }

    pub fn mode_b_dock_rows(&self) -> usize {
        self.mode_b_dock_rows
    }

    pub fn set_mode_b_copy_on_release(&mut self, on: bool) {
        self.mode_b_copy_on_release = on;
        if let Some(mb) = self.mode_b.as_mut() {
            mb.set_copy_on_release(on);
        }
    }

    /// Consume Mode B copy-notice edge (ptim15). True once per successful
    /// copy-on-release until taken; empty selection / copy-off never arm it.
    pub fn take_copy_notice(&mut self) -> bool {
        self.mode_b.as_mut().is_some_and(|mb| mb.take_copy_notice())
    }

    /// Queue OSC52 (or other) clipboard sequences to flush after the next paint
    /// batch — same path as transcript copy-on-release (ptim05 / ptim13).
    /// Non-empty sequences also arm Mode B copy-notice (ptim15).
    pub fn enqueue_clipboard_sequences(&mut self, seqs: impl IntoIterator<Item = String>) {
        if let Some(mb) = self.mode_b.as_mut() {
            mb.enqueue_clipboard_seqs(seqs);
        }
    }

    /// Whether Mode B copy-notice is still within its TTL (~2s).
    pub fn copy_notice_active(&self) -> bool {
        self.mode_b
            .as_ref()
            .is_some_and(|mb| mb.copy_notice_active())
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

    /// Whether a soft/force `request_render` is pending.
    pub fn is_render_requested(&self) -> bool {
        self.render_requested
    }
    pub fn set_show_hardware_cursor(&mut self, enabled: bool) {
        if self.show_hardware_cursor == enabled {
            return;
        }
        self.show_hardware_cursor = enabled;
        if !enabled {
            self.terminal.hide_cursor();
        }
    }
    pub fn set_clear_on_shrink(&mut self, enabled: bool) {
        self.clear_on_shrink = enabled;
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

    fn overlay_index(&self, overlay_id: u64) -> Option<usize> {
        self.overlays
            .iter()
            .position(|(_, _, e)| e.overlay_id == overlay_id)
    }

    fn is_overlay_entry_visible(&self, entry: &OverlayStackEntry) -> bool {
        !entry.hidden
    }

    fn topmost_visible_capturing_overlay_id(&self) -> Option<u64> {
        self.overlays
            .iter()
            .filter(|(_, opts, e)| !opts.non_capturing && self.is_overlay_entry_visible(e))
            .max_by_key(|(_, _, e)| e.focus_order)
            .map(|(_, _, e)| e.overlay_id)
    }

    fn current_focus_target(&self) -> Option<FocusTarget> {
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

    fn clear_overlay_focus_restore(&mut self) {
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

    fn get_visible_overlay_focus_restore(&self) -> OverlayFocusRestore {
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

    fn set_focus_internal(
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
        // Mode B: enter alt + mouse only after the TTY is started (c2070).
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
                        // Mode A: drop bare Moved floods. Mode B owns mouse in
                        // dispatch_event (selection may need Moved while dragging).
                        if event.is_pointer_motion() && !self.application_session_active {
                            continue;
                        }
                        let reaction = self.dispatch_event(event);
                        if reaction == InputReaction::Rerender {
                            self.run_after_dispatch_hook();
                            self.do_render()?;
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

        // Host-driven loops must call the same teardown (see `finish_inline`).
        self.finish_inline();
        Ok(())
    }

    /// Park the cursor below the last rendered frame, emit `\r\n`, then
    /// restore the terminal (raw mode / keyboard protocol).
    ///
    /// Call this from host-driven loops (product TUI / trust gate) instead of
    /// bare `terminal.stop()`, so the shell prompt lands under the leftover
    /// frame — same exit shape as `start` / `start_with_flag` / agent_demo.
    ///
    /// Mode B ([`InteractionMode::ApplicationOwned`]): leave alt-buffer, then
    /// dump transcript (+ last dock) onto the **main** screen so the session
    /// remains in terminal scrollback (Pi-style optional dump after `?1049l`).
    pub fn finish_inline(&mut self) {
        self.stopped = true;
        if self.application_session_active || self.interaction_mode.is_application_owned() {
            let dump = self
                .mode_b
                .as_ref()
                .map(|mb| mb.exit_dump_lines())
                .filter(|lines| !lines.is_empty())
                .unwrap_or_else(|| self.previous_lines.clone());
            self.end_application_owned_session();
            // Main buffer is restored; append session text as scrollback.
            for line in &dump {
                self.terminal.write(line);
                self.terminal.write("\r\n");
            }
            self.terminal.flush();
            self.terminal.stop();
            return;
        }
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
        let restore_mode_b = self.application_session_active;
        self.terminal.stop();
        // `stop` leaves alt-buffer; keep logical Mode B flag so we can re-enter.
        if restore_mode_b {
            self.application_session_active = true;
        }
        let result = f();
        self.terminal.hide_cursor();
        self.terminal.start();
        if restore_mode_b {
            // Re-sync alt + mouse with application_session_active (ptim11).
            self.terminal.enter_alternate_screen();
            self.terminal.clear_screen();
            self.enable_mouse_capture();
            if self.mode_b.is_none() {
                let mut mb = ModeBRuntime::new(self.mode_b_dock_rows);
                mb.set_copy_on_release(self.mode_b_copy_on_release);
                self.mode_b.replace(mb);
            }
            self.previous_width = FORCE_SIZE_SENTINEL;
            self.previous_height = FORCE_SIZE_SENTINEL;
            self.previous_lines.clear();
            self.previous_viewport_top = 0;
        }
        // Size may have changed while the editor owned the TTY (pi SIGWINCH).
        self.terminal.refresh_size();
        // Soft pending only — do not wipe previous_lines / do not paint yet.
        self.request_render(restore_mode_b);
        result
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
        // Mode B: application selection / wheel consume mouse before listeners
        // when they dirty presentation. Unhandled presses (e.g. Down in dock)
        // fall through so Editor/Input can own an independent selection later.
        if self.application_session_active
            && let InputEvent::Mouse(mouse) = &event
        {
            let cols = self.terminal.columns();
            let rows = self.terminal.rows();
            let dirty = self
                .mode_b
                .as_mut()
                .is_some_and(|mb| mb.handle_mouse(mouse, cols, rows));
            if dirty {
                return InputReaction::Rerender;
            }
            if matches!(mouse.kind, MouseEventKind::Moved) {
                // Moved with no dirty: drop silently (ptim07).
                return InputReaction::None;
            }
            // Non-moved, not consumed by transcript selection → fall through.
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
                InputListenerResult::ConsumedRerender => return InputReaction::Rerender,
                InputListenerResult::Consumed => {
                    // Key/Paste: historical always-paint. Mouse: silent consume
                    // so a no-op handler cannot flood frames (c2020 / c2040).
                    return InputReaction::rerender_if(!matches!(event, InputEvent::Mouse(_)));
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
            let wants = self.overlays[index].0.input_wants_rerender(&event);
            self.overlays[index].0.handle_input(event);
            let seqs = self.overlays[index].0.take_pending_clipboard();
            self.ingest_component_clipboard(seqs);
            return InputReaction::rerender_if(wants);
        }
        if let Some(idx) = self.focused_index
            && idx < self.components.len()
        {
            let wants = self.components[idx].input_wants_rerender(&event);
            self.components[idx].handle_input(event);
            let seqs = self.components[idx].take_pending_clipboard();
            self.ingest_component_clipboard(seqs);
            return InputReaction::rerender_if(wants);
        }
        InputReaction::None
    }

    fn ingest_component_clipboard(&mut self, seqs: Vec<String>) {
        if seqs.is_empty() {
            return;
        }
        if let Some(mb) = self.mode_b.as_mut() {
            mb.enqueue_clipboard_seqs(seqs);
        }
    }

    /// Opt in to crossterm mouse capture (default off). Restored across
    /// [`Self::with_terminal_suspended`] when still desired.
    pub fn enable_mouse_capture(&mut self) {
        self.terminal.enable_mouse_capture();
    }

    /// Opt out of mouse capture. Also runs on `Terminal::stop` / `finish_inline`.
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
        if self.application_session_active {
            let cols = self.terminal.columns();
            let rows = self.terminal.rows();
            if let Some(mb) = self.mode_b.as_mut() {
                changed |= mb.tick_autoscroll(cols, rows);
                changed |= mb.tick_copy_notice();
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

    /// Run one render pass: composite children + overlays, diff against the
    /// previous frame, and write only the changed lines to the terminal.
    ///
    /// Public so host loops (and tests) can drive single frames instead of the
    /// blocking `start()` loop. In a host-driven setup the host calls this after
    /// state changes (input, async events, ticks). Bypasses the throttle — use
    /// `try_render` to honor it, or `request_render` to mark + let a host loop
    /// drive the actual frame.
    pub fn render_frame(&mut self) -> Result<(), RenderError> {
        self.do_render()
    }

    /// Mark a render as needed. The actual frame is driven by whoever calls
    /// `try_render` (a host loop) or by `run_event_loop`'s internal timer. If
    /// `force`, previous-frame state is reset so the next render takes the
    /// **clearing** full-redraw path (aligned with pi `requestRender(true)`).
    ///
    /// Force sets `previous_width/height` to [`FORCE_SIZE_SENTINEL`] (pi `-1`),
    /// which makes `width_changed`/`height_changed` true so `full_render`
    /// emits `\x1b[2J\x1b[H\x1b[3J`. First paint and ordinary soft renders still
    /// use `previous_* = 0` / real sizes (no clear). See `PI_DELTAS.md` D19.
    ///
    /// **Resize:** call with `force=false` (pi: soft `requestRender()` on
    /// stdout resize) so real size deltas drive the same clear path.
    pub fn request_render(&mut self, force: bool) {
        if force {
            self.previous_lines.clear();
            self.previous_width = FORCE_SIZE_SENTINEL;
            self.previous_height = FORCE_SIZE_SENTINEL;
            self.previous_viewport_top = 0;
            self.cursor_row = 0;
            self.hardware_cursor_row = 0;
            self.max_lines_rendered = 0;
        }
        self.render_requested = true;
    }

    /// Render only if one is pending AND the 16ms throttle has elapsed. Returns
    /// Ok(true) if a frame was actually rendered, Ok(false) if skipped, Err if
    /// the render hit the width invariant. Host loops call this on their tick.
    pub fn try_render(&mut self) -> Result<bool, RenderError> {
        if !self.render_requested {
            return Ok(false);
        }
        if let Some(last) = self.last_render_at {
            let elapsed = last.elapsed();
            if elapsed < std::time::Duration::from_millis(MIN_RENDER_INTERVAL_MS) {
                return Ok(false);
            }
        }
        self.render_requested = false;
        self.last_render_at = Some(std::time::Instant::now());
        self.do_render()?;
        Ok(true)
    }

    /// Render immediately, bypassing the throttle. Returns Ok(true) if a frame
    /// was rendered (i.e. not stopped), Err on width-invariant violation.
    pub fn render_now(&mut self) -> Result<bool, RenderError> {
        self.render_requested = false;
        self.last_render_at = Some(std::time::Instant::now());
        self.do_render()?;
        Ok(!self.stopped)
    }

    fn do_render(&mut self) -> Result<(), RenderError> {
        if self.stopped {
            return Ok(());
        }
        self.frame_count = self.frame_count.saturating_add(1);
        let width = self.terminal.columns() as usize;
        let height = self.terminal.rows() as usize;
        if width == 0 {
            return Ok(());
        }

        let mut new_lines = Vec::new();
        for comp in &mut self.components {
            new_lines.extend(comp.render(width));
        }

        if !self.overlays.is_empty() {
            new_lines = self.composite_overlays(new_lines, width, height);
        }

        // pi: extract cursor marker before applyLineResets.
        let cursor_pos = self.extract_cursor_position(&mut new_lines, height);

        // pi applyLineResets + hard width invariant.
        // Fast path (A+B): when terminal width is unchanged and the component
        // emitted the same pre-reset body as last frame, reuse the previous
        // finalized line (already normalized + SEGMENT_RESET + width-ok).
        // Skip normalize and visible_width for that line. On resize, or when
        // content differs, take the slow path so overflow still errors.
        const SEGMENT_RESET: &str = "\x1b[0m\x1b]8;;\x07";
        let reuse_ok = self.previous_width == width;
        for (i, line) in new_lines.iter_mut().enumerate() {
            if is_image_line(line) {
                continue;
            }
            if reuse_ok
                && let Some(prev) = self.previous_lines.get(i)
                && let Some(body) = prev.strip_suffix(SEGMENT_RESET)
                && body == line.as_str()
            {
                *line = prev.clone();
                self.finalize_line_reuses = self.finalize_line_reuses.saturating_add(1);
                continue;
            }

            let mut finalized = normalize_terminal_output(line);
            finalized.push_str(SEGMENT_RESET);
            // Image-bearing lines (Kitty APC) are exempt — their visible width is 0.
            // CURSOR_MARKER lines are measured after extract; marker-bearing leftovers skip.
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

        // Mode B: project full buffer into fixed-height app viewport + dock.
        if self.application_session_active {
            if let Some(mb) = self.mode_b.as_mut() {
                new_lines = mb.project_frame(&new_lines, height);
            }
            // Never grow terminal scrollback in Mode B.
            self.previous_viewport_top = 0;
        }

        let width_changed = self.previous_width != 0 && self.previous_width != width;
        let height_changed = self.previous_height != 0 && self.previous_height != height;

        // Compute the changed range up front so the full-vs-diff decision can
        // consider it (pi decides after computing firstChanged/lastChanged).
        let (first_changed, last_changed, appended) = self.compute_line_diff(&new_lines);

        // pi all-deletions safety nets (tui.ts:1411-1425): escalate to full clear.
        let deletions_need_full = first_changed >= new_lines.len() as isize
            && self.previous_lines.len() > new_lines.len()
            && {
                let target = new_lines.len().saturating_sub(1);
                let extra = self.previous_lines.len() - new_lines.len();
                target < self.previous_viewport_top || extra > height
            };

        // Full redraw triggers (order matches pi tui.ts:1335-1459):
        // 1. first frame / width change / height change (non-Termux)
        // 2. clearOnShrink (content shrank below the historical high-water mark)
        // 3. firstChanged < prevViewportTop — the change is above the visible
        //    viewport, the diff path can't reach it, so repaint everything.
        // 4. all-deletions with the new tail above the viewport — likewise can't
        //    be expressed as a diff, fullRedraw to resync.
        // 5. all-deletions safety nets (target above viewport / extra > height).
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

        if do_full {
            let clear = !(self.previous_lines.is_empty() && !width_changed && !height_changed);
            self.full_render(&new_lines, clear, height);
        } else if first_changed < 0 {
            // No change at all — just reposition the cursor (pi's no-op branch).
        } else {
            self.differential_render(
                &new_lines,
                width,
                height,
                first_changed,
                last_changed,
                appended,
            );
        }

        self.position_cursor(cursor_pos, new_lines.len());
        self.previous_lines = new_lines;
        self.previous_width = width;
        self.previous_height = height;
        if self.application_session_active {
            self.previous_viewport_top = 0;
        }
        self.terminal.flush();
        // OSC52 must leave the synchronized output batch (ptim05).
        if let Some(mb) = self.mode_b.as_mut() {
            for seq in mb.take_pending_clipboard() {
                self.terminal.write(&seq);
            }
            self.terminal.flush();
        }
        Ok(())
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
        // Content-end aligned viewport: the bottom of the content sticks to the
        // bottom of the screen when content exceeds one screen. Mirrors pi's
        // `previousViewportTop = max(0, max(height, len) - height)`.
        self.previous_viewport_top = height.max(len).saturating_sub(height);
    }

    /// Find the first/last changed line index comparing new_lines to
    /// previous_lines. Returns (first, last, appended) with -1 meaning "none".
    /// Mirrors pi's firstChanged/lastChanged + append detection.
    fn compute_line_diff(&self, new_lines: &[String]) -> (isize, isize, bool) {
        let mut first_changed: isize = -1;
        let mut last_changed: isize = -1;
        let max_lines = new_lines.len().max(self.previous_lines.len());
        for i in 0..max_lines {
            let old = self.previous_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let new = new_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            if old != new {
                if first_changed == -1 {
                    first_changed = i as isize;
                }
                last_changed = i as isize;
            }
        }
        let appended = new_lines.len() > self.previous_lines.len();
        if appended {
            if first_changed == -1 {
                first_changed = self.previous_lines.len() as isize;
            }
            last_changed = (new_lines.len() - 1) as isize;
        }
        (first_changed, last_changed, appended)
    }

    fn differential_render(
        &mut self,
        new_lines: &[String],
        _width: usize,
        height: usize,
        first_changed_in: isize,
        last_changed: isize,
        appended: bool,
    ) {
        let prev_viewport_top = self.previous_viewport_top;

        // All-deletions branch: content only shrank. Clear the surplus lines.
        // Full-clear guards already handled in `do_render` (`deletions_need_full`).
        if first_changed_in >= new_lines.len() as isize {
            if self.previous_lines.len() > new_lines.len() {
                let mut buf = String::from(BEGIN_RENDER_BATCH);
                let target = new_lines.len().saturating_sub(1);
                // Viewport-relative move (pi computeLineDiff).
                let current_screen = (self.hardware_cursor_row as isize
                    - prev_viewport_top as isize)
                    .clamp(0, height.saturating_sub(1) as isize);
                let target_screen = target as isize - prev_viewport_top as isize;
                let diff = target_screen - current_screen;
                if diff > 0 {
                    buf.push_str(&format!("\x1b[{}B", diff));
                } else if diff < 0 {
                    buf.push_str(&format!("\x1b[{}A", -diff));
                }
                buf.push('\r');
                let extra = self.previous_lines.len() - new_lines.len();
                let off = if new_lines.is_empty() { 0 } else { 1 };
                if extra > 0 && off > 0 {
                    buf.push_str(&format!("\x1b[{}B", off));
                }
                for i in 0..extra {
                    buf.push_str("\r\x1b[2K");
                    if i < extra - 1 {
                        buf.push_str("\x1b[1B");
                    }
                }
                let back = extra.saturating_sub(1) + off;
                if back > 0 {
                    buf.push_str(&format!("\x1b[{}A", back));
                }
                buf.push_str(END_RENDER_BATCH);
                self.terminal.write(&buf);
                self.cursor_row = target;
                self.hardware_cursor_row = target;
                self.previous_viewport_top = height.max(new_lines.len()).saturating_sub(height);
                self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
            }
            return;
        }

        let first_changed = first_changed_in as usize;
        let append_start =
            appended && first_changed == self.previous_lines.len() && first_changed > 0;
        let move_target = if append_start {
            first_changed - 1
        } else {
            first_changed
        };

        let mut buf = String::from(BEGIN_RENDER_BATCH);
        let mut hardware_cursor_row = self.hardware_cursor_row;

        // Viewport scroll (pi Step 5C, tui.ts:1466-1478): if the changed region
        // falls below the previous viewport bottom, CUD to the screen's last row
        // then emit `\r\n` to scroll the terminal — lifting prevViewportTop so
        // the diff writes land on visible rows instead of past the screen edge.
        let mut prev_viewport_top = self.previous_viewport_top;
        let prev_viewport_bottom = prev_viewport_top + height.saturating_sub(1);
        if move_target > prev_viewport_bottom {
            let current_screen_row = (hardware_cursor_row as isize - prev_viewport_top as isize)
                .clamp(0, height.saturating_sub(1) as isize)
                as usize;
            let move_to_bottom = height - 1 - current_screen_row;
            if move_to_bottom > 0 {
                buf.push_str(&format!("\x1b[{}B", move_to_bottom));
            }
            let scroll = move_target - prev_viewport_bottom;
            if scroll > 0 {
                buf.push_str(&"\r\n".repeat(scroll));
            }
            prev_viewport_top += scroll;
            // pi: hardwareCursorRow = moveTargetRow after scroll.
            hardware_cursor_row = move_target;
        }

        // Move to the target row (relative to the current viewport top) and
        // start the changed region.
        let screen_row = move_target as isize - prev_viewport_top as isize;
        let cursor_diff = screen_row - (hardware_cursor_row as isize - prev_viewport_top as isize);
        if cursor_diff > 0 {
            buf.push_str(&format!("\x1b[{}B", cursor_diff));
        } else if cursor_diff < 0 {
            buf.push_str(&format!("\x1b[{}A", -cursor_diff));
        }
        buf.push_str(if append_start { "\r\n" } else { "\r" });

        let end = last_changed.min((new_lines.len() - 1) as isize) as usize;
        let start = first_changed;
        for (i, line) in new_lines.iter().enumerate().take(end + 1).skip(start) {
            if i > start {
                buf.push_str("\r\n");
            }
            buf.push_str("\x1b[2K");
            buf.push_str(line);
        }

        // Shrink cleanup in the same sync batch (pi tui.ts:1555-1568): move to
        // content end if we stopped early, clear surplus rows, step back.
        let mut final_cursor_row = end;
        if self.previous_lines.len() > new_lines.len() && !appended {
            let content_end = new_lines.len().saturating_sub(1);
            if end < content_end {
                let move_down = content_end - end;
                buf.push_str(&format!("\x1b[{}B", move_down));
                final_cursor_row = content_end;
            }
            let extra = self.previous_lines.len() - new_lines.len();
            for _ in 0..extra {
                buf.push_str("\r\n\x1b[2K");
            }
            if extra > 0 {
                buf.push_str(&format!("\x1b[{}A", extra));
            }
        }

        buf.push_str(END_RENDER_BATCH);
        self.terminal.write(&buf);

        self.cursor_row = new_lines.len().saturating_sub(1);
        self.hardware_cursor_row = final_cursor_row;
        self.previous_viewport_top =
            prev_viewport_top.max(final_cursor_row.saturating_sub(height.saturating_sub(1)));
        self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
    }

    fn composite_overlays(
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

    /// Position the hardware cursor for IME (pi `positionHardwareCursor`).
    ///
    /// Default `show_hardware_cursor == false`: still move to the marker so
    /// IME candidate windows track the edit point, but keep the cursor
    /// **hidden** — the Editor's reverse-video fake cursor is the visible cue.
    /// Showing the hardware cursor during streaming redraws causes flicker.
    fn position_cursor(&mut self, cursor_pos: Option<(usize, usize)>, total_lines: usize) {
        if cursor_pos.is_none() || total_lines == 0 {
            self.terminal.hide_cursor();
            return;
        }
        let (row, col) = cursor_pos.expect("checked above");
        let target_row = row.min(total_lines.saturating_sub(1));
        let target_col = col;

        // Relative row move from the last known hardware position (pi), then
        // absolute column (CHA). Avoids CUP-from-origin which desyncs once
        // content has scrolled into the terminal scrollback.
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
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventState, KeyModifiers};

    struct DummyTerminal;

    impl Terminal for DummyTerminal {
        fn write(&mut self, _data: &str) {}
        fn columns(&self) -> u16 {
            80
        }
        fn rows(&self) -> u16 {
            24
        }
        fn hide_cursor(&mut self) {}
        fn show_cursor(&mut self) {}
        fn clear_line(&mut self) {}
        fn clear_from_cursor(&mut self) {}
        fn clear_screen(&mut self) {}
        fn flush(&mut self) {}
    }

    struct DummyComponent {
        wants_release: bool,
    }

    impl Component for DummyComponent {
        fn render(&mut self, _width: usize) -> Vec<String> {
            Vec::new()
        }

        fn handle_input(&mut self, _event: InputEvent) {}

        fn invalidate(&mut self) {}

        fn wants_key_release(&self) -> bool {
            self.wants_release
        }
    }

    fn key_with_kind(kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn dispatches_press_and_repeat() {
        let tui = TUI::new(DummyTerminal);

        assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Press)));
        assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Repeat)));
    }

    #[test]
    fn ignores_release_without_opt_in() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(DummyComponent {
            wants_release: false,
        }));
        tui.set_focus(Some(0));

        assert!(!tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
    }

    #[test]
    fn dispatches_release_when_focused_component_requests_it() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(DummyComponent {
            wants_release: true,
        }));
        tui.set_focus(Some(0));

        assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
    }

    struct RecordingComponent {
        events: std::sync::Arc<std::sync::Mutex<Vec<InputEvent>>>,
    }

    impl Component for RecordingComponent {
        fn render(&mut self, _width: usize) -> Vec<String> {
            Vec::new()
        }
        fn handle_input(&mut self, event: InputEvent) {
            self.events.lock().unwrap().push(event);
        }
        fn invalidate(&mut self) {}
    }

    fn letter_a() -> InputEvent {
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn ctrl_c() -> InputEvent {
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn input_listener_consumes_before_focus() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(RecordingComponent {
            events: events.clone(),
        }));
        tui.set_focus(Some(0));
        tui.add_input_listener(|ev| {
            if matches!(
                &ev,
                InputEvent::Key(k)
                    if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL)
            ) {
                InputListenerResult::Consumed
            } else {
                InputListenerResult::Continue
            }
        });

        tui.dispatch_event(ctrl_c());
        assert!(events.lock().unwrap().is_empty());

        tui.dispatch_event(letter_a());
        assert_eq!(events.lock().unwrap().len(), 1);
    }

    #[test]
    fn input_listeners_run_fifo_until_consumed() {
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut tui = TUI::new(DummyTerminal);
        let o1 = order.clone();
        tui.add_input_listener(move |_| {
            o1.lock().unwrap().push(1);
            InputListenerResult::Continue
        });
        let o2 = order.clone();
        tui.add_input_listener(move |_| {
            o2.lock().unwrap().push(2);
            InputListenerResult::Consumed
        });
        let o3 = order.clone();
        tui.add_input_listener(move |_| {
            o3.lock().unwrap().push(3);
            InputListenerResult::Continue
        });

        tui.dispatch_event(letter_a());
        assert_eq!(*order.lock().unwrap(), vec![1, 2]);
    }

    #[test]
    fn paste_passes_through_continuing_listener() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(RecordingComponent {
            events: events.clone(),
        }));
        tui.set_focus(Some(0));
        tui.add_input_listener(|_| InputListenerResult::Continue);

        tui.dispatch_event(InputEvent::Paste("hello".into()));
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &[InputEvent::Paste("hello".into())]
        );
    }

    fn mouse_moved() -> InputEvent {
        InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 1,
            row: 2,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn mouse_left_down() -> InputEvent {
        InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 3,
            row: 4,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn mouse_motion_is_pointer_motion() {
        assert!(mouse_moved().is_pointer_motion());
        assert!(!mouse_left_down().is_pointer_motion());
        assert!(!letter_a().is_pointer_motion());
    }

    #[test]
    fn mouse_down_reaches_focused_child_without_default_rerender() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(RecordingComponent {
            events: events.clone(),
        }));
        tui.set_focus(Some(0));

        let reaction = tui.dispatch_event(mouse_left_down());
        assert_eq!(reaction, InputReaction::None);
        assert_eq!(events.lock().unwrap().len(), 1);
        assert!(matches!(
            events.lock().unwrap()[0],
            InputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(_),
                ..
            })
        ));
    }

    #[test]
    fn mouse_listener_consumed_does_not_request_rerender() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_input_listener(|_| InputListenerResult::Consumed);
        assert_eq!(tui.dispatch_event(mouse_left_down()), InputReaction::None);
    }

    #[test]
    fn mouse_listener_consumed_rerender_requests_frame() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_input_listener(|_| InputListenerResult::ConsumedRerender);
        assert_eq!(
            tui.dispatch_event(mouse_left_down()),
            InputReaction::Rerender
        );
    }

    #[test]
    fn mouse_listener_consumed_blocks_focused_child() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(RecordingComponent {
            events: events.clone(),
        }));
        tui.set_focus(Some(0));
        tui.add_input_listener(|_| InputListenerResult::Consumed);
        assert_eq!(tui.dispatch_event(mouse_left_down()), InputReaction::None);
        assert!(
            events.lock().unwrap().is_empty(),
            "Consumed listener must block focused child"
        );
    }

    struct MouseAwareComponent;

    impl Component for MouseAwareComponent {
        fn render(&mut self, _width: usize) -> Vec<String> {
            Vec::new()
        }
        fn handle_input(&mut self, _event: InputEvent) {}
        fn invalidate(&mut self) {}
        fn input_wants_rerender(&self, event: &InputEvent) -> bool {
            matches!(
                event,
                InputEvent::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(_),
                    ..
                })
            )
        }
    }

    #[test]
    fn component_can_opt_in_mouse_rerender() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(MouseAwareComponent));
        tui.set_focus(Some(0));
        assert_eq!(
            tui.dispatch_event(mouse_left_down()),
            InputReaction::Rerender
        );
        assert_eq!(tui.dispatch_event(mouse_moved()), InputReaction::None);
    }

    #[test]
    fn mouse_moved_flood_does_not_bump_frame_count() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(RecordingComponent {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }));
        tui.set_focus(Some(0));
        let before = tui.frame_count();
        for _ in 0..64 {
            if tui.dispatch_event(mouse_moved()) == InputReaction::Rerender {
                tui.request_render(false);
            }
        }
        let _ = tui.try_render();
        assert_eq!(
            tui.frame_count(),
            before,
            "Moved flood must not schedule or paint frames"
        );
    }

    #[test]
    fn mouse_consumed_listener_can_bump_frame_count() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_input_listener(|_| InputListenerResult::ConsumedRerender);
        let before = tui.frame_count();
        assert_eq!(
            tui.dispatch_event(mouse_left_down()),
            InputReaction::Rerender
        );
        tui.request_render(false);
        tui.try_render().expect("ConsumedRerender mouse may paint");
        assert!(
            tui.frame_count() > before,
            "ConsumedRerender mouse Down may schedule a frame"
        );
    }
}
