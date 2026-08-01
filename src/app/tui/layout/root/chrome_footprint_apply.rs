//! Chrome Footprint apply path for [`UiRoot`] (atc23 / c1810).

use super::super::{
    IMPORT_SLOT, MCP_SLOT_BASE, MODELS_SLOT, RESUME_SLOT, THEMES_SLOT, TREE_SLOT,
    queue_strip_line_count, reserved_lower_chrome, slot_body_budget,
};
use super::UiRoot;

impl UiRoot {
    /// Terminal height for Chrome Footprint budgets (atc23). Host MUST sync on
    /// construct / resize / before paint.
    pub fn set_term_rows(&mut self, rows: usize) {
        self.term_rows = rows.max(1);
    }

    pub fn term_rows(&self) -> usize {
        self.term_rows
    }

    /// Apply term-aware `max_visible` to all flex list/tree slots (atc23).
    pub(crate) fn apply_chrome_footprint(&mut self) {
        let queue_lines = queue_strip_line_count(
            self.ui_model.pending_steer.len(),
            self.ui_model.pending_follow_up.len(),
        );
        let toast_present = self.chrome_toast.is_some();
        let reserved = reserved_lower_chrome(self.status_busy, queue_lines, toast_present);

        self.models_list.max_visible = slot_body_budget(self.term_rows, reserved, MODELS_SLOT);
        self.themes_list.max_visible = slot_body_budget(self.term_rows, reserved, THEMES_SLOT);
        self.import_confirm_list.max_visible =
            slot_body_budget(self.term_rows, reserved, IMPORT_SLOT);

        let mut mcp_slot = MCP_SLOT_BASE;
        mcp_slot.trailer = mcp_slot.trailer.saturating_add(self.mcp_diag_lines.len());
        self.mcp_list.max_visible = slot_body_budget(self.term_rows, reserved, mcp_slot);

        let mut resume_slot = RESUME_SLOT;
        if self.session_resume.status_line().is_some() {
            resume_slot.header = resume_slot.header.saturating_add(1);
        }
        self.session_resume.set_max_visible(slot_body_budget(
            self.term_rows,
            reserved,
            resume_slot,
        ));

        self.tree
            .set_max_visible(slot_body_budget(self.term_rows, reserved, TREE_SLOT));
    }
}
