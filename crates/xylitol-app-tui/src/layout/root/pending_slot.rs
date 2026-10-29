use super::super::models_picker::PendingModelChoice;
use super::super::slots::ImportConfirmDecision;

#[derive(Debug, Default)]
pub(super) struct PendingSlotOps {
    pub tree_open: bool,
    pub tree_travel: Option<String>,
    pub tree_fork: Option<String>,
    pub tree_label: Option<(String, Option<String>)>,
    pub model_select: Option<PendingModelChoice>,
    pub scene_select: Option<String>,
    pub theme_select: Option<String>,
    pub capability_toggle: Option<(String, bool)>,
    pub import_decision: Option<ImportConfirmDecision>,
    pub session_resume_select: Option<String>,
    pub session_resume_rename: Option<(String, String)>,
    pub session_resume_delete: Option<String>,
    pub session_popup: bool,
    pub session_sidebar: bool,
}

impl PendingSlotOps {
    pub fn clear_cancelled_on_close(&mut self) {
        self.model_select = None;
        self.scene_select = None;
        self.theme_select = None;
        self.capability_toggle = None;
        self.session_resume_rename = None;
        self.session_resume_delete = None;
    }
}
