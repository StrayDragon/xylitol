//! Property-based tests for the Input component (c405 layer 4, spec tt01).
//!
//! Generates random key sequences and asserts invariants. The editor port
//! (later change) will add stronger invariants (cursor within bounds, undo
//! idempotence, buffer stays valid UTF-8); this change establishes the
//! framework with two minimal invariants on `Input`.

mod support;

use proptest::prelude::*;
use support::vt_feed::feed_vt;

use xylitol_tui::components::input::Input;
use xylitol_tui::tui::Component;

/// The alphabet of keys we fuzz with: printable ASCII, plus the control
/// sequences the Input actually reacts to (Enter, Backspace, arrows). Keeping
/// it small makes failures shrink to minimal cases.
fn key_seq() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            // Printable a-z
            "[a-z]",
            // Enter
            Just("\r".to_string()),
            // Backspace
            Just("\x7f".to_string()),
            // Left / Right arrow (CSI D / CSI C)
            Just("\x1b[D".to_string()),
            Just("\x1b[C".to_string()),
            // Home / End (CSI H / CSI F)
            Just("\x1b[H".to_string()),
            Just("\x1b[F".to_string()),
        ],
        1..50,
    )
    .prop_map(|chunks| chunks.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant 1: no random key sequence (up to 50 keys) may panic the Input.
    #[test]
    fn input_never_panics_on_random_keys(seq in key_seq()) {
        let mut input = Input::new();
        input.set_focused(true);
        feed_vt(&mut input, &seq);
        // Reaching here means no panic. Also assert render doesn't blow up.
        let _ = input.render(40);
    }

    /// Invariant 2: after any key sequence, the rendered output is non-empty
    /// (the prompt renders even for empty input) and the value is valid UTF-8
    /// (a trivial property since Rust strings are UTF-8, but it pins the
    /// contract that handle_input never corrupts internal state).
    #[test]
    fn input_render_always_non_empty(seq in key_seq()) {
        let mut input = Input::new();
        input.set_focused(true);
        feed_vt(&mut input, &seq);
        let lines = input.render(40);
        prop_assert!(
            !lines.is_empty(),
            "Input must always render at least one line (the prompt row)"
        );
        // The value() is a &str (UTF-8 by construction); touching it confirms
        // no internal corruption.
        let _ = input.value();
    }
}
