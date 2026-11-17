use std::collections::VecDeque;

use super::super::models_picker::PendingModelChoice;
use super::super::slots::ImportConfirmDecision;

#[derive(Debug)]
pub(crate) enum UiIntent {
    TreeOpen,
    TreeTravel(String),
    TreeFork(String),
    TreeLabel(String, Option<String>),
    ModelSelect(PendingModelChoice),
    SceneSelect(String),
    ThemeSelect(String),
    CapabilityToggle(String, bool),
    ImportDecision(ImportConfirmDecision),
    SessionResumeSelect(String),
    SessionResumeRename(String, String),
    SessionResumeDelete(String),
    SessionSidebar,
    McpPromptSelect(String, String),
    McpOAuthStart(String),
    McpResourceAdd(xylitol_plugin_sdk::mcp::McpResourceRef),
}

impl UiIntent {
    pub fn is_ephemeral(&self) -> bool {
        matches!(
            self,
            UiIntent::ModelSelect(_)
                | UiIntent::SceneSelect(_)
                | UiIntent::ThemeSelect(_)
                | UiIntent::CapabilityToggle(..)
                | UiIntent::SessionResumeRename(..)
                | UiIntent::SessionResumeDelete(_)
                | UiIntent::McpPromptSelect(..)
                | UiIntent::McpOAuthStart(_)
                | UiIntent::McpResourceAdd(_)
        )
    }
}

#[derive(Debug, Default)]
pub(crate) struct PendingIntents {
    queue: VecDeque<UiIntent>,
}

impl PendingIntents {
    pub fn request(&mut self, intent: UiIntent) {
        let key = std::mem::discriminant(&intent);
        if let Some(slot) = self
            .queue
            .iter_mut()
            .find(|i| std::mem::discriminant(*i) == key)
        {
            *slot = intent;
        } else {
            self.queue.push_back(intent);
        }
    }

    pub fn cancel(&mut self, pred: impl Fn(&UiIntent) -> bool) {
        self.queue.retain(|i| !pred(i));
    }

    pub fn pop(&mut self) -> Option<UiIntent> {
        self.queue.pop_front()
    }

    pub fn pending_tree_fork(&self) -> Option<String> {
        match self
            .queue
            .iter()
            .find(|i| matches!(i, UiIntent::TreeFork(_)))
        {
            Some(UiIntent::TreeFork(id)) => Some(id.clone()),
            _ => None,
        }
    }

    pub fn pending_tree_label(&self) -> Option<(String, Option<String>)> {
        match self
            .queue
            .iter()
            .find(|i| matches!(i, UiIntent::TreeLabel(_, _)))
        {
            Some(UiIntent::TreeLabel(id, label)) => Some((id.clone(), label.clone())),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn has_session_sidebar(&self) -> bool {
        self.queue
            .iter()
            .any(|i| matches!(i, UiIntent::SessionSidebar))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_replaces_pending_intent_of_same_kind() {
        let mut intents = PendingIntents::default();
        intents.request(UiIntent::TreeFork("a".into()));
        intents.request(UiIntent::TreeFork("b".into()));
        intents.request(UiIntent::TreeOpen);
        assert!(matches!(intents.pop(), Some(UiIntent::TreeFork(id)) if id == "b"));
        assert!(matches!(intents.pop(), Some(UiIntent::TreeOpen)));
        assert!(intents.pop().is_none());
    }

    #[test]
    fn pop_consumes_intent_once() {
        let mut intents = PendingIntents::default();
        intents.request(UiIntent::SessionSidebar);
        assert!(matches!(intents.pop(), Some(UiIntent::SessionSidebar)));
        assert!(intents.pop().is_none());
    }

    #[test]
    fn cancel_ephemeral_keeps_tree_import_and_select() {
        let mut intents = PendingIntents::default();
        intents.request(UiIntent::TreeTravel("t".into()));
        intents.request(UiIntent::TreeOpen);
        intents.request(UiIntent::ImportDecision(ImportConfirmDecision::Rejected));
        intents.request(UiIntent::SessionResumeSelect("s".into()));
        intents.request(UiIntent::SessionSidebar);
        intents.request(UiIntent::ModelSelect(PendingModelChoice {
            model_id: String::new(),
            thinking: String::new(),
        }));
        intents.request(UiIntent::ThemeSelect("dark".into()));
        intents.request(UiIntent::CapabilityToggle("mcp".into(), true));
        intents.request(UiIntent::SessionResumeRename("s".into(), "n".into()));
        intents.request(UiIntent::SessionResumeDelete("s".into()));
        intents.request(UiIntent::McpPromptSelect("srv".into(), "p".into()));
        intents.request(UiIntent::McpOAuthStart("srv".into()));
        intents.request(UiIntent::McpResourceAdd(Default::default()));
        intents.request(UiIntent::SceneSelect("code".into()));
        intents.cancel(UiIntent::is_ephemeral);
        assert!(matches!(intents.pop(), Some(UiIntent::TreeTravel(id)) if id == "t"));
        assert!(matches!(intents.pop(), Some(UiIntent::TreeOpen)));
        assert!(matches!(
            intents.pop(),
            Some(UiIntent::ImportDecision(ImportConfirmDecision::Rejected))
        ));
        assert!(matches!(intents.pop(), Some(UiIntent::SessionResumeSelect(id)) if id == "s"));
        assert!(matches!(intents.pop(), Some(UiIntent::SessionSidebar)));
        assert!(intents.pop().is_none());
    }
}
