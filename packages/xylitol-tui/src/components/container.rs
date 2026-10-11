use crate::tui::{Component, InputEvent};

/// Vertical stack of child components. Renders by concatenating each child's
/// `render(width)` lines in order. Does **not** fan out keyboard or paste via
/// `handle_input` — focus routing stays with the TUI / host (mirrors pi
/// `Container`). Pointer events are an exception: [`Self::offer_pointer`]
/// forwards to the child whose last-render row span contains the pointer.
pub struct Container {
    children: Vec<Box<dyn Component>>,
    origin: (u16, u16),
    last_child_heights: Vec<usize>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            origin: (0, 0),
            last_child_heights: Vec::new(),
        }
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.children.push(component);
    }

    pub fn remove_child(&mut self, index: usize) {
        if index < self.children.len() {
            self.children.remove(index);
            if index < self.last_child_heights.len() {
                self.last_child_heights.remove(index);
            }
        }
    }

    pub fn clear(&mut self) {
        self.children.clear();
        self.last_child_heights.clear();
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Container {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        self.last_child_heights.clear();
        let mut row = self.origin.1;
        for child in &mut self.children {
            child.set_screen_origin(self.origin.0, row);
            let rendered = child.render(width);
            let h = rendered.len();
            self.last_child_heights.push(h);
            row = row.saturating_add(h as u16);
            lines.extend(rendered);
        }
        lines
    }

    fn handle_input(&mut self, _event: InputEvent) {
        // Intentionally no keyboard/paste fan-out — see r1614 / package-tui-engine.
    }

    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
        self.last_child_heights.clear();
    }

    fn tick(&mut self) -> bool {
        self.children.iter_mut().any(|c| c.tick())
    }

    fn set_screen_origin(&mut self, col: u16, row: u16) {
        self.origin = (col, row);
    }

    fn offer_pointer(&mut self, event: &InputEvent) -> bool {
        let InputEvent::Mouse(mouse) = event else {
            return false;
        };
        if self.last_child_heights.is_empty() {
            return false;
        }
        if mouse.row < self.origin.1 {
            return false;
        }
        let rel = (mouse.row - self.origin.1) as usize;
        let mut acc = 0usize;
        for (i, child) in self.children.iter_mut().enumerate() {
            let h = self.last_child_heights.get(i).copied().unwrap_or(0);
            if rel < acc.saturating_add(h) {
                let child_row = self.origin.1.saturating_add(acc as u16);
                child.set_screen_origin(self.origin.0, child_row);
                return child.offer_pointer(event);
            }
            acc = acc.saturating_add(h);
        }
        false
    }
}
