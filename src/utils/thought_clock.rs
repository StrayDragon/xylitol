//! Online stream clocks: O(1) stamp per event, duration is a subtraction when read.
//!
//! [`ThoughtClock`]: TUI live thinking channel (start / first switch away).
//! [`StreamNodeClock`]: ReAct persist nodes (`streamTiming` unix-ms).

use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Wall-clock stamps for one live thinking burst (TUI).
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

    /// First thinking node. Subsequent calls are no-ops.
    pub fn stamp_start(&mut self) {
        self.stamp_start_at(Instant::now());
    }

    pub fn stamp_start_at(&mut self, at: Instant) {
        if self.started_at.is_none() {
            self.started_at = Some(at);
        }
    }

    /// Thinking-channel end (first TextDelta / tool / ThinkingEnd). No-op if already ended
    /// or never started. Responses `ThinkingEnd` often arrives with `Done` after text —
    /// first TextDelta must win.
    pub fn stamp_end(&mut self) {
        self.stamp_end_at(Instant::now());
    }

    pub fn stamp_end_at(&mut self, at: Instant) {
        if self.started_at.is_some() && self.ended_at.is_none() {
            self.ended_at = Some(at);
        }
    }

    /// Test/scene: pin start and clear end so a later `stamp_end` measures from here.
    #[cfg(test)]
    pub fn pin_start_at(&mut self, at: Instant) {
        self.started_at = Some(at);
        self.ended_at = None;
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

    /// If still open, treat `until` as end without mutating.
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

const STREAM_NODE_COUNT: usize = 8;

/// Named wall-clock nodes on one assistant generate (O(1) stamp, subtract when read).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamNode {
    AgentStart = 0,
    TurnStart = 1,
    ThinkingStart = 2,
    ThinkingEnd = 3,
    TextStart = 4,
    TextEnd = 5,
    ToolIntent = 6,
    MessageEnd = 7,
}

impl StreamNode {
    fn idx(self) -> usize {
        self as usize
    }

    fn json_key(self) -> &'static str {
        match self {
            Self::AgentStart => "agentStartedAtMs",
            Self::TurnStart => "turnStartedAtMs",
            Self::ThinkingStart => "thinkingStartedAtMs",
            Self::ThinkingEnd => "thinkingEndedAtMs",
            Self::TextStart => "textStartedAtMs",
            Self::TextEnd => "textEndedAtMs",
            Self::ToolIntent => "toolIntentAtMs",
            Self::MessageEnd => "messageEndedAtMs",
        }
    }

    const ALL: [Self; STREAM_NODE_COUNT] = [
        Self::AgentStart,
        Self::TurnStart,
        Self::ThinkingStart,
        Self::ThinkingEnd,
        Self::TextStart,
        Self::TextEnd,
        Self::ToolIntent,
        Self::MessageEnd,
    ];
}

/// Online node clock: start-class nodes write once; [`StreamNode::TextEnd`] last-writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamNodeClock {
    instants: [Option<Instant>; STREAM_NODE_COUNT],
    unix_ms: [Option<u64>; STREAM_NODE_COUNT],
}

