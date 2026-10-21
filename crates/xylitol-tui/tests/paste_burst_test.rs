use std::time::{Duration, Instant};

use xylitol_tui::paste_burst::*;

fn t0() -> Instant {
    Instant::now()
}

#[test]
fn burst_detected_after_8_fast_chars() {
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

#[test]
fn time_boundary_7ms_inside() {
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i * 7));
    }
    assert!(
        burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(50)),
        "8 chars at 7ms gaps (inside threshold) should burst"
    );
}

#[test]
fn time_boundary_9ms_outside() {
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

#[test]
fn enter_submits_after_suppress_window() {
    let t0 = t0();
    let mut burst = PasteBurst::new();
    for i in 0..8u64 {
        burst.on_plain_char(t0 + Duration::from_millis(i));
    }
    assert!(
        burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(7)),
        "within the suppress window, Enter inserts a newline"
    );
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
    burst.reset();
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(10)),
        "after reset, Enter submits (burst state cleared)"
    );
    burst.on_plain_char(t0 + Duration::from_millis(20));
    assert!(
        !burst.should_insert_newline_instead_of_submit(t0 + Duration::from_millis(21)),
        "a single char after reset is not a burst"
    );
}

#[test]
fn single_char_does_not_suppress() {
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
