//! Minimal ApplicationOwned host loop — canonical ptim14 surface (c2070 §7.9).
//!
//! Run: `cargo run -p xylitol-tui --example host_loop_application_owned`
//!   or: `just demo-tui-host-loop`
//!
//! Demonstrates the reusable path (no demo-private mouse glue):
//! [`ApplicationOwnedTui`] → begin → dispatch / idle / render → finish.

use std::io::{Write, stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use xylitol_tui::{
    ApplicationOwnedTui, Component, CrosstermTerminal, InputEvent, InputReaction, Terminal,
    editor_screen_origin,
};

struct HelloHost {
    lines: Vec<String>,
}

impl HelloHost {
    fn new() -> Self {
        Self {
            lines: vec![
                "ApplicationOwned host loop (ptim14)".into(),
                "Type characters; Ctrl+C or q to quit.".into(),
                String::new(),
                "> ".into(),
            ],
        }
    }
}

impl Component for HelloHost {
    fn render(&mut self, width: usize) -> Vec<String> {
        let dock = 2usize;
        // Keep origin helper exercised (hosts sync Editor the same way).
        let term_rows = self.lines.len().max(dock) as u16;
        let _origin = editor_screen_origin(term_rows.max(dock as u16), dock, 1);
        let _ = width;
        self.lines.clone()
    }

    fn handle_input(&mut self, event: InputEvent) {
        let InputEvent::Key(key) = event else {
            return;
        };
        if let KeyCode::Char(c) = key.code
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            if let Some(last) = self.lines.last_mut() {
                last.push(c);
            }
        } else if key.code == KeyCode::Backspace
            && let Some(last) = self.lines.last_mut()
            && last.len() > 2
        {
            last.pop();
        }
    }

    fn input_wants_rerender(&self, _event: &InputEvent) -> bool {
        true
    }

    fn invalidate(&mut self) {}

    fn dock_rows_hint(&self) -> Option<usize> {
        Some(2)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = stdout();
    writeln!(
        out,
        "host_loop_application_owned — press q or Ctrl+C to exit (session stays in main scrollback)"
    )?;
    out.flush()?;

    let term = CrosstermTerminal::new()?;
    let mut tui = ApplicationOwnedTui::new(term);
    tui.terminal.start();
    tui.begin();
    tui.add_child(Box::new(HelloHost::new()));
    tui.set_focus(Some(0));
    tui.set_dock_rows(2);
    tui.request_render(true);
    let _ = tui.render_now();

    loop {
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
                        break;
                    }
                    let reaction = tui.dispatch_event(InputEvent::Key(key));
                    if reaction == InputReaction::Rerender {
                        let _ = tui.render_now();
                    }
                }
                Event::Resize(_, _) => {
                    tui.terminal.refresh_size();
                    let _ = tui.render_now();
                }
                Event::Mouse(mouse) => {
                    let _ = tui.dispatch_event(InputEvent::Mouse(mouse));
                    if tui.idle_tick() {
                        let _ = tui.try_render();
                    }
                }
                _ => {}
            }
        } else if tui.idle_tick() {
            let _ = tui.try_render();
        }
    }

    tui.finish();
    Ok(())
}
