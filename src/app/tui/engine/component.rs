//! Component contract + Container + Focusable (c399, skill ui-components.md).
//!
//! The widget contract the engine renders. Retained-mode: widgets are long-lived
//! objects with mutable state and a render cache (`Box<dyn Component>`); the
//! engine calls `render(width)` each frame, diffs the resulting line array, and
//! writes only changed lines. State lives on the widget (not externalized to an
//! app struct every frame) — this is pi-tui's mode, the natural fit for the
//! stateful Input/Editor widgets.
//!
//! Shared invariants (all widgets must honor — engine enforces some):
//! 1. **Width**: every `render(width)` line has display width ≤ `width`. The
//!    engine hard-errors on overflow (rendering-engine.md Step 6).
//! 2. **Styles per line**: re-open colors per line; the engine appends a reset
//!    per line (`StyledLine::LINE_RESET`).
//! 3. **No direct terminal writes**: all output is the `render()` return value.
//! 4. **No render scheduling from `render()`**: infinite loop risk.
//! 5. **`invalidate()` clears caches** (including pre-baked themed strings).
//! 6. **`CURSOR_MARKER` only when focused**: the engine strips it for IME.

use crossterm::event::KeyEvent;

use super::outcome::UxOutcome;
use super::style::StyledLine;

/// Result of a widget handling a key: whether it consumed the input, and whether
/// it requests a render. Most widgets return `Handled` (consumed, request render)
/// or `NotHandled` (pass through). Special cases (submit, quit) bubble up via
/// [`Component::take_outcome`], not here.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputResult {
    #[default]
    NotHandled,
    /// Consumed the key; caller should request a render frame.
    Handled,
}

/// A renderable widget. The engine calls `render(width)` each frame; widgets
/// cache by `(content signature, width)` and clear the cache in `invalidate()`.
///
/// `handle_input` is called only when the widget is focused (see [`Focusable`]);
/// non-focusable widgets use the default (NotHandled).
pub trait Component {
    /// Produce this widget's lines at `width`. Each line's display width MUST be
    /// ≤ `width` (the engine hard-errors on overflow).
    fn render(&self, width: usize) -> Vec<StyledLine>;

    /// Handle a key when focused. Default: not handled.
    fn handle_input(&mut self, _key: &KeyEvent) -> InputResult {
        InputResult::NotHandled
    }

    /// Clear cached render state. Called by the engine on theme change and on
    /// `request_render(true)` (force redraw). Widgets that pre-bake themed
    /// strings rebuild them here.
    fn invalidate(&mut self) {}

    /// Optional: whether this widget currently holds focus (and thus should emit
    /// `CURSOR_MARKER` and receive `handle_input`). Default: not focusable.
    fn focused(&self) -> bool {
        false
    }

    /// Toggle focus state. The engine calls this on the focused-widget path.
    /// Default: no-op (non-focusable widgets ignore it). [`Focusable`] widgets
    /// override to actually track the state — this is on `Component` (not just
    /// `Focusable`) so a `Box<dyn Component>` in a [`Container`] can dispatch
    /// without downcasting (Rust trait objects can't cross traits).
    fn set_focused(&mut self, _focused: bool) {}

    /// Drain any pending high-level intent (Submit/Slash/Abort/Quit) produced by
    /// the last `handle_input`. Default: no outcome (leaf widgets that don't
    /// produce app-level intents leave this as `None`). The engine's event
    /// router polls this after routing to surface outcomes to the host loop
    /// without the engine itself depending on the agent layer.
    fn take_outcome(&mut self) -> Option<UxOutcome> {
        None
    }
}

/// Marker trait for a widget that can receive focus and show a text cursor
/// (IME). Implementors emit [`CURSOR_MARKER`](super::style::CURSOR_MARKER) in
/// their `render` output at the on-screen cursor position when `focused()`.
///
/// `set_focused` lives on [`Component`] (not here) so a `Box<dyn Component>` in
/// a [`Container`] can dispatch focus without downcasting. Implement this marker
/// to signal "I honor `set_focused` / `focused` and accept `handle_input` when
/// focused" — it's the engine's cue to route keys here.
///
/// Containers with embedded inputs MUST also implement this marker and forward
/// `set_focused` to the focused child (see [`Container`]'s impl), or the IME
/// candidate window lands in the wrong place.
pub trait Focusable: Component {}

