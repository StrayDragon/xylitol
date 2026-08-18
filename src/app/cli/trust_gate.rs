//! Pre-bootstrap trust ChoicePrompt gate (c490).
//!
//! Runs in raw mode **before** [`crate::app::core::bootstrap::bootstrap`] so
//! project resources load after the store decision. Product TUI MUST NOT use
//! `prompt_trust_options_stdio`.
//!
//! Esc / Ctrl+C: cancel without writing `trust.json`, restore the terminal,
//! and return [`TrustGateError::Cancelled`] so the CLI exits (no product TUI).
//!
//! Enter on **Do not trust**: write deny to the store, then
//! [`TrustGateError::Denied`] — CLI exits without starting the product TUI.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crossterm::event::{Event, KeyEventKind};
use xylitol_tui::{
    ChoiceMode, ChoiceOption, ChoicePrompt, ChoiceQuestion, ChoiceResult, Component, InputEvent,
    Palette, TUI, Terminal,
};

use crate::app::tui::terminal_guard::{TerminalGuard, exit_requested, install_lifecycle_hooks};
use crate::infra::trust::{TrustManager, TrustOption, format_trust_prompt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustGateResult {
    Selected(usize),
    Cancelled,
}

/// Soft exit — caller MUST NOT start the product TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustGateError {
    /// Esc / Ctrl+C — store untouched.
    Cancelled,
    /// Enter on Do not trust — deny persisted.
    Denied,
    Other(String),
}

impl std::fmt::Display for TrustGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(
                f,
                "Project not trusted (trust prompt cancelled); skipping `.xylitol/` project resources."
            ),
            Self::Denied => write!(
                f,
                "Project not trusted (denied in trust store); skipping `.xylitol/` project resources."
            ),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<String> for TrustGateError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

/// Returns trust options when an interactive Ask is required; `None` if already decided.
pub fn pending_trust_options(
    manager: &TrustManager,
    cwd: &str,
    trust_override: Option<bool>,
) -> Option<Vec<TrustOption>> {
    if trust_override.is_some() {
        return None;
    }
    if !manager.has_trust_inputs(cwd) {
        return None;
    }
    if manager.is_project_trusted(cwd).is_some() {
        return None;
    }
    Some(manager.get_trust_options(cwd, false))
}

/// If Ask is needed, open a ChoicePrompt TUI, persist an Enter decision, then restore.
///
/// - Cancel (Esc / Ctrl+C) does **not** write the trust store → [`TrustGateError::Cancelled`].
/// - Enter on an untrusted option writes deny → [`TrustGateError::Denied`] (no product TUI).
/// - Enter on a trusted option writes trust → `Ok(())` (caller may start TUI).
pub fn run_trust_gate_if_needed(trust_override: Option<bool>) -> Result<(), TrustGateError> {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".into());
    let manager = TrustManager::new(TrustManager::default_dir());
    let Some(options) = pending_trust_options(&manager, &cwd, trust_override) else {
        return Ok(());
    };
    if options.is_empty() {
        return Ok(());
    }

    let result = run_choice_ui(&cwd, &options)?;
    match result {
        TrustGateResult::Selected(idx) => {
            let Some(opt) = options.get(idx) else {
                return Ok(());
            };
            if !opt.updates.is_empty() {
                manager
                    .apply_updates(&opt.updates)
                    .map_err(|e| TrustGateError::Other(format!("trust store write failed: {e}")))?;
            }
            if opt.trusted {
                Ok(())
            } else {
                Err(TrustGateError::Denied)
            }
        }
        TrustGateResult::Cancelled => Err(TrustGateError::Cancelled),
    }
}

