use crate::components::loader::{Loader, LoaderIndicatorOptions};
use crate::tui::Component;

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
        if self.aborted {
            return;
        }
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
        if self.aborted {
            return Vec::new();
        }
        self.loader.render(width)
    }

    fn handle_input(&mut self, event: crate::tui::InputEvent) {
        use crate::keybindings::with_keybindings;
        use crate::tui::InputEvent;
        let InputEvent::Key(ref key) = event else {
            return;
        };
        if with_keybindings(|kb| kb.matches_event(key, "tui.select.cancel")) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::InputEvent;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn loader() -> CancellableLoader {
        CancellableLoader::new(
            Box::new(|s| s.to_string()),
            Box::new(|s| s.to_string()),
            "loading".into(),
            None,
        )
    }

    fn esc() -> InputEvent {
        InputEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
    }

    #[test]
    fn render_and_tick_are_gated_after_abort() {
        let mut l = loader();
        assert!(!l.render(40).is_empty());
        assert!(!l.aborted());
        l.handle_input(esc());
        assert!(l.aborted());
        assert!(l.render(40).is_empty(), "aborted loader must not paint");
        l.tick();
        assert!(l.render(40).is_empty(), "tick must stay gated after abort");
    }
}
