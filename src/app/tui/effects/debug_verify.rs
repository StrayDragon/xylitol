//! `/debug verify-smoke` — UI-only product smoke (no real LLM / `/exit`).
//!
//! Covers checklist-style B4/B7 style gates: open models overlay, Esc closes
//! without quit, fixed zone still Ready. Harness owns full B1–B5; this is a hand /
//! slash entry that reuses the same UI mounts.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::tui::harness::esc_event;
use crate::protocol::Command;

use super::super::host::{HostEvent, HostSession, LayoutMode};

/// Run UI-only verify smoke and report via scroll notice.
pub(super) async fn run_verify_smoke<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn crate::app::core::driver::XyDriver,
) {
    let mut report: Vec<String> = Vec::new();
    let mut failed = 0u32;

    let pass = |report: &mut Vec<String>, name: &str| {
        report.push(format!("PASS {name}"));
    };
    let fail = |report: &mut Vec<String>, failed: &mut u32, name: &str, why: &str| {
        *failed += 1;
        report.push(format!("FAIL {name}: {why}"));
    };

    if session.mode() != LayoutMode::Ready {
        fail(
            &mut report,
            &mut failed,
            "ready",
            &format!("mode={:?}", session.mode()),
        );
    } else {
        pass(&mut report, "ready");
    }

    // B4: bare /model opens picker
    match dispatch(driver, Command::GetAvailableModels {}).await {
        Ok(DispatchOutcome::Models(models)) => {
            let current = driver.current_model().map(|m| m.id);
            session.mount_models_picker(models, current, driver.thinking_level());
            let _ = session.render_now();
            let open = session.ui_root().is_some_and(|r| r.borrow().models_open());
            if open {
                pass(&mut report, "B4 /model opens");
            } else {
                fail(
                    &mut report,
                    &mut failed,
                    "B4 /model opens",
                    "models_open=false",
                );
            }
        }
        Ok(_) => fail(
            &mut report,
            &mut failed,
            "B4 /model opens",
            "dispatch returned non-Models",
        ),
        Err(e) => fail(
            &mut report,
            &mut failed,
            "B4 /model opens",
            &format!("dispatch: {e}"),
        ),
    }

    // B7: Esc closes overlay without quit
    let _ = session.step(HostEvent::Input(esc_event()));
    let still_open = session
        .ui_root()
        .is_some_and(|r| r.borrow().models_open() || r.borrow().slot().is_overlay());
    if session.should_quit() {
        fail(
            &mut report,
            &mut failed,
            "B7 Esc no-quit",
            "should_quit after Esc",
        );
    } else if still_open {
        fail(
            &mut report,
            &mut failed,
            "B7 Esc closes overlay",
            "overlay still open",
        );
    } else {
        pass(&mut report, "B7 Esc closes overlay");
        pass(&mut report, "B7 Esc no-quit");
    }

    // B5 is intentionally NOT run here — real `/exit` tears down the TTY.
    report.push("SKIP B5 /exit (use harness h8 or hand `/exit`)".into());
    report.push("SKIP B1–B3 (need Fake/stream; see harness H1–H9)".into());

    let header = if failed == 0 {
        format!("verify-smoke: OK ({} checks logged)", report.len())
    } else {
        format!("verify-smoke: {failed} FAIL")
    };
    let body = std::iter::once(header)
        .chain(report)
        .collect::<Vec<_>>()
        .join("\n");
    session.push_scroll_notice(body);
    let _ = session.render_now();
}
