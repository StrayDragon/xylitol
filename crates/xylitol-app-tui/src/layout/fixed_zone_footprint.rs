pub const DEFAULT_MAX_VISIBLE: usize = 10;

pub const STATUS_BUSY_ROWS: usize = 2;
pub const STATUS_IDLE_ROWS: usize = 1;
pub const FOOTER_ROWS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotFixedZoneRows {
    pub header: usize,
    pub trailer: usize,
}

impl SlotFixedZoneRows {
    pub const fn overhead(self) -> usize {
        self.header.saturating_add(self.trailer)
    }
}

pub const MODELS_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 1,
    trailer: 0,
};
pub const SCENES_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 1,
    trailer: 0,
};
pub const THEMES_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 1,
    trailer: 0,
};
pub const IMPORT_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 1,
    trailer: 0,
};
pub const MCP_SLOT_BASE: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 1,
    trailer: 1,
};
pub const CAPABILITIES_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 1,
    trailer: 1,
};
pub const RESUME_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 4,
    trailer: 1,
};
pub const TREE_SLOT: SlotFixedZoneRows = SlotFixedZoneRows {
    header: 4,
    trailer: 1,
};

pub fn queue_strip_line_count(steer: usize, follow_up: usize) -> usize {
    if steer == 0 && follow_up == 0 {
        0
    } else {
        1usize
            .saturating_add(steer)
            .saturating_add(follow_up)
            .saturating_add(1)
    }
}

pub fn reserved_lower_fixed_zone(
    status_busy: bool,
    queue_lines: usize,
    todo_lines: usize,
    toast_present: bool,
    mode_lines: usize,
) -> usize {
    let status = if status_busy {
        STATUS_BUSY_ROWS
    } else {
        STATUS_IDLE_ROWS
    };
    let toast = usize::from(toast_present);
    queue_lines
        .saturating_add(todo_lines)
        .saturating_add(mode_lines)
        .saturating_add(toast)
        .saturating_add(status)
        .saturating_add(FOOTER_ROWS)
}

pub fn slot_body_budget(term_rows: usize, reserved_lower: usize, slot: SlotFixedZoneRows) -> usize {
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
        let reserved = reserved_lower_fixed_zone(true, 0, 0, false, 0);
        assert_eq!(reserved, 3);
        let body = slot_body_budget(16, reserved, RESUME_SLOT);
        assert_eq!(body, 8);
        let slot_total = RESUME_SLOT.overhead() + body;
        assert_eq!(2 + slot_total + 1, 16);
    }

    #[test]
    fn toast_and_queue_shrink_resume_body_so_lower_stack_fits() {
        assert_eq!(queue_strip_line_count(1, 0), 3);
        let reserved = reserved_lower_fixed_zone(true, 3, 0, true, 0);
        assert_eq!(reserved, 7);
        let body = slot_body_budget(16, reserved, RESUME_SLOT);
        assert_eq!(body, 4);
        assert_eq!(3 + 1 + 2 + RESUME_SLOT.overhead() + body + 1, 16);
    }

    #[test]
    fn todo_bar_rows_count_in_lower_reserved() {
        let reserved = reserved_lower_fixed_zone(true, 0, 3, false, 0);
        assert_eq!(reserved, 6);
        let body = slot_body_budget(16, reserved, RESUME_SLOT);
        assert_eq!(body, 5);
    }

    #[test]
    fn mode_strip_counts_one_lower_row() {
        let reserved = reserved_lower_fixed_zone(true, 0, 0, false, 1);
        assert_eq!(reserved, 4);
        let body = slot_body_budget(16, reserved, RESUME_SLOT);
        assert_eq!(body, 7);
    }
}
