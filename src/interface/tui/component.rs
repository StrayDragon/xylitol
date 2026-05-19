//! Core component abstractions for the TUI.
//!
//! The TUI is built from a small set of stateful components that:
//! - Render into a rect (`render`)
//! - Track whether they need redraw (`is_dirty` / `mark_clean`)
//! - React to events (`handle_event`)

use ratatui::Frame;
use ratatui::layout::Rect;

use super::event::{AppAction, TuiEvent};

/// Result of handling an event in a component.
#[derive(Debug, Default)]
pub(crate) struct EventResult {
    /// Whether the event was consumed and should stop routing.
    pub(crate) consumed: bool,
    /// Optional high-level action requested by this component.
    pub(crate) action: Option<AppAction>,
    /// Optional overlay action (e.g. dismiss).
    pub(crate) overlay: Option<OverlayAction>,
}

impl EventResult {
    pub(crate) fn consumed() -> Self {
        Self {
            consumed: true,
            ..Self::default()
        }
    }

    pub(crate) fn action(action: AppAction) -> Self {
        Self {
            consumed: true,
            action: Some(action),
            ..Self::default()
        }
    }
}

/// Overlay-specific actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayAction {
    Dismiss,
}

/// Base trait for all renderable TUI components.
pub(crate) trait Component {
    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn is_dirty(&self) -> bool;

    fn mark_clean(&mut self);

    fn handle_event(&mut self, event: &TuiEvent) -> EventResult;
}

/// Z-ordered stack of overlays (modals).
///
/// - Rendering: bottom → top in insertion order.
/// - Input routing: top → bottom until a layer consumes the event.
pub(crate) struct OverlayStack {
    layers: Vec<Box<dyn Component>>,
}

impl OverlayStack {
    pub(crate) fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.layers.len()
    }

    pub(crate) fn push(&mut self, layer: Box<dyn Component>) {
        self.layers.push(layer);
    }

    pub(crate) fn pop(&mut self) -> Option<Box<dyn Component>> {
        self.layers.pop()
    }

    /// Route an event through the overlay stack (top → bottom).
    ///
    /// Returns the first consumed `EventResult`, or `None` when unconsumed.
    /// If the top overlay requests dismissal, it is popped automatically.
    pub(crate) fn route_event(&mut self, event: &TuiEvent) -> Option<EventResult> {
        let top_idx = self.layers.len().saturating_sub(1);
        for idx in (0..self.layers.len()).rev() {
            let result = self.layers[idx].handle_event(event);
            if result.consumed {
                if idx == top_idx && result.overlay == Some(OverlayAction::Dismiss) {
                    self.layers.pop();
                }
                return Some(result);
            }
        }
        None
    }

    /// Render all overlays (bottom → top).
    pub(crate) fn render_all(&mut self, frame: &mut Frame, area: Rect) {
        for layer in &mut self.layers {
            layer.render(frame, area);
        }
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}
