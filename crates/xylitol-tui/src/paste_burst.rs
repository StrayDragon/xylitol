use std::time::{Duration, Instant};

pub const PASTE_BURST_MIN_CHARS: u32 = 8;
pub const PASTE_BURST_CHAR_INTERVAL_MS: u64 = 8;
pub const PASTE_BURST_ACTIVE_IDLE_TIMEOUT_MS: u64 = 30;
pub const PASTE_BURST_ENTER_SUPPRESS_WINDOW_MS: u64 = 120;

const CHAR_INTERVAL: Duration = Duration::from_millis(PASTE_BURST_CHAR_INTERVAL_MS);
const ACTIVE_IDLE_TIMEOUT: Duration = Duration::from_millis(PASTE_BURST_ACTIVE_IDLE_TIMEOUT_MS);
const ENTER_SUPPRESS_WINDOW: Duration = Duration::from_millis(PASTE_BURST_ENTER_SUPPRESS_WINDOW_MS);

#[derive(Default)]
pub struct PasteBurst {
    last_plain_char_at: Option<Instant>,
    consecutive_plain_chars: u32,
    active_until: Option<Instant>,
    enter_suppress_until: Option<Instant>,
}

impl PasteBurst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_plain_char(&mut self, now: Instant) {
        let is_consecutive = self
            .last_plain_char_at
            .is_some_and(|last| now.duration_since(last) <= CHAR_INTERVAL);

        if is_consecutive {
            self.consecutive_plain_chars += 1;
        } else {
            self.consecutive_plain_chars = 1;
        }
        self.last_plain_char_at = Some(now);

        if self.consecutive_plain_chars >= PASTE_BURST_MIN_CHARS {
            self.extend_window(now);
        }
    }

    pub fn should_insert_newline_instead_of_submit(&self, now: Instant) -> bool {
        if self.active_until.is_some_and(|t| now <= t)
            || self.enter_suppress_until.is_some_and(|t| now <= t)
        {
            return true;
        }
        self.last_plain_char_at.is_some_and(|last| {
            self.consecutive_plain_chars >= PASTE_BURST_MIN_CHARS
                && now.duration_since(last) <= CHAR_INTERVAL
        })
    }

    pub fn is_coalescing(&self, now: Instant) -> bool {
        self.active_until.is_some_and(|t| now <= t)
            || self.enter_suppress_until.is_some_and(|t| now <= t)
            || self.last_plain_char_at.is_some_and(|last| {
                self.consecutive_plain_chars >= PASTE_BURST_MIN_CHARS
                    && now.duration_since(last) <= ACTIVE_IDLE_TIMEOUT
            })
    }

    pub fn consecutive_plain_chars(&self) -> u32 {
        self.consecutive_plain_chars
    }

    pub fn extend_window(&mut self, now: Instant) {
        self.active_until = Some(now + ACTIVE_IDLE_TIMEOUT);
        self.enter_suppress_until = Some(now + ENTER_SUPPRESS_WINDOW);
    }

    pub fn reset(&mut self) {
        self.last_plain_char_at = None;
        self.consecutive_plain_chars = 0;
        self.active_until = None;
        self.enter_suppress_until = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_coalescing_expires_after_idle_even_with_high_consecutive() {
        let mut burst = PasteBurst::new();
        let t0 = Instant::now();
        for _ in 0..PASTE_BURST_MIN_CHARS {
            burst.on_plain_char(t0);
        }
        assert!(burst.is_coalescing(t0));
        let later = t0 + ENTER_SUPPRESS_WINDOW + Duration::from_millis(1);
        assert!(
            !burst.is_coalescing(later),
            "must not stick forever after active/suppress windows expire"
        );
    }
}