/// A vertical stack of children. The only base-content layout primitive (pi-tui
/// skill ui-components.md Step 1). `render` concatenates each child's line array;
/// `invalidate` propagates.
///
/// **Focus routing**: the engine routes `handle_input` to *one* focused child,
/// not broadcast. `focused_index` identifies which direct child owns focus (or
/// is the ancestor of the focused descendant if that child is itself a
/// `Container`). `set_focused_index` is how the engine points focus at a slot.
/// This avoids the self-referential borrow that a separate `focused: &child`
/// handle would require (pi uses reference semantics; Rust's ownership model
/// can't express "engine owns root + borrows a child" without unsafe/lending).
pub struct Container {
    pub children: Vec<Box<dyn Component>>,
    /// Index of the direct child that holds (or routes to) the focused widget.
    /// `None` = no child has focus. The engine sets this; the container
    /// forwards `handle_input` / `set_focused` / `take_outcome` to this child.
    focused_index: Option<usize>,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            focused_index: None,
        }
    }
    pub fn add(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }
    pub fn clear(&mut self) {
        self.children.clear();
        self.focused_index = None;
    }
    /// Point focus at the child at `idx` (or `None` to clear). The engine calls
    /// this; the container then forwards focus-related calls to that child.
    /// Out-of-range indices are clamped to `None`.
    pub fn set_focused_index(&mut self, idx: Option<usize>) {
        self.focused_index = idx.filter(|i| *i < self.children.len());
    }
    /// Current focused child index (for diagnostics / engine state queries).
    pub fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }
}

impl Component for Container {
    fn render(&self, width: usize) -> Vec<StyledLine> {
        let mut lines = Vec::new();
        for child in &self.children {
            lines.extend(child.render(width));
        }
        lines
    }

    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
    }

    fn handle_input(&mut self, key: &KeyEvent) -> InputResult {
        // Route to the focused child only (not broadcast). No focused child →
        // the container itself doesn't interpret keys (pi model: key semantics
        // live in widgets, the engine/container only routes).
        match self.focused_index.and_then(|i| self.children.get_mut(i)) {
            Some(child) => child.handle_input(key),
            None => InputResult::NotHandled,
        }
    }

    fn focused(&self) -> bool {
        // The container is "focused" iff one of its children is.
        self.focused_index
            .and_then(|i| self.children.get(i))
            .is_some_and(|c| c.focused())
    }

    fn set_focused(&mut self, focused: bool) {
        // Propagate to the focused child (trait doc: containers with embedded
        // inputs MUST forward, or IME lands wrong). No focused_index → noop.
        if let Some(child) = self.focused_index.and_then(|i| self.children.get_mut(i)) {
            child.set_focused(focused);
        }
    }

    fn take_outcome(&mut self) -> Option<UxOutcome> {
        // Forward to the focused child so the engine can drain an outcome that
        // a deeply-nested Input widget produced without downcasting.
        self.focused_index
            .and_then(|i| self.children.get_mut(i))
            .and_then(|c| c.take_outcome())
    }
}

impl Focusable for Container {
    // Marker only — focus forwarding lives on `Component::set_focused` (above)
    // so it's reachable through `Box<dyn Component>` without downcasting.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::style::{CellStyle, Span};
    use crossterm::event::{KeyCode, KeyModifiers};

    /// Minimal leaf widget for testing: emits N copies of a fixed line.
    struct StaticLine {
        text: String,
        rows: usize,
    }
    impl Component for StaticLine {
        fn render(&self, _width: usize) -> Vec<StyledLine> {
            (0..self.rows)
                .map(|_| StyledLine::raw(self.text.clone()))
                .collect()
        }
    }

    /// A focusable leaf that records the keys it handled + whether it's focused,
    /// and optionally holds a pending outcome. Used to exercise Container's
    /// focus-routing + forwarding behavior without pulling in `widgets::input`.
    struct FocusableLeaf {
        focused: bool,
        handled: Vec<KeyEvent>,
        outcome: Option<UxOutcome>,
    }
    impl FocusableLeaf {
        fn new() -> Self {
            Self {
                focused: false,
                handled: Vec::new(),
                outcome: None,
            }
        }
    }
    impl Component for FocusableLeaf {
        fn render(&self, _width: usize) -> Vec<StyledLine> {
            Vec::new()
        }
        fn handle_input(&mut self, key: &KeyEvent) -> InputResult {
            self.handled.push(*key);
            InputResult::Handled
        }
        fn focused(&self) -> bool {
            self.focused
        }
        fn set_focused(&mut self, focused: bool) {
            self.focused = focused;
        }
        fn take_outcome(&mut self) -> Option<UxOutcome> {
            self.outcome.take()
        }
    }
    impl Focusable for FocusableLeaf {}

