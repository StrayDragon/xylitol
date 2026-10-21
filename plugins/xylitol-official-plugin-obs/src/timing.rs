use std::time::Instant;

fn is_enabled() -> bool {
    std::env::var("XYLITOL_TIMING")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

struct TimingState {
    timings: Vec<TimingEntry>,
    last_time: Instant,
}

#[derive(Clone)]
struct TimingEntry {
    label: &'static str,
    ms: u64,
}

use std::sync::Mutex;

static STATE: Mutex<Option<TimingState>> = Mutex::new(None);

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
    let mut block = String::from("--- Startup Timings ---\n");
    for entry in &entries {
        block.push_str(&format!("  {}: {}ms\n", entry.label, entry.ms));
    }
    block.push_str(&format!("  TOTAL: {total}ms\n------------------------"));
    log::info!(target: "xylitol::timing", "{block}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_by_default() {
        reset_timings();
        time("test");
        let state = STATE.lock().unwrap();
        assert!(state.is_none(), "disabled timing must not initialize state");
    }

    #[test]
    fn test_reset_does_not_panic() {
        reset_timings();
        time("step1");
        reset_timings();
        time("step2");
        let state = STATE.lock().unwrap();
        assert!(state.is_none(), "disabled timing must not record entries");
    }
}