impl StreamNodeClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unix_ms(&self, node: StreamNode) -> Option<u64> {
        self.unix_ms[node.idx()]
    }

    pub fn has(&self, node: StreamNode) -> bool {
        self.instants[node.idx()].is_some()
    }

    pub fn stamp(&mut self, node: StreamNode) {
        self.stamp_at(node, Instant::now(), unix_now_ms());
    }

    pub fn stamp_at(&mut self, node: StreamNode, at: Instant, unix_ms: u64) {
        let i = node.idx();
        if node == StreamNode::TextEnd {
            self.instants[i] = Some(at);
            self.unix_ms[i] = Some(unix_ms);
            return;
        }
        if self.instants[i].is_none() {
            self.instants[i] = Some(at);
            self.unix_ms[i] = Some(unix_ms);
        }
    }

    /// Keep AgentStart; clear per-turn / per-message nodes.
    pub fn begin_turn(&mut self) {
        let agent_at = self.instants[StreamNode::AgentStart.idx()];
        let agent_ms = self.unix_ms[StreamNode::AgentStart.idx()];
        *self = Self::default();
        self.instants[StreamNode::AgentStart.idx()] = agent_at;
        self.unix_ms[StreamNode::AgentStart.idx()] = agent_ms;
    }

    pub fn on_thinking_delta(&mut self) {
        self.stamp(StreamNode::ThinkingStart);
    }

    pub fn on_text_delta(&mut self) {
        self.stamp(StreamNode::ThinkingEnd);
        self.stamp(StreamNode::TextStart);
        self.stamp(StreamNode::TextEnd);
    }

    pub fn on_thinking_end_chunk(&mut self) {
        self.stamp(StreamNode::ThinkingEnd);
    }

    pub fn on_tool_intent(&mut self) {
        self.stamp(StreamNode::ThinkingEnd);
        self.stamp(StreamNode::ToolIntent);
    }

    /// Thinking-only fallback then message end, just before persist.
    pub fn finish_message(&mut self) {
        // Single clock capture: thinking closes *at* message end, so both stamps
        // share one wall-clock read (keeps the ms-equality assertion stable under
        // parallel load instead of straddling two SystemTime::now() reads).
        let at = Instant::now();
        let ms = unix_now_ms();
        if self.has(StreamNode::ThinkingStart) && !self.has(StreamNode::ThinkingEnd) {
            self.stamp_at(StreamNode::ThinkingEnd, at, ms);
        }
        self.stamp_at(StreamNode::MessageEnd, at, ms);
    }

    pub fn elapsed_secs(&self, start: StreamNode, end: StreamNode) -> Option<u64> {
        let s = self.instants[start.idx()]?;
        let e = self.instants[end.idx()]?;
        secs_between(s, e)
    }

    pub fn thinking_elapsed_secs(&self) -> Option<u64> {
        self.elapsed_secs(StreamNode::ThinkingStart, StreamNode::ThinkingEnd)
    }

    pub fn timing_pairs(&self) -> Vec<(&'static str, u64)> {
        StreamNode::ALL
            .into_iter()
            .filter_map(|node| self.unix_ms(node).map(|ms| (node.json_key(), ms)))
            .collect()
    }
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
        c.stamp_start_at(a);
        c.stamp_start_at(b);
        c.stamp_end_at(b);
        c.stamp_end_at(d);
        assert_eq!(c.elapsed_secs(), Some(3));
    }

    #[test]
    fn elapsed_is_subtraction_not_open_interval() {
        let mut c = ThoughtClock::new();
        let a = t0();
        c.stamp_start_at(a);
        assert_eq!(c.elapsed_secs(), None, "open clock has no frozen elapsed");
        c.stamp_end_at(a + Duration::from_secs(2));
        assert_eq!(c.elapsed_secs(), Some(2));
        assert_eq!(
            c.elapsed_secs_until(a + Duration::from_secs(40)),
            Some(2),
            "later read must not include text/tool wall time"
        );
    }

    #[test]
    fn sub_second_omits_elapsed_secs() {
        let mut c = ThoughtClock::new();
        let a = t0();
        c.stamp_start_at(a);
        c.stamp_end_at(a + Duration::from_millis(400));
        assert_eq!(c.elapsed_secs(), None);
    }

    #[test]
    fn end_without_start_is_noop() {
        let mut c = ThoughtClock::new();
        c.stamp_end_at(t0());
        assert!(!c.has_ended());
        assert_eq!(c.elapsed_secs(), None);
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

    #[test]
    fn stream_nodes_write_once_except_text_end() {
        let mut c = StreamNodeClock::new();
        let a = t0();
        c.stamp_at(StreamNode::AgentStart, a, 10);
        c.begin_turn();
        c.stamp_at(StreamNode::TurnStart, a, 20);
        c.stamp_at(StreamNode::ThinkingStart, a, 30);
        c.stamp_at(StreamNode::ThinkingEnd, a + Duration::from_secs(2), 2_030);
        c.stamp_at(StreamNode::TextStart, a + Duration::from_secs(2), 2_031);
        c.stamp_at(StreamNode::TextEnd, a + Duration::from_secs(2), 2_031);
        c.stamp_at(StreamNode::TextEnd, a + Duration::from_secs(7), 7_000);
        c.stamp_at(StreamNode::ThinkingEnd, a + Duration::from_secs(9), 9_000);
        assert_eq!(c.unix_ms(StreamNode::AgentStart), Some(10));
        assert_eq!(c.unix_ms(StreamNode::ThinkingEnd), Some(2_030));
        assert_eq!(c.unix_ms(StreamNode::TextEnd), Some(7_000));
        assert_eq!(
            c.elapsed_secs(StreamNode::ThinkingStart, StreamNode::ThinkingEnd),
            Some(2)
        );
        c.finish_message();
        assert!(c.has(StreamNode::MessageEnd));
        let keys: Vec<_> = c.timing_pairs().into_iter().map(|(k, _)| k).collect();
        assert!(keys.contains(&"thinkingStartedAtMs"));
        assert!(keys.contains(&"textEndedAtMs"));
        assert!(keys.contains(&"messageEndedAtMs"));
    }

    #[test]
    fn finish_message_closes_open_thinking_without_done() {
        let mut c = StreamNodeClock::new();
        let a = t0();
        c.stamp_at(StreamNode::ThinkingStart, a, 1);
        c.finish_message();
        assert!(c.has(StreamNode::ThinkingEnd));
        assert!(c.has(StreamNode::MessageEnd));
        assert_eq!(
            c.unix_ms(StreamNode::ThinkingEnd),
            c.unix_ms(StreamNode::MessageEnd)
        );
    }
}
