//! Pending slot-handshake flags for [`super::UiRoot`] (host drain, not live widgets).

use super::super::models_picker::PendingModelChoice;
use super::super::slots::ImportConfirmDecision;

/// Confirm / open requests that outlive the editor-slot payload until `drain_pending_ui`.
#[derive(Debug, Default)]
pub(super) struct PendingSlotOps {
    pub tree_open: bool,
    pub tree_travel: Option<String>,
    pub tree_fork: Option<String>,
    pub tree_label: Option<(String, Option<String>)>,
    pub model_select: Option<PendingModelChoice>,
    pub theme_select: Option<String>,
    pub import_decision: Option<ImportConfirmDecision>,
    pub session_resume_select: Option<String>,
    pub session_resume_rename: Option<(String, String)>,
    pub session_resume_delete: Option<String>,
}

impl PendingSlotOps {
    /// Overlay close: cancel in-flight picker confirms that MUST NOT fire after Esc.
    pub fn clear_cancelled_on_close(&mut self) {
        self.model_select = None;
        self.theme_select = None;
        self.session_resume_rename = None;
        self.session_resume_delete = None;
    }
}
