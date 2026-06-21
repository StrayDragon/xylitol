//! Startup timing instrumentation — gated behind XYLITOL_TIMING=1 env var.
//!
//! Aligns with pi's timings.ts.

use std::time::Instant;

/// Whether timing instrumentation is enabled.
fn is_enabled() -> bool {
    std::env::var("XYLITOL_TIMING")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Global timing state.
struct TimingState {
    timings: Vec<TimingEntry>,
    last_time: Instant,
}

#[derive(Clone)]
struct TimingEntry {
    label: &'static str,
    ms: u64,
}

// Use std::sync::OnceLock for lazy initialization
use std::sync::Mutex;

static STATE: Mutex<Option<TimingState>> = Mutex::new(None);

/// Reset all timings and start a new timing window.
///
/// Safe to call multiple times — reinitializes the state.
pub fn reset_timings() {
    if !is_enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        *state = Some(TimingState {
            timings: Vec::new(),
            last_time: Instant::now(),
        });
    }
}

/// Record a timing point.
pub fn time(label: &'static str) {
    if !is_enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock()
        && let Some(ref mut s) = *state
    {
        let now = Instant::now();
        let ms = now.duration_since(s.last_time).as_millis() as u64;
        s.timings.push(TimingEntry { label, ms });
        s.last_time = now;
    }
}

/// Print all recorded timings to stderr.
pub fn print_timings() {
    if !is_enabled() {
        return;
    }
    let entries = STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.timings.clone()));
    let entries = match entries {
        Some(e) if !e.is_empty() => e,
        _ => return,
    };

    let total: u64 = entries.iter().map(|e| e.ms).sum();
    eprintln!("\n--- Startup Timings ---");
    for entry in &entries {
        eprintln!("  {}: {}ms", entry.label, entry.ms);
    }
    eprintln!("  TOTAL: {total}ms");
    eprintln!("------------------------\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_by_default() {
        // Don't set the env var — timing should be a no-op
        reset_timings();
        time("test");
        // Should not crash
    }

    #[test]
    fn test_reset_does_not_panic() {
        reset_timings();
        time("step1");
        reset_timings();
        time("step2");
        // Should not crash
    }
}