    #[test]
    fn container_concatenates_children_lines() {
        let mut c = Container::new();
        c.add(Box::new(StaticLine {
            text: "a".into(),
            rows: 2,
        }));
        c.add(Box::new(StaticLine {
            text: "b".into(),
            rows: 1,
        }));
        let lines = c.render(80);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].plain_text(), "a");
        assert_eq!(lines[1].plain_text(), "a");
        assert_eq!(lines[2].plain_text(), "b");
    }

    #[test]
    fn container_invalidate_propagates() {
        // invalidate is a no-op on StaticLine but must not panic and must walk all.
        let mut c = Container::new();
        c.add(Box::new(StaticLine {
            text: "x".into(),
            rows: 1,
        }));
        c.add(Box::new(StaticLine {
            text: "y".into(),
            rows: 1,
        }));
        c.invalidate(); // no panic
        assert_eq!(c.children.len(), 2);
    }

    #[test]
    fn empty_container_renders_no_lines() {
        let c = Container::new();
        assert!(c.render(80).is_empty());
    }

    #[test]
    fn container_clear_removes_children() {
        let mut c = Container::new();
        c.add(Box::new(StaticLine {
            text: "z".into(),
            rows: 1,
        }));
        c.clear();
        assert!(c.render(80).is_empty());
    }

    #[test]
    fn default_input_result_is_not_handled() {
        let mut m = StaticLine {
            text: "x".into(),
            rows: 1,
        };
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let k = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(matches!(m.handle_input(&k), InputResult::NotHandled));
    }

    #[test]
    fn component_default_not_focused() {
        let m = StaticLine {
            text: "x".into(),
            rows: 1,
        };
        assert!(!m.focused());
    }

    #[test]
    fn styled_line_emitted_by_widget() {
        // A widget can emit a styled line; the engine will serialize + diff it.
        let line = StyledLine::from_spans(vec![Span::styled("hi", CellStyle::default().bold())]);
        let m = StaticLine {
            text: String::new(),
            rows: 0,
        };
        let _ = m.render(80); // ensure compiles with the styled line type
        assert_eq!(line.to_ansi(), "\x1b[1mhi");
    }

    // ── Container focus routing (c399 stage 3) ───────────────────────────────

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn container_no_focus_does_not_handle() {
        let mut c = Container::new();
        c.add(Box::new(FocusableLeaf::new()));
        assert_eq!(c.handle_input(&key('a')), InputResult::NotHandled);
    }

    #[test]
    fn container_routes_handle_input_to_focused_child() {
        let mut c = Container::new();
        c.add(Box::new(StaticLine {
            text: "x".into(),
            rows: 1,
        }));
        c.add(Box::new(FocusableLeaf::new()));
        c.set_focused_index(Some(1));
        assert_eq!(c.handle_input(&key('a')), InputResult::Handled);
        // Routing targeted index 1; the static child at 0 has no handle_input.
        // Forward take_outcome also reaches the focused child (none set → None).
        assert!(c.take_outcome().is_none(), "no outcome set yet");
    }

    #[test]
    fn container_set_focused_propagates_to_child() {
        let mut c = Container::new();
        c.add(Box::new(FocusableLeaf::new()));
        c.set_focused_index(Some(0));
        // Toggle focus on via the Component trait method (Box<dyn Component>).
        c.children[0].set_focused(true);
        assert!(c.focused(), "container reports focused when child is");
        c.children[0].set_focused(false);
        assert!(!c.focused());
    }

    #[test]
    fn container_set_focused_via_own_set_focused_forwards() {
        // Container::set_focused forwards to the focused-index child.
        let mut c = Container::new();
        c.add(Box::new(StaticLine {
            text: "a".into(),
            rows: 1,
        }));
        c.add(Box::new(FocusableLeaf::new()));
        c.set_focused_index(Some(1));
        Component::set_focused(&mut c, true);
        assert!(c.focused());
    }

    #[test]
    fn container_take_outcome_drains_focused_child() {
        let mut c = Container::new();
        let mut leaf = FocusableLeaf::new();
        leaf.outcome = Some(UxOutcome::Submit("hi".into()));
        c.add(Box::new(leaf));
        c.set_focused_index(Some(0));
        match c.take_outcome() {
            Some(UxOutcome::Submit(s)) => assert_eq!(s, "hi"),
            other => panic!("expected Submit, got {other:?}"),
        }
        // Drained: second call is None.
        assert!(c.take_outcome().is_none());
    }

    #[test]
    fn container_focused_index_clamps_out_of_range() {
        let mut c = Container::new();
        c.add(Box::new(FocusableLeaf::new()));
        c.set_focused_index(Some(99)); // out of range → None
        assert_eq!(c.focused_index(), None);
        assert_eq!(c.handle_input(&key('a')), InputResult::NotHandled);
    }

    #[test]
    fn container_clear_resets_focus() {
        let mut c = Container::new();
        c.add(Box::new(FocusableLeaf::new()));
        c.set_focused_index(Some(0));
        c.clear();
        assert_eq!(c.focused_index(), None);
    }
}
