//! Shared lag breadcrumbs (`xylitol::lag`) for spinner-freeze diagnosis.
//!
//! Thresholds: ≥80ms → warn (≈1 Loader frame), ≥16ms → info, else debug.
//! Grep via `just obs-tui-lag`.

use std::time::Instant;

const WARN_MS: u128 = 80;
const INFO_MS: u128 = 16;

/// Log elapsed since `start` for `phase` (snake_case verb).
pub fn note(phase: &str, start: Instant) {
    note_detail(phase, start, "");
}

/// Like [`note`] with a short detail suffix (tool count, prompt len, …).
pub fn note_detail(phase: &str, start: Instant, detail: &str) {
    let ms = start.elapsed().as_millis();
    let detail = detail.trim();
    let suffix = if detail.is_empty() {
        String::new()
    } else {
        format!(" {detail}")
    };
    if ms >= WARN_MS {
        log::warn!(target: "xylitol::lag", "{phase} {ms}ms{suffix}");
    } else if ms >= INFO_MS {
        log::info!(target: "xylitol::lag", "{phase} {ms}ms{suffix}");
    } else {
        log::debug!(target: "xylitol::lag", "{phase} {ms}ms{suffix}");
    }
}
