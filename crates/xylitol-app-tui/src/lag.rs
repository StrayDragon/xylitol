use std::time::Instant;

const WARN_MS: u128 = 80;
const INFO_MS: u128 = 16;

pub fn note(phase: &str, start: Instant) {
    note_detail(phase, start, "");
}

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
