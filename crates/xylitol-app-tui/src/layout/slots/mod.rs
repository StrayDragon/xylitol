mod ask;
mod capabilities;
mod import;
mod mcp;
mod models;
mod scenes;
mod themes;
mod tree;

use super::theme::LayoutTheme;
use crate::keybindings::matches_binding;

pub use ask::AskSlot;
pub use capabilities::CapabilitiesSlot;
pub use import::{ImportConfirmDecision, ImportSlot};
pub use mcp::McpSlot;
pub use models::ModelsSlot;
pub use scenes::ScenesSlot;
pub use themes::ThemesSlot;
pub use tree::{TreeAction, TreeSlot};

pub(crate) use capabilities::CapabilitiesAction;
pub(crate) use import::ImportAction;

pub(crate) fn is_select_nav_key(key: &crossterm::event::KeyEvent) -> bool {
    matches_binding(key, "tui.select.up")
        || matches_binding(key, "tui.select.down")
        || matches_binding(key, "tui.select.pageUp")
        || matches_binding(key, "tui.select.pageDown")
}
pub(crate) use mcp::McpAction;
pub(crate) use models::ModelsAction;
pub(crate) use scenes::ScenesAction;
pub(crate) use themes::ThemesAction;

use crate::session_resume::SessionResumePanel;
use strum::EnumMessage as _;

#[derive(Default, strum::EnumDiscriminants)]
#[strum_discriminants(name(EditorSlotKind), derive(strum::EnumMessage), vis(pub))]
pub enum EditorSlot {
    #[default]
    #[strum_discriminants(strum(message = "Editor"))]
    Editor,
    #[strum_discriminants(strum(message = "Session tree"))]
    Tree(Box<TreeSlot>),
    #[strum_discriminants(strum(message = "Command Plate"))]
    Plate,
    #[strum_discriminants(strum(message = "Settings"))]
    Settings,
    #[strum_discriminants(strum(message = "Choice"))]
    Choice(AskSlot),
    #[strum_discriminants(strum(message = "Models"))]
    Models(ModelsSlot),
    #[strum_discriminants(strum(message = "Scenes"))]
    Scenes(ScenesSlot),
    #[strum_discriminants(strum(message = "Themes"))]
    Themes(ThemesSlot),
    #[strum_discriminants(strum(message = "Import confirm"))]
    ImportConfirm(ImportSlot),
    #[strum_discriminants(strum(message = "Resume session"))]
    SessionResume(SessionResumePanel),
    #[strum_discriminants(strum(message = "MCP"))]
    Mcp(McpSlot),
    #[strum_discriminants(strum(message = "Capabilities"))]
    Capabilities(CapabilitiesSlot),
}

impl EditorSlotKind {
    pub fn is_editor(self) -> bool {
        matches!(self, Self::Editor)
    }

    pub fn is_tree(self) -> bool {
        matches!(self, Self::Tree)
    }

    pub fn is_overlay(self) -> bool {
        !self.is_editor()
    }

    pub fn title(self) -> &'static str {
        self.get_message()
            .expect("every EditorSlotKind variant has strum(message)")
    }
}

impl EditorSlot {
    pub fn kind(&self) -> EditorSlotKind {
        self.into()
    }

    pub fn is_editor(&self) -> bool {
        self.kind().is_editor()
    }

    pub fn is_tree(&self) -> bool {
        self.kind().is_tree()
    }

    pub fn is_overlay(&self) -> bool {
        self.kind().is_overlay()
    }

    pub fn title(&self) -> &'static str {
        self.kind().title()
    }

    pub fn choice_shell() -> Self {
        Self::Choice(AskSlot::empty())
    }

    pub(crate) fn retheme(&mut self, theme: LayoutTheme) {
        if matches!(self, EditorSlot::SessionResume(_)) {
            *self = EditorSlot::SessionResume(SessionResumePanel::new(theme));
            return;
        }
        match self {
            Self::Tree(tree) => tree.wipe_themed(theme),
            Self::Models(models) => models.retheme(theme),
            Self::Scenes(scenes) => scenes.retheme(theme),
            Self::Themes(themes) => themes.retheme(theme),
            Self::Mcp(mcp) => mcp.retheme(theme),
            Self::Capabilities(caps) => caps.retheme(theme),
            Self::ImportConfirm(imp) => imp.retheme(theme),
            Self::Editor
            | Self::Plate
            | Self::Settings
            | Self::Choice(_)
            | Self::SessionResume(_) => {}
        }
    }

    pub(crate) fn invalidate(&mut self) {
        match self {
            Self::Tree(s) => s.invalidate(),
            Self::Models(s) => s.invalidate(),
            Self::Scenes(s) => s.invalidate(),
            Self::Themes(s) => s.invalidate(),
            Self::ImportConfirm(s) => s.invalidate(),
            Self::SessionResume(s) => s.invalidate(),
            Self::Mcp(s) => s.invalidate(),
            Self::Capabilities(s) => s.invalidate(),
            Self::Choice(s) => s.invalidate(),
            Self::Editor | Self::Plate | Self::Settings => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::session::CapabilitySnapshot;

    #[test]
    fn invalidate_handles_non_tree_slots() {
        let mut editor = EditorSlot::Editor;
        editor.invalidate();
        let mut plate = EditorSlot::Plate;
        plate.invalidate();
        let mut choice = EditorSlot::Choice(AskSlot::empty());
        choice.invalidate();
        let mut caps = EditorSlot::Capabilities(CapabilitiesSlot::mount(
            LayoutTheme::default(),
            &CapabilitySnapshot::default(),
        ));
        caps.invalidate();
    }
}
