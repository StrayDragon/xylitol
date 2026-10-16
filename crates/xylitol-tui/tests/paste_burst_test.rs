//! PasteBurst detector tests (c415, spec pb01-pb03).
//!
//! Pure state machine; time is driven by explicit `Instant` values (no
//! MockClock needed — Instant arithmetic is deterministic). Mirrors pi
//! `editor.test.ts` PasteBurst describe block (c405 layer 1+3).

use std::time::{Duration, Instant};

use xylitol_tui::paste_burst::*;

/// Helper: a baseline instant tests build offsets from. Using a fixed epoch
/// keeps assertions readable (on_plain_char(t0 + 1ms)).
fn t0() -> Instant {
    Instant::now()
}

// ── pb01: burst detection ──────────────────────────────────────────────────

#[test]
fn burst_detected_after_8_fast_chars() {
    // 8 chars each 1ms apart → within the 8ms threshold → burst.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i));
    }
    assert!(
        burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(8)),
        "8 fast chars should suppress the following Enter"
    );
}

#[test]
fn no_burst_slow_typing() {
    // 8 chars each 20ms apart → exceeds the 8ms threshold → not consecutive.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i * 20));
    }
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(160)),
        "slow typing (20ms gaps) should not be a burst"
    );
}

#[test]
fn fewer_than_8_chars_no_burst() {
    // Only 5 fast chars — below the 8-char minimum.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..5u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i));
    }
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(5)),
        "fewer than 8 chars should not trigger a burst"
    );
}

// ── pb02: time boundary ────────────────────────────────────────────────────

#[test]
fn time_boundary_7ms_inside() {
    // 7ms gap is inside the 8ms threshold → consecutive counter increments.
    // After 8 such chars, a burst is recognized.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i * 7));
    }
    // 7ms * 7 gaps = 49ms; check right after the 8th char.
    assert!(
        burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(50)),
        "8 chars at 7ms gaps (inside threshold) should burst"
    );
}

#[test]
fn time_boundary_9ms_outside() {
    // 9ms gap exceeds the 8ms threshold → counter resets each time → never
    // reaches 8 consecutive.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i * 9));
    }
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(80)),
        "8 chars at 9ms gaps (outside threshold) should not burst"
    );
}

// ── pb03: enter-suppress window + reset ────────────────────────────────────

#[test]
fn enter_submits_after_suppress_window() {
    // A burst opens a 120ms suppress window; after it elapses, Enter submits.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i));
    }
    // Burst peaked at t0+7ms; suppress window opens at +7ms for 120ms → +127ms.
    assert!(
        burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(7)),
        "within the suppress window, Enter inserts a newline"
    );
    // +128ms: the 120ms window (from +7ms) has elapsed (7+120=127 < 128).
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(128)),
        "after the 120ms suppress window, Enter submits normally"
    );
}

#[test]
fn reset_clears_state() {
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i));
    }
    // Reset mid-burst.
    burst.reset();
    // An Enter right after reset must submit (no burst state).
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(10)),
        "after reset, Enter submits (burst state cleared)"
    );
    // And a fresh char does not carry over the prior run.
    burst.on_plain_char(t0 + Duration::from_millis(20));
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(21)),
        "a single char after reset is not a burst"
    );
}

// ── extra parity with pi editor.test.ts PasteBurst block ────────────────────

#[test]
fn single_char_does_not_suppress() {
    // pi: "does not suppress Enter after a single character"
    let t0 = t0();
    let mut burst = PasteBurst::new();
    burst.on_plain_char(t0);
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(1)),
        "a single char must not suppress Enter"
    );
}

#[test]
fn burst_window_extends_on_more_chars() {
    // Continued fast chars keep extending the suppress window; an Enter right
    // after the 10th char (still within the window) is suppressed.
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..10u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i));
    }
    assert!(
        burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(10)),
        "10 fast chars keep the suppress window open"
    );
}
