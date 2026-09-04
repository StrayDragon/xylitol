//! Fixed-Zone Footprint apply path for [`UiRoot`] (atc23 / c1810).

use super::super::slots::EditorSlot;
use super::super::{
    IMPORT_SLOT, MCP_SLOT_BASE, MODELS_SLOT, RESUME_SLOT, THEMES_SLOT, TREE_SLOT,
    queue_strip_line_count, reserved_lower_fixed_zone, slot_body_budget,
};
use super::UiRoot;

impl UiRoot {
    /// Terminal height for Fixed-Zone Footprint budgets (atc23). Host MUST sync on
    /// construct / resize / before paint.
    pub fn set_term_rows(&mut self, rows: usize) {
        self.term_rows = rows.max(1);
    }

    pub fn term_rows(&self) -> usize {
        self.term_rows
    }

    /// Apply term-aware `max_visible` to the live flex list/tree slot (atc23).
    pub(crate) fn apply_fixed_zone_footprint(&mut self) {
        let queue_lines = queue_strip_line_count(
            self.ui_model.pending_steer.len(),
            self.ui_model.pending_follow_up.len(),
        );
        let toast_present = self.toast_notice.is_some();
        let reserved = reserved_lower_fixed_zone(self.status_busy, queue_lines, toast_present);
        let rows = self.term_rows;

        match &mut self.slot {
            EditorSlot::Models(models) => {
                models.set_max_visible(slot_body_budget(rows, reserved, MODELS_SLOT));
            }
            EditorSlot::Themes(themes) => {
                themes.set_max_visible(slot_body_budget(rows, reserved, THEMES_SLOT));
            }
            EditorSlot::ImportConfirm(imp) => {
                imp.set_max_visible(slot_body_budget(rows, reserved, IMPORT_SLOT));
            }
            EditorSlot::Mcp(mcp) => {
                let mut mcp_slot = MCP_SLOT_BASE;
                mcp_slot.trailer = mcp_slot.trailer.saturating_add(mcp.diag_len());
                mcp.set_max_visible(slot_body_budget(rows, reserved, mcp_slot));
            }
            EditorSlot::SessionResume(panel) => {
                let mut resume_slot = RESUME_SLOT;
                if panel.status_line().is_some() {
                    resume_slot.header = resume_slot.header.saturating_add(1);
                }
                panel.set_max_visible(slot_body_budget(rows, reserved, resume_slot));
            }
            EditorSlot::Tree(tree) => {
                tree.set_max_visible(slot_body_budget(rows, reserved, TREE_SLOT));
            }
            EditorSlot::Editor
            | EditorSlot::Plate
            | EditorSlot::Settings
            | EditorSlot::Choice(_) => {}
        }
    }
}
