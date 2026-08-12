//! Closed product keybinding set for append-only (atao6).

/// Actions that MUST NOT be registered on this surface.
pub const FORBIDDEN_PRODUCT_ACTIONS: &[&str] = &[
    "fold.toggle",
    "ctrl+o.expand",
    "alt+e.expand",
    "fold.triangle",
    "slash.catalog",
    "command.plate",
    "session.tree.double_esc",
    "steer.chrome",
    "follow_up.chrome",
    "spill.open_file",
];

/// Closed set membership check for product action ids.
pub fn is_allowed_product_action(id: &str) -> bool {
    matches!(
        id,
        "abort" | "exit.slash" | "exit.idle_double_ctrl_c" | "scroll.native"
    )
}

/// Parse `/exit` (exact, trimmed). Other slash discovery is not registered.
pub fn is_exit_slash(input: &str) -> bool {
    input.trim() == "/exit"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atao6_closed_set() {
        assert!(is_allowed_product_action("abort"));
        assert!(is_allowed_product_action("exit.slash"));
        assert!(is_allowed_product_action("exit.idle_double_ctrl_c"));
        assert!(is_allowed_product_action("scroll.native"));
        for forbidden in FORBIDDEN_PRODUCT_ACTIONS {
            assert!(
                !is_allowed_product_action(forbidden),
                "forbidden action leaked: {forbidden}"
            );
        }
    }

    #[test]
    fn exit_slash_exact() {
        assert!(is_exit_slash("/exit"));
        assert!(is_exit_slash("  /exit  "));
        assert!(!is_exit_slash("/help"));
        assert!(!is_exit_slash("/exit now"));
    }
}
