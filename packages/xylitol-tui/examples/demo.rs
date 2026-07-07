use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use xylitol_tui::components::{
    loader::Loader,
    panel::Panel,
    select_list::{SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme},
    spacer::Spacer,
    text::Text,
};
use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions, set_keybindings};
use xylitol_tui::{Component, CrosstermTerminal, TUI};

fn cyan(s: &str) -> String {
    format!("\x1b[36m{}\x1b[39m", s)
}
fn yellow(s: &str) -> String {
    format!("\x1b[33m{}\x1b[39m", s)
}
fn dim(s: &str) -> String {
    format!("\x1b[2m{}\x1b[22m", s)
}
fn blue_bg(s: &str) -> String {
    format!("\x1b[44m\x1b[37m{}\x1b[49m\x1b[39m", s)
}
fn selected_text(s: &str) -> String {
    format!("\x1b[7m{}\x1b[27m", s)
}

macro_rules! quit {
    ($flag:expr) => {
        $flag.store(true, Ordering::SeqCst)
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defs = create_default_definitions();
    set_keybindings(KeybindingsManager::new(defs, HashMap::new()));

    let term = CrosstermTerminal::new()?;
    let mut tui = TUI::new(term);

    let quit_flag = Arc::new(AtomicBool::new(false));
    let qf = quit_flag.clone();

    let demo = DemoApp::new(qf);
    tui.add_child(Box::new(demo));
    tui.set_focus(Some(0));
    tui.start_with_flag(&quit_flag)
}

struct DemoApp {
    header: Panel,
    menu: SelectList,
    status_text: String,
    current_view: View,
    loader: Option<Loader>,
    quit_flag: Arc<AtomicBool>,
}

enum View {
    Menu,
    Weather,
    Settings,
    About,
    Tasks,
}

impl DemoApp {
    fn new(quit_flag: Arc<AtomicBool>) -> Self {
        let mut header = Panel::new(2, 1, Some(Box::new(blue_bg)));
        header.add_child(Box::new(Text::new("  xylitol-tui Demo".into(), 0, 0)));
        header.add_child(Box::new(Text::new(
            "  A Rust terminal UI library — composable, differential rendering".into(),
            0,
            0,
        )));

        let menu_theme = SelectListTheme {
            selected_prefix: Box::new(cyan),
            selected_text: Box::new(selected_text),
            description: Box::new(dim),
            scroll_info: Box::new(dim),
            no_match: Box::new(dim),
        };
        let menu = SelectList::new(
            vec![
                SelectItem::new("weather", "Weather")
                    .with_description("Show current weather (simulated)"),
                SelectItem::new("tasks", "Tasks").with_description("Manage your tasks"),
                SelectItem::new("settings", "Settings").with_description("Configure settings"),
                SelectItem::new("about", "About").with_description("About xylitol-tui"),
                SelectItem::new("quit", "Quit").with_description("Exit the demo"),
            ],
            5,
            menu_theme,
            SelectListLayoutOptions {
                min_primary_column_width: Some(20),
                max_primary_column_width: Some(36),
                truncate_primary: None,
            },
        );

        Self {
            header,
            menu,
            status_text: "  ↑↓ navigate  ↵ select  Esc back  |  xylitol-tui v0.1.0".into(),
            current_view: View::Menu,
            loader: None,
            quit_flag,
        }
    }

    fn weather_view(&self, width: usize) -> Vec<String> {
        let mut panel = Panel::new(2, 1, None);
        panel.add_child(Box::new(Text::new(
            format!("{} Location: San Francisco, CA", yellow("☀")),
            0,
            0,
        )));
        panel.add_child(Box::new(Spacer::new(1)));
        panel.add_child(Box::new(Text::new(
            "Temperature: 18°C  |  Humidity: 72%  |  Wind: 12 km/h".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Text::new(
            "Forecast: Partly cloudy with sunny intervals".into(),
            0,
            0,
        )));
        panel.render(width)
    }

    fn settings_view(&self, width: usize) -> Vec<String> {
        let mut panel = Panel::new(2, 1, None);
        panel.add_child(Box::new(Text::new("  Settings".into(), 0, 0)));
        panel.add_child(Box::new(Spacer::new(1)));
        for (k, v) in &[
            ("Theme", "dark"),
            ("Font Size", "14"),
            ("Auto-save", "enabled"),
            ("Proxy", "none"),
        ] {
            panel.add_child(Box::new(Text::new(
                format!("{} {} : {}", dim("  •"), k, cyan(v)),
                0,
                0,
            )));
        }
        panel.render(width)
    }

    fn tasks_view(&self, width: usize) -> Vec<String> {
        let mut panel = Panel::new(2, 1, None);
        panel.add_child(Box::new(Text::new("  Tasks".into(), 0, 0)));
        panel.add_child(Box::new(Spacer::new(1)));
        for (mark, task) in &[
            ("✓", "Implement SelectList"),
            ("✓", "Add Panel component"),
            ("✓", "Write tests"),
            ("…", "Build demo"),
        ] {
            panel.add_child(Box::new(Text::new(
                format!(
                    "{} {}",
                    if *mark == "✓" {
                        cyan(mark)
                    } else {
                        yellow(mark)
                    },
                    task
                ),
                0,
                0,
            )));
        }
        panel.render(width)
    }

    fn about_view(&self, width: usize) -> Vec<String> {
        let mut panel = Panel::new(2, 1, None);
        panel.add_child(Box::new(Text::new(
            "xylitol-tui — A Rust terminal UI library".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Spacer::new(1)));
        panel.add_child(Box::new(Text::new(
            "Port of pi-tui (TypeScript) to Rust using crossterm. No ratatui.".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Spacer::new(1)));
        panel.add_child(Box::new(Text::new(
            format!(
                "{} {} {} {} {} {} {}",
                cyan("Text"),
                cyan("Panel"),
                cyan("Input"),
                cyan("SelectList"),
                cyan("Loader"),
                cyan("Spacer"),
                cyan("TruncatedText")
            ),
            0,
            0,
        )));
        panel.add_child(Box::new(Text::new("Features: diff rendering, overlay compositing, Kitty keyboard protocol, Emacs bindings, CJK".into(), 0, 0)));
        panel.render(width)
    }
}

impl Component for DemoApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        if let Some(ref mut loader) = self.loader {
            loader.tick();
        }
        let mut lines = self.header.render(width);

        match &self.current_view {
            View::Menu => {
                let mut label = Text::new(dim(" Choose an option:").to_string(), 2, 0);
                lines.extend(label.render(width));
                lines.extend(self.menu.render(width));
                let mut status = Text::new(self.status_text.clone(), 2, 0);
                lines.extend(status.render(width));
            }
            _ => {
                #[allow(clippy::type_complexity)]
                let (title_str, view_fn): (&str, fn(&Self, usize) -> Vec<String>) =
                    match &self.current_view {
                        View::Weather => ("Weather", Self::weather_view),
                        View::Settings => ("Settings", Self::settings_view),
                        View::About => ("About", Self::about_view),
                        View::Tasks => ("Tasks", Self::tasks_view),
                        View::Menu => unreachable!(),
                    };
                let label = format!(" {}", title_str);
                let mut title = Text::new(dim(&label), 2, 0);
                lines.extend(title.render(width));
                lines.extend(view_fn(self, width));
                lines.push(String::new());
                let mut back = Text::new(dim("  ← Press Esc to return").to_string(), 2, 0);
                lines.extend(back.render(width));
            }
        }
        lines
    }

    fn handle_input(&mut self, data: &str) {
        use xylitol_tui::matches_key;

        match &self.current_view {
            View::Menu => {
                if matches_key(data, "ctrl+c") || matches_key(data, "escape") {
                    quit!(self.quit_flag);
                    return;
                }
                if matches_key(data, "enter") {
                    if let Some(item) = self.menu.get_selected_item() {
                        match item.value.as_str() {
                            "quit" => {
                                quit!(self.quit_flag);
                            }
                            "weather" => self.current_view = View::Weather,
                            "settings" => self.current_view = View::Settings,
                            "about" => self.current_view = View::About,
                            "tasks" => self.current_view = View::Tasks,
                            _ => {}
                        }
                    }
                } else {
                    self.menu.handle_input(data);
                }
            }
            _ => {
                if matches_key(data, "escape") || matches_key(data, "ctrl+c") {
                    self.current_view = View::Menu;
                    self.loader = None;
                }
            }
        }
    }

    fn invalidate(&mut self) {
        self.header.invalidate();
        self.menu.invalidate();
    }
}
