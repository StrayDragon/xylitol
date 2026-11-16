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

    fn take_matching(&mut self, pred: impl Fn(&UiIntent) -> bool) -> Option<UiIntent> {
        let index = self.queue.iter().position(pred)?;
        self.queue.remove(index)
    }

    pub fn take_tree_open(&mut self) -> bool {
        self.take_matching(|i| matches!(i, UiIntent::TreeOpen))
            .is_some()
    }

    pub fn take_session_sidebar(&mut self) -> bool {
        self.take_matching(|i| matches!(i, UiIntent::SessionSidebar))
            .is_some()
    }

    #[cfg(test)]
    pub fn has_session_sidebar(&self) -> bool {
        self.queue
            .iter()
            .any(|i| matches!(i, UiIntent::SessionSidebar))
    }

    pub fn take_tree_travel(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::TreeTravel(_))) {
            Some(UiIntent::TreeTravel(id)) => Some(id),
            _ => None,
        }
    }

    pub fn take_tree_fork(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::TreeFork(_))) {
            Some(UiIntent::TreeFork(id)) => Some(id),
            _ => None,
        }
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

    pub fn take_tree_label(&mut self) -> Option<(String, Option<String>)> {
        match self.take_matching(|i| matches!(i, UiIntent::TreeLabel(_, _))) {
            Some(UiIntent::TreeLabel(id, label)) => Some((id, label)),
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

    pub fn take_model_select(&mut self) -> Option<PendingModelChoice> {
        match self.take_matching(|i| matches!(i, UiIntent::ModelSelect(_))) {
            Some(UiIntent::ModelSelect(choice)) => Some(choice),
            _ => None,
        }
    }

    pub fn take_scene_select(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::SceneSelect(_))) {
            Some(UiIntent::SceneSelect(id)) => Some(id),
            _ => None,
        }
    }

    pub fn take_theme_select(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::ThemeSelect(_))) {
            Some(UiIntent::ThemeSelect(name)) => Some(name),
            _ => None,
        }
    }

    pub fn take_capability_toggle(&mut self) -> Option<(String, bool)> {
        match self.take_matching(|i| matches!(i, UiIntent::CapabilityToggle(_, _))) {
            Some(UiIntent::CapabilityToggle(id, enabled)) => Some((id, enabled)),
            _ => None,
        }
    }

    pub fn take_import_decision(&mut self) -> Option<ImportConfirmDecision> {
        match self.take_matching(|i| matches!(i, UiIntent::ImportDecision(_))) {
            Some(UiIntent::ImportDecision(decision)) => Some(decision),
            _ => None,
        }
    }

    pub fn take_session_resume_select(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::SessionResumeSelect(_))) {
            Some(UiIntent::SessionResumeSelect(id)) => Some(id),
            _ => None,
        }
    }

    pub fn take_session_resume_rename(&mut self) -> Option<(String, String)> {
        match self.take_matching(|i| matches!(i, UiIntent::SessionResumeRename(_, _))) {
            Some(UiIntent::SessionResumeRename(id, name)) => Some((id, name)),
            _ => None,
        }
    }

    pub fn take_session_resume_delete(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::SessionResumeDelete(_))) {
            Some(UiIntent::SessionResumeDelete(id)) => Some(id),
            _ => None,
        }
    }

    pub fn take_mcp_prompt_select(&mut self) -> Option<(String, String)> {
        match self.take_matching(|i| matches!(i, UiIntent::McpPromptSelect(_, _))) {
            Some(UiIntent::McpPromptSelect(server, name)) => Some((server, name)),
            _ => None,
        }
    }

    pub fn take_mcp_oauth_start(&mut self) -> Option<String> {
        match self.take_matching(|i| matches!(i, UiIntent::McpOAuthStart(_))) {
            Some(UiIntent::McpOAuthStart(server)) => Some(server),
            _ => None,
        }
    }

    pub fn take_mcp_resource_add(&mut self) -> Option<xylitol_plugin_sdk::mcp::McpResourceRef> {
        match self.take_matching(|i| matches!(i, UiIntent::McpResourceAdd(_))) {
            Some(UiIntent::McpResourceAdd(resource)) => Some(resource),
            _ => None,
        }
    }

    pub fn clear_cancelled_on_close(&mut self) {
        self.queue.retain(|i| {
            !matches!(
                i,
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
        });
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
        assert_eq!(intents.take_tree_fork().as_deref(), Some("b"));
        assert!(intents.take_tree_fork().is_none());
        assert!(intents.take_tree_open());
    }

    #[test]
    fn take_consumes_intent_once() {
        let mut intents = PendingIntents::default();
        intents.request(UiIntent::SessionSidebar);
        assert!(intents.take_session_sidebar());
        assert!(!intents.take_session_sidebar());
    }

    #[test]
    fn clear_cancelled_on_close_keeps_tree_import_and_select() {
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
        intents.clear_cancelled_on_close();
        assert!(intents.take_tree_open());
        assert_eq!(intents.take_tree_travel().as_deref(), Some("t"));
        assert!(intents.take_import_decision().is_some());
        assert_eq!(intents.take_session_resume_select().as_deref(), Some("s"));
        assert!(intents.take_session_sidebar());
        assert!(intents.take_model_select().is_none());
        assert!(intents.take_theme_select().is_none());
        assert!(intents.take_capability_toggle().is_none());
        assert!(intents.take_session_resume_rename().is_none());
        assert!(intents.take_session_resume_delete().is_none());
        assert!(intents.take_mcp_prompt_select().is_none());
        assert!(intents.take_mcp_oauth_start().is_none());
        assert!(intents.take_mcp_resource_add().is_none());
        assert!(intents.take_scene_select().is_none());
    }
}
