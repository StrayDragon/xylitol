//! Editor-slot state machine (ati18 / c494).
//!
//! Mutually exclusive replacement of the bottom editor zone. Atomic widgets
//! still come from `xylitol_tui`; this enum only names which face is mounted.

/// Which face occupies the fixed editor slot (scrollback → queue → status → **slot** → footer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorSlot {
    #[default]
    Editor,
    /// c615 MessageHistory live tree (Driver `session_tree` + `travel_session_tree`).
    Tree,
    /// Command plate empty shell (MAY; Esc closes).
    Plate,
    /// Settings empty shell (MAY; Esc closes).
    Settings,
    /// Ask / ChoicePrompt empty shell (MAY; Esc closes).
    Choice,
    /// Fuzzy model picker (`/model`, c630).
    Models,
    /// `/session-import` Yes/No confirm (c1010; not Trust Choice stub).
    ImportConfirm,
    /// `/session-resume` session picker (c1015).
    SessionResume,
}

impl EditorSlot {
    pub fn is_editor(self) -> bool {
        matches!(self, Self::Editor)
    }

    pub fn is_tree(self) -> bool {
        matches!(self, Self::Tree)
    }

    /// Any non-Editor face that Esc MUST close first.
    pub fn is_overlay(self) -> bool {
        !self.is_editor()
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Editor => "Editor",
            Self::Tree => "Session tree",
            Self::Plate => "Command Plate",
            Self::Settings => "Settings",
            Self::Choice => "Choice",
            Self::Models => "Models",
            Self::ImportConfirm => "Import confirm",
            Self::SessionResume => "Resume session",
        }
    }
}
