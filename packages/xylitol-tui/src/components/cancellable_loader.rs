use crate::components::loader::{Loader, LoaderIndicatorOptions};
use crate::tui::Component;

/// Loader that can be cancelled with Escape.
pub struct CancellableLoader {
    loader: Loader,
    pub on_abort: Option<Box<dyn FnOnce()>>,
    aborted: bool,
}

impl CancellableLoader {
    pub fn new(
        spinner_color_fn: Box<dyn Fn(&str) -> String>,
        message_color_fn: Box<dyn Fn(&str) -> String>,
        message: String,
        indicator: Option<LoaderIndicatorOptions>,
    ) -> Self {
        Self {
            loader: Loader::new(spinner_color_fn, message_color_fn, message, indicator),
            on_abort: None,
            aborted: false,
        }
    }

    pub fn set_message(&mut self, message: String) {
        self.loader.set_message(message);
    }

    pub fn tick(&mut self) {
        self.loader.tick();
    }

    pub fn interval_ms(&self) -> u64 {
        self.loader.interval_ms()
    }

    pub fn aborted(&self) -> bool {
        self.aborted
    }
}

impl Component for CancellableLoader {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.loader.render(width)
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keybindings::with_keybindings;
        if with_keybindings(|kb| kb.matches(data, "tui.select.cancel")) {
            self.aborted = true;
            if let Some(cb) = self.on_abort.take() {
                cb();
            }
        }
    }

    fn invalidate(&mut self) {
        self.loader.invalidate();
    }
}
