//! Chrome Footprint — term-aware reserved rows + slot body budget (atc23 / c1810).
//!
//! Keeps xylitol-tui content-end viewport; budgets EditorSlot list/tree `max_visible`
//! so busy status stays in the visible window on short terminals.

/// Soft default when constructing widgets before the first `term_rows` sync.
pub const DEFAULT_MAX_VISIBLE: usize = 10;

/// Busy status: Loader leading blank + spinner row (status.md / atc12).
pub const STATUS_BUSY_ROWS: usize = 2;
/// Idle breathing blank above editor.
pub const STATUS_IDLE_ROWS: usize = 1;
pub const FOOTER_ROWS: usize = 1;

/// Non-body lines for each flex slot (header above body + optional trail below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotChromeRows {
    pub header: usize,
    pub trailer: usize,
}

impl SlotChromeRows {
    pub const fn overhead(self) -> usize {
        self.header.saturating_add(self.trailer)
    }
}

pub const MODELS_SLOT: SlotChromeRows = SlotChromeRows {
    header: 1,
    trailer: 0,
};
pub const THEMES_SLOT: SlotChromeRows = SlotChromeRows {
    header: 1,
    trailer: 0,
};
pub const IMPORT_SLOT: SlotChromeRows = SlotChromeRows {
    header: 1,
    trailer: 0,
};
/// Header summary + Esc trailer; diag lines add to trailer at apply time.
pub const MCP_SLOT_BASE: SlotChromeRows = SlotChromeRows {
    header: 1,
    trailer: 1,
};
/// Scope + help + filter hint + filter input; optional status_line adds to header.
/// Trailer: `(selected/total)` scroll info under the body.
pub const RESUME_SLOT: SlotChromeRows = SlotChromeRows {
    header: 4,
    trailer: 1,
};
/// Title + ~2 help wraps + search; tree scroll_info as trailer.
pub const TREE_SLOT: SlotChromeRows = SlotChromeRows {
    header: 4,
    trailer: 1,
};

/// Lines emitted by [`crate::app::tui::widgets::render_queue_strip`] when queues non-empty.
pub fn queue_strip_line_count(steer: usize, follow_up: usize) -> usize {
    if steer == 0 && follow_up == 0 {
        0
    } else {
        // spacer + messages + Alt+Up hint
        1usize
            .saturating_add(steer)
            .saturating_add(follow_up)
            .saturating_add(1)
    }
}

/// Lower chrome reserved (queue + toast + status + footer). Upper scrollback is not reserved.
pub fn reserved_lower_chrome(status_busy: bool, queue_lines: usize, toast_present: bool) -> usize {
    let status = if status_busy {
        STATUS_BUSY_ROWS
    } else {
        STATUS_IDLE_ROWS
    };
    let toast = usize::from(toast_present);
    queue_lines
        .saturating_add(toast)
        .saturating_add(status)
        .saturating_add(FOOTER_ROWS)
}

/// Body `max_visible` for a flex list/tree slot. Always ≥ 1.
pub fn slot_body_budget(term_rows: usize, reserved_lower: usize, slot: SlotChromeRows) -> usize {
    term_rows
        .saturating_sub(reserved_lower)
        .saturating_sub(slot.overhead())
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_terminal_resume_budget_keeps_status_room() {
        // 16 rows: reserved busy+footer(+no queue/toast)=3; resume overhead=5 → body ≤ 8
        let reserved = reserved_lower_chrome(true, 0, false);
        assert_eq!(reserved, 3);
        let body = slot_body_budget(16, reserved, RESUME_SLOT);
        assert_eq!(body, 8);
        let slot_total = RESUME_SLOT.overhead() + body;
        assert_eq!(2 + slot_total + 1, 16);
    }

    #[test]
    fn toast_and_queue_shrink_resume_body_so_lower_stack_fits() {
        // 1 steer → spacer + msg + hint = 3 queue lines; + toast 1 → reserved 7
        assert_eq!(queue_strip_line_count(1, 0), 3);
        let reserved = reserved_lower_chrome(true, 3, true);
        assert_eq!(reserved, 7);
        let body = slot_body_budget(16, reserved, RESUME_SLOT);
        assert_eq!(body, 4);
        // queue(3)+toast(1)+status(2)+header(4)+body(4)+trailer(1)+footer(1) == 16
        assert_eq!(3 + 1 + 2 + RESUME_SLOT.overhead() + body + 1, 16);
    }
}
