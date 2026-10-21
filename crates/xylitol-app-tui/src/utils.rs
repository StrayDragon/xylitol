use std::time::Instant;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThoughtClock {
    started_at: Option<Instant>,
    ended_at: Option<Instant>,
}

impl ThoughtClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_ended(&self) -> bool {
        self.ended_at.is_some()
    }

    #[cfg(test)]
    pub fn started_at(&self) -> Option<Instant> {
        self.started_at
    }

    pub fn stamp_start(&mut self) {
        self.stamp_start_at(Instant::now());
    }

    pub fn stamp_start_at(&mut self, at: Instant) {
        if self.started_at.is_none() {
            self.started_at = Some(at);
        }
    }

    pub fn stamp_end(&mut self) {
        self.stamp_end_at(Instant::now());
    }

    pub fn stamp_end_at(&mut self, at: Instant) {
        if self.started_at.is_some() && self.ended_at.is_none() {
            self.ended_at = Some(at);
        }
    }

    #[cfg(test)]
    pub fn pin_start_at(&mut self, at: Instant) {
        self.started_at = Some(at);
        self.ended_at = None;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn elapsed_secs(&self) -> Option<u64> {
        let start = self.started_at?;
        let end = self.ended_at?;
        secs_between(start, end)
    }

    #[cfg(test)]
    pub fn elapsed_secs_until(&self, until: Instant) -> Option<u64> {
        let start = self.started_at?;
        let end = self.ended_at.unwrap_or(until);
        secs_between(start, end)
    }
}

fn secs_between(start: Instant, end: Instant) -> Option<u64> {
    let secs = end.saturating_duration_since(start).as_secs();
    (secs > 0).then_some(secs)
}

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
