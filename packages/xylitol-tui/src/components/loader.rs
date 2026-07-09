use crate::components::text::Text;
use crate::tui::Component;

pub struct LoaderIndicatorOptions {
    pub frames: Vec<String>,
    pub interval_ms: u64,
}

impl Default for LoaderIndicatorOptions {
    fn default() -> Self {
        Self {
            frames: vec![
                "⠋".into(),
                "⠙".into(),
                "⠹".into(),
                "⠸".into(),
                "⠼".into(),
                "⠴".into(),
                "⠦".into(),
                "⠧".into(),
                "⠇".into(),
                "⠏".into(),
            ],
            interval_ms: 80,
        }
    }
}

/// Loader component with optional spinning animation.
pub struct Loader {
    text: Text,
    frames: Vec<String>,
    interval_ms: u64,
    current_frame: usize,
    message: String,
    spinner_color_fn: Box<dyn Fn(&str) -> String>,
    message_color_fn: Box<dyn Fn(&str) -> String>,
}

impl Loader {
    pub fn new(
        spinner_color_fn: Box<dyn Fn(&str) -> String>,
        message_color_fn: Box<dyn Fn(&str) -> String>,
        message: String,
        indicator: Option<LoaderIndicatorOptions>,
    ) -> Self {
        let indicator = indicator.unwrap_or_default();
        let frames = if indicator.frames.is_empty() {
            LoaderIndicatorOptions::default().frames
        } else {
            indicator.frames
        };
        let interval_ms = if indicator.interval_ms > 0 {
            indicator.interval_ms
        } else {
            80
        };

        let mut s = Self {
            text: Text::new(String::new(), 1, 0),
            frames,
            interval_ms,
            current_frame: 0,
            message: String::new(),
            spinner_color_fn,
            message_color_fn,
        };
        s.set_message(message);
        s
    }

    pub fn set_message(&mut self, message: String) {
        self.message = message;
        self.update_display();
    }

    pub fn set_indicator(&mut self, indicator: Option<LoaderIndicatorOptions>) {
        let indicator = indicator.unwrap_or_default();
        self.frames = if indicator.frames.is_empty() {
            LoaderIndicatorOptions::default().frames
        } else {
            indicator.frames
        };
        self.interval_ms = if indicator.interval_ms > 0 {
            indicator.interval_ms
        } else {
            80
        };
        self.current_frame = 0;
        self.update_display();
    }

    pub fn tick(&mut self) {
        if self.frames.len() <= 1 {
            return;
        }
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        self.update_display();
    }

    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    fn update_display(&mut self) {
        let frame = self
            .frames
            .get(self.current_frame)
            .cloned()
            .unwrap_or_default();
        if frame.is_empty() {
            self.text.set_text((self.message_color_fn)(&self.message));
        } else {
            let rendered = (self.spinner_color_fn)(&frame);
            let full = format!("{} {}", rendered, (self.message_color_fn)(&self.message));
            self.text.set_text(full);
        }
    }
}

impl Component for Loader {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = vec![String::new()]; // empty top line
        lines.extend(self.text.render(width));
        lines
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}
    fn invalidate(&mut self) {
        self.text.invalidate();
    }
    fn tick(&mut self) -> bool {
        // Advance the spinner frame; signal a re-render so the host repaints.
        // (Only meaningful when more than one frame exists — single-frame
        // loaders are static.)
        if self.frames.len() > 1 {
            self.tick();
            true
        } else {
            false
        }
    }
}
