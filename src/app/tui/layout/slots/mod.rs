//! Editor-slot state machine (ati18 / c494).
//!
//! Mutually exclusive replacement of the bottom editor zone. Atomic widgets
//! still come from `xylitol_tui`; this enum owns the live face payload.

mod ask;
mod import;
mod mcp;
mod models;
mod themes;
mod tree;

use crate::app::tui::keybindings::matches_binding;

pub use ask::AskSlot;
pub use import::{ImportConfirmDecision, ImportSlot};
pub use mcp::McpSlot;
pub use models::ModelsSlot;
pub use themes::ThemesSlot;
pub use tree::{TreeAction, TreeSlot};

pub(crate) use import::ImportAction;

/// Whether `key` is bound to one of the shared `tui.select.*` navigation ids
/// (up / down / pageUp / pageDown). Single place to add a new shared nav key.
pub(crate) fn is_select_nav_key(key: &crossterm::event::KeyEvent) -> bool {
    matches_binding(key, "tui.select.up")
        || matches_binding(key, "tui.select.down")
        || matches_binding(key, "tui.select.pageUp")
        || matches_binding(key, "tui.select.pageDown")
}
pub(crate) use mcp::McpAction;
pub(crate) use models::ModelsAction;
pub(crate) use themes::ThemesAction;

use crate::app::tui::session_resume::SessionResumePanel;
use strum::EnumMessage as _;

/// Which face occupies the fixed editor slot (scrollback → queue → status → **slot** → footer).
///
/// [`EditorSlotKind`] is generated (`EnumDiscriminants`): new faces add one
/// variant + `strum(message)` here, not a second match table.
#[derive(Default, strum::EnumDiscriminants)]
#[strum_discriminants(name(EditorSlotKind), derive(strum::EnumMessage), vis(pub))]
pub enum EditorSlot {
    #[default]
    #[strum_discriminants(strum(message = "Editor"))]
    Editor,
    #[strum_discriminants(strum(message = "Session tree"))]
    Tree(TreeSlot),
    /// 命令面板 empty shell (MAY; Esc closes).
    #[strum_discriminants(strum(message = "Command Plate"))]
    Plate,
    /// Settings empty shell (MAY; Esc closes).
    #[strum_discriminants(strum(message = "Settings"))]
    Settings,
    #[strum_discriminants(strum(message = "Choice"))]
    Choice(AskSlot),
    #[strum_discriminants(strum(message = "Models"))]
    Models(ModelsSlot),
    #[strum_discriminants(strum(message = "Themes"))]
    Themes(ThemesSlot),
    #[strum_discriminants(strum(message = "Import confirm"))]
    ImportConfirm(ImportSlot),
    #[strum_discriminants(strum(message = "Resume session"))]
    SessionResume(SessionResumePanel),
    #[strum_discriminants(strum(message = "MCP"))]
    Mcp(McpSlot),
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

    /// Any non-Editor face that Esc MUST close first.
    pub fn is_overlay(&self) -> bool {
        self.kind().is_overlay()
    }

    pub fn title(&self) -> &'static str {
        self.kind().title()
    }

    /// Empty Choice shell (harness / `open_slot`); product Ask goes through `AskSlot::mount`.
    pub fn choice_shell() -> Self {
        Self::Choice(AskSlot::empty())
    }

    pub(crate) fn invalidate(&mut self) {
        match self {
            Self::Tree(s) => s.invalidate(),
            Self::Models(s) => s.invalidate(),
            Self::Themes(s) => s.invalidate(),
            Self::ImportConfirm(s) => s.invalidate(),
            Self::SessionResume(s) => s.invalidate(),
            Self::Mcp(s) => s.invalidate(),
            Self::Choice(s) => s.invalidate(),
            Self::Editor | Self::Plate | Self::Settings => {}
        }
    }
}
