//! UX-level outcome of routing a key event (c399 stage 3).
//!
//! The engine's event router ([`crate::app::tui::engine::tui::Tui::handle_event`])
//! returns `Option<UxOutcome>` so the host loop can act on high-level intents
//! (submit, slash command, abort, quit) without the engine knowing about the
//! agent/driver layer. This keeps the engine free of upward agent/infra imports
//! (arch_guard) — the engine only routes and renders; the host loop translates
//! outcomes into `Driver` calls + `commands::dispatch`.
//!
//! Shape-mirrors the legacy `app::tui::input::InputOutcome` (which stays until
//! stage 4 retires the old `handle(event, &mut TuiApp)` path). The two enums
//! are intentionally distinct so the engine never depends upward.
//!
//! Design note (pi alignment): pi's `TUI.handleInput` returns `void` and
//! drives the host via widget callbacks (`onSubmit`/`onEscape`). The Rust port
//! surfaces the same intents as a return value because Rust widgets can't hold
//! host closures without a borrow cycle (see `widgets/input.rs` module docs).

/// The high-level intent resulting from a routed key event. `None` from
/// `handle_event` means "no actionable outcome this key" (idle / not handled).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UxOutcome {
    /// User pressed submit (Enter) on non-slash input: send it as a prompt.
    Submit(String),
    /// User typed `/cmd` and submitted: route to slash command handling. The
    /// string is the body *after* the leading `/`.
    Slash(String),
    /// Abort the current turn (Ctrl+C-style). The host decides whether to
    /// abort an in-flight stream or quit if idle.
    Abort,
    /// Explicit quit (Ctrl+D-style).
    Quit,
    /// Key was handled by a widget (e.g. a buffer edit) but produced no
    /// app-level intent. Distinct from `None` (not handled at all).
    Idle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcomes_compare_equal() {
        assert_eq!(UxOutcome::Abort, UxOutcome::Abort);
        assert_eq!(
            UxOutcome::Submit("hi".into()),
            UxOutcome::Submit("hi".into())
        );
        assert_ne!(UxOutcome::Abort, UxOutcome::Quit);
    }

    #[test]
    fn submit_and_slash_carry_payload() {
        match UxOutcome::Submit("hello".into()) {
            UxOutcome::Submit(s) => assert_eq!(s, "hello"),
            other => panic!("expected Submit, got {other:?}"),
        }
        match UxOutcome::Slash("model".into()) {
            UxOutcome::Slash(s) => assert_eq!(s, "model"),
            other => panic!("expected Slash, got {other:?}"),
        }
    }

    #[test]
    fn debug_formats_readable() {
        // Debug is used in test panic messages + host-loop tracing.
        assert_eq!(format!("{:?}", UxOutcome::Abort), "Abort");
        assert_eq!(
            format!("{:?}", UxOutcome::Submit("x".into())),
            "Submit(\"x\")"
        );
    }
}
