use crate::tui::Component;

/// Spacer component that renders empty lines.
pub struct Spacer {
    lines: usize,
}

impl Spacer {
    pub fn new(lines: usize) -> Self {
        Self { lines }
    }

    pub fn set_lines(&mut self, lines: usize) {
        self.lines = lines;
    }
}

impl Component for Spacer {
    fn render(&mut self, _width: usize) -> Vec<String> {
        vec![String::new(); self.lines]
    }

    fn handle_input(&mut self, _data: &str) {}

    fn invalidate(&mut self) {}
}
