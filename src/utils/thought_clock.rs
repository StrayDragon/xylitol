//! Online thought interval: stamp start/end once, elapsed is a subtraction.
//!
//! O(1) per event. Nodes: first thinking delta (start); first channel switch
//! away from thinking (end). Later events are no-ops. Duration is computed
//! only when read — never by holding a start Instant until message flush.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Wall-clock stamps for one thinking burst (live + persist).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThoughtClock {
    started_at: Option<Instant>,
    ended_at: Option<Instant>,
    started_at_ms: Option<u64>,
    ended_at_ms: Option<u64>,
}

/// JSON extras on a persisted assistant message (`thinkingElapsedSecs` + node ms).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThoughtPersist {
    pub elapsed_secs: Option<u64>,
    pub started_at_ms: Option<u64>,
    pub ended_at_ms: Option<u64>,
}

impl ThoughtClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_started(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn has_ended(&self) -> bool {
        self.ended_at.is_some()
    }

    #[cfg(test)]
    pub fn started_at(&self) -> Option<Instant> {
        self.started_at
    }

    #[cfg(test)]
    pub fn started_at_ms(&self) -> Option<u64> {
        self.started_at_ms
    }

    #[cfg(test)]
    pub fn ended_at_ms(&self) -> Option<u64> {
        self.ended_at_ms
    }

    /// First thinking node. Subsequent calls are no-ops.
    pub fn stamp_start(&mut self) {
        self.stamp_start_at(Instant::now(), unix_now_ms());
    }

    pub fn stamp_start_at(&mut self, at: Instant, unix_ms: u64) {
        if self.started_at.is_none() {
            self.started_at = Some(at);
            self.started_at_ms = Some(unix_ms);
        }
    }

    /// Thinking-channel end (first TextDelta / tool / ThinkingEnd). No-op if already ended
    /// or never started. Responses `ThinkingEnd` often arrives with `Done` after text —
    /// first TextDelta must win.
    pub fn stamp_end(&mut self) {
        self.stamp_end_at(Instant::now(), unix_now_ms());
    }

    pub fn stamp_end_at(&mut self, at: Instant, unix_ms: u64) {
        if self.started_at.is_some() && self.ended_at.is_none() {
            self.ended_at = Some(at);
            self.ended_at_ms = Some(unix_ms);
        }
    }

    /// Test/scene: pin start and clear end so a later `stamp_end` measures from here.
    #[cfg(test)]
    pub fn pin_start_at(&mut self, at: Instant, unix_ms: u64) {
        self.started_at = Some(at);
        self.started_at_ms = Some(unix_ms);
        self.ended_at = None;
        self.ended_at_ms = None;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whole seconds from stamped start to stamped end. `None` if open or under 1s.
    pub fn elapsed_secs(&self) -> Option<u64> {
        let start = self.started_at?;
        let end = self.ended_at?;
        secs_between(start, end)
    }

    /// If still open, treat `until` as end without mutating (abort / thinking-only flush).
    pub fn elapsed_secs_until(&self, until: Instant) -> Option<u64> {
        let start = self.started_at?;
        let end = self.ended_at.unwrap_or(until);
        secs_between(start, end)
    }

    /// Snapshot for session JSONL. Open clocks close at `now` (thinking-only / abort).
    pub fn persist_fields(&self) -> ThoughtPersist {
        if !self.has_started() {
            return ThoughtPersist::default();
        }
        let (elapsed_secs, ended_at_ms) = if self.has_ended() {
            (self.elapsed_secs(), self.ended_at_ms)
        } else {
            (self.elapsed_secs_until(Instant::now()), Some(unix_now_ms()))
        };
        ThoughtPersist {
            elapsed_secs,
            started_at_ms: self.started_at_ms,
            ended_at_ms,
        }
    }
}

fn secs_between(start: Instant, end: Instant) -> Option<u64> {
    let secs = end.saturating_duration_since(start).as_secs();
    (secs > 0).then_some(secs)
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Resume helper: prefer stored secs, else `ended_ms - started_ms`.
pub fn elapsed_from_persist_ms(
    elapsed_secs: Option<u64>,
    started_at_ms: Option<u64>,
    ended_at_ms: Option<u64>,
) -> Option<u64> {
    elapsed_secs.filter(|s| *s > 0).or_else(|| {
        let start = started_at_ms?;
        let end = ended_at_ms?;
        end.checked_sub(start).map(|d| d / 1000).filter(|s| *s > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn stamps_are_write_once() {
        let mut c = ThoughtClock::new();
        let a = t0();
        let b = a + Duration::from_secs(3);
        let d = a + Duration::from_secs(9);
        c.stamp_start_at(a, 1_000);
        c.stamp_start_at(b, 4_000);
        c.stamp_end_at(b, 4_000);
        c.stamp_end_at(d, 10_000);
        assert_eq!(c.started_at_ms(), Some(1_000));
        assert_eq!(c.ended_at_ms(), Some(4_000));
        assert_eq!(c.elapsed_secs(), Some(3));
    }

    #[test]
    fn elapsed_is_subtraction_not_open_interval() {
        let mut c = ThoughtClock::new();
        let a = t0();
        c.stamp_start_at(a, 0);
        assert_eq!(c.elapsed_secs(), None, "open clock has no frozen elapsed");
        c.stamp_end_at(a + Duration::from_secs(2), 2_000);
        assert_eq!(c.elapsed_secs(), Some(2));
        assert_eq!(
            c.elapsed_secs_until(a + Duration::from_secs(40)),
            Some(2),
            "later read must not include text/tool wall time"
        );
    }

    #[test]
    fn sub_second_omits_secs_but_keeps_node_ms() {
        let mut c = ThoughtClock::new();
        let a = t0();
        c.stamp_start_at(a, 100);
        c.stamp_end_at(a + Duration::from_millis(400), 500);
        let p = c.persist_fields();
        assert_eq!(p.elapsed_secs, None);
        assert_eq!(p.started_at_ms, Some(100));
        assert_eq!(p.ended_at_ms, Some(500));
    }

    #[test]
    fn end_without_start_is_noop() {
        let mut c = ThoughtClock::new();
        c.stamp_end_at(t0(), 1);
        assert!(!c.has_started());
        assert!(!c.has_ended());
        assert_eq!(c.persist_fields(), ThoughtPersist::default());
    }

    #[test]
    fn resume_prefers_secs_then_ms_diff() {
        assert_eq!(
            elapsed_from_persist_ms(Some(4), Some(0), Some(9_000)),
            Some(4)
        );
        assert_eq!(
            elapsed_from_persist_ms(None, Some(1_000), Some(3_500)),
            Some(2)
        );
        assert_eq!(
            elapsed_from_persist_ms(Some(0), Some(1_000), Some(1_400)),
            None
        );
    }
}
