//! Builtin `ask` ChoicePrompt payload for [`super::EditorSlot::Choice`].

use std::cell::RefCell;
use std::rc::Rc;

use tokio::sync::oneshot;
use xylitol_tui::{ChoicePrompt, ChoiceQuestion, ChoiceResult, Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::protocol::error::XyToolError;

pub struct AskSlot {
    prompt: Option<ChoicePrompt>,
    pending: Option<Rc<RefCell<Option<ChoiceResult>>>>,
    reply: Option<oneshot::Sender<Result<String, XyToolError>>>,
}

impl AskSlot {
    pub fn empty() -> Self {
        Self {
            prompt: None,
            pending: None,
            reply: None,
        }
    }

    pub fn mount(
        theme: LayoutTheme,
        questions: Vec<ChoiceQuestion>,
        reply: oneshot::Sender<Result<String, XyToolError>>,
    ) -> Self {
        let pending: Rc<RefCell<Option<ChoiceResult>>> = Rc::new(RefCell::new(None));
        let slot = pending.clone();
        let mut prompt_theme = theme.palette().choice_prompt_theme();
        prompt_theme.rail = Some(theme.palette().accent);
        let prompt = ChoicePrompt::new(questions, prompt_theme, move |r| {
            *slot.borrow_mut() = Some(r);
        });
        Self {
            prompt: Some(prompt),
            pending: Some(pending),
            reply: Some(reply),
        }
    }

    pub fn has_prompt(&self) -> bool {
        self.prompt.is_some()
    }

    pub fn abort(&mut self) {
        if let Some(tx) = self.reply.take() {
            let _ = tx.send(Err(XyToolError::Aborted));
        }
        self.prompt = None;
        self.pending = None;
    }

    pub fn handle_esc(&mut self) -> bool {
        if let Some(ref mut prompt) = self.prompt {
            use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            if !prompt.finished() {
                prompt.handle_input(InputEvent::Key(KeyEvent::new(
                    KeyCode::Esc,
                    KeyModifiers::NONE,
                )));
            }
            true
        } else {
            false
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) {
        if let Some(ref mut prompt) = self.prompt {
            prompt.handle_input(event);
        }
    }

    pub fn complete_if_ready(&mut self) -> Option<String> {
        let pending = self.pending.as_ref()?;
        let result = pending.borrow_mut().take()?;
        let json = result.to_ask_payload_json();
        if let Some(tx) = self.reply.take() {
            let _ = tx.send(Ok(json.clone()));
        }
        self.prompt = None;
        self.pending = None;
        Some(json)
    }

    pub fn invalidate(&mut self) {
        if let Some(ref mut prompt) = self.prompt {
            prompt.invalidate();
        }
    }

    pub fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(ref mut prompt) = self.prompt {
            lines.extend(prompt.render(width.max(1)));
        }
        lines
    }
}
