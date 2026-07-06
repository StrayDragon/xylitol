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

use super::style::StyledLine;

/// Result of a widget handling a key: whether it consumed the input, and whether
/// it requests a render. Most widgets return `Handled` (consumed, request render)
/// or `NotHandled` (pass through). Special cases (submit, quit) bubble up via
/// the UX layer's routing, not here.
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
}

/// A widget that can receive focus and show a text cursor (IME). Implementors
/// emit [`CURSOR_MARKER`](super::style::CURSOR_MARKER) in their `render` output
/// at the on-screen cursor position when `focused()`. The engine scans for the
/// marker, strips it, and positions the hardware cursor there.
///
/// Containers with embedded inputs MUST propagate focus to the child (override
/// `set_focused` to forward) or the IME candidate window lands in the wrong place.
pub trait Focusable: Component {
    /// Set focus state. The engine toggles this via `set_focus`.
    fn set_focused(&mut self, focused: bool);
}

/// A vertical stack of children. The only base-content layout primitive (pi-tui
/// skill ui-components.md Step 1). `render` concatenates each child's line array;
/// `invalidate` propagates; `handle_input` is routed by the engine to the focused
/// child, not broadcast.
pub struct Container {
    pub children: Vec<Box<dyn Component>>,
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
        }
    }
    pub fn add(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }
    pub fn clear(&mut self) {
        self.children.clear();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::style::{CellStyle, Span};

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
}
