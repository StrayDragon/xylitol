use std::time::{Instant, SystemTime, UNIX_EPOCH};

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

const STREAM_NODE_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamNode {
    AgentStart = 0,
    StepStart = 1,
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
            Self::StepStart => "turnStartedAtMs",
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
        Self::StepStart,
        Self::ThinkingStart,
        Self::ThinkingEnd,
        Self::TextStart,
        Self::TextEnd,
        Self::ToolIntent,
        Self::MessageEnd,
    ];
}

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

    pub fn finish_message(&mut self) {
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
    fn stream_nodes_write_once_except_text_end() {
        let mut c = StreamNodeClock::new();
        let a = t0();
        c.stamp_at(StreamNode::AgentStart, a, 10);
        c.begin_turn();
        c.stamp_at(StreamNode::StepStart, a, 20);
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