fn run_choice_ui(cwd: &str, options: &[TrustOption]) -> Result<TrustGateResult, TrustGateError> {
    install_lifecycle_hooks();
    let guard = TerminalGuard::enter().map_err(|e| TrustGateError::Other(e.to_string()))?;
    let terminal = guard.take();

    let done: Rc<RefCell<Option<ChoiceResult>>> = Rc::new(RefCell::new(None));
    let done_cb = done.clone();

    let choice_options: Vec<ChoiceOption> = options
        .iter()
        .enumerate()
        .map(|(i, o)| ChoiceOption::new(i.to_string(), o.label.clone()))
        .collect();

    let question = ChoiceQuestion {
        id: "trust".into(),
        label: "Trust".into(),
        // Multi-line body is rendered as separate rows by ChoicePrompt.
        prompt: format_trust_prompt(cwd),
        mode: ChoiceMode::Single,
        options: choice_options,
        allow_other: false,
    };

    let prompt = ChoicePrompt::new(
        vec![question],
        Palette::dark().choice_prompt_theme(),
        move |r| {
            *done_cb.borrow_mut() = Some(r);
        },
    );

    let mut tui = TUI::new(terminal);
    tui.add_child(Box::new(TrustGateView {
        title: "Project trust".into(),
        prompt,
    }));
    tui.set_focus(Some(0));
    tui.render_now()
        .map_err(|e| TrustGateError::Other(e.to_string()))?;

    let outcome = loop {
        if exit_requested() {
            break TrustGateResult::Cancelled;
        }
        if let Some(result) = done.borrow_mut().take() {
            break if result.cancelled {
                TrustGateResult::Cancelled
            } else {
                let idx = result
                    .answers
                    .first()
                    .and_then(|a| a.values.first())
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                TrustGateResult::Selected(idx)
            };
        }

        if crossterm::event::poll(Duration::from_millis(33)).unwrap_or(false) {
            match crossterm::event::read() {
                Ok(Event::Key(key)) => {
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                        tui.dispatch_event(InputEvent::Key(key));
                        tui.request_render(false);
                        let _ = tui.try_render();
                    }
                }
                Ok(Event::Resize(cols, rows)) => {
                    tui.terminal.set_size_hint(cols, rows);
                    tui.terminal.refresh_size();
                    tui.request_render(true);
                    let _ = tui.try_render();
                }
                Ok(Event::Paste(data)) => {
                    tui.dispatch_event(InputEvent::Paste(data));
                    tui.request_render(false);
                    let _ = tui.try_render();
                }
                Ok(_) => {}
                Err(e) => {
                    tui.finish_inline();
                    return Err(TrustGateError::Other(format!(
                        "trust gate input error: {e}"
                    )));
                }
            }
        }
    };

    tui.finish_inline();
    Ok(outcome)
}

struct TrustGateView {
    title: String,
    prompt: ChoicePrompt,
}

impl Component for TrustGateView {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(self.title.clone());
        lines.push(String::new());
        lines.extend(self.prompt.render(width.max(1)));
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.prompt.handle_input(event);
    }

    fn invalidate(&mut self) {
        self.prompt.invalidate();
    }

    fn tick(&mut self) -> bool {
        self.prompt.tick()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pending_none_without_trust_inputs() {
        let dir = tempdir().unwrap();
        let mgr = TrustManager::new(dir.path().to_path_buf());
        let cwd = dir.path().to_string_lossy().to_string();
        assert!(pending_trust_options(&mgr, &cwd, None).is_none());
    }

    #[test]
    fn pending_none_with_override() {
        let dir = tempdir().unwrap();
        let mgr = TrustManager::new(dir.path().to_path_buf());
        // Even with a fake path, override short-circuits.
        assert!(pending_trust_options(&mgr, "/tmp", Some(true)).is_none());
    }

    #[test]
    fn cancelled_display_matches_exit_message() {
        let msg = TrustGateError::Cancelled.to_string();
        assert!(msg.contains("trust prompt cancelled"));
        assert!(msg.contains("skipping"));
    }

    #[test]
    fn denied_display_matches_store_message() {
        let msg = TrustGateError::Denied.to_string();
        assert!(msg.contains("denied in trust store"));
        assert!(msg.contains("skipping"));
    }
}
