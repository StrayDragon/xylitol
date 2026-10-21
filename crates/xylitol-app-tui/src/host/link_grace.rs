use std::time::{Duration, Instant};

use crate::driver::LinkHealth;

pub const LINK_GRACE_INITIAL: Duration = Duration::from_secs(5);
pub const LINK_GRACE_RECONNECT: Duration = Duration::from_secs(1);

pub const LINK_DOWN_NOTICE: &str = "连接断开，重连中…";
pub const LINK_RECOVERED_NOTICE: &str = "Host 连接已恢复";

pub const OBSERVE_INFLIGHT_NOTICE: &str =
    "另一端正在对话：已显示会话快照；结束后 /session-resume 收看与接管";
pub const OBSERVE_ENDED_NOTICE: &str = "另一端回合已结束：/session-resume 收看刚才的回合";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkNotice {
    Disconnected,
    Recovered,
}

#[derive(Debug, Default)]
pub struct LinkGrace {
    ever_up: bool,
    down_since: Option<Instant>,
    announced_down: bool,
}

impl LinkGrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(
        &mut self,
        health: LinkHealth,
        now: Instant,
        grace_initial: Duration,
        grace_reconnect: Duration,
    ) -> Option<LinkNotice> {
        match health {
            LinkHealth::Up => {
                let was_down = self.down_since.take().is_some();
                self.ever_up = true;
                if was_down && self.announced_down {
                    self.announced_down = false;
                    return Some(LinkNotice::Recovered);
                }
                None
            }
            LinkHealth::Down => {
                let since = *self.down_since.get_or_insert(now);
                let grace = if self.ever_up {
                    grace_reconnect
                } else {
                    grace_initial
                };
                if !self.announced_down && now.duration_since(since) >= grace {
                    self.announced_down = true;
                    return Some(LinkNotice::Disconnected);
                }
                None
            }
        }
    }
}

pub fn observe_notice(
    prev: crate::driver::AttachObserve,
    cur: crate::driver::AttachObserve,
) -> Option<&'static str> {
    if prev == cur {
        return None;
    }
    match cur {
        crate::driver::AttachObserve::Inflight => Some(OBSERVE_INFLIGHT_NOTICE),
        crate::driver::AttachObserve::Ended => Some(OBSERVE_ENDED_NOTICE),
        crate::driver::AttachObserve::Live => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ms: u64) -> Instant {
        Instant::now() - Duration::from_millis(10_000) + Duration::from_millis(ms)
    }

    #[test]
    fn initial_grace_uses_long_window_then_notices_once() {
        let mut g = LinkGrace::new();
        let t0 = t(0);
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0,
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None
        );
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0 + Duration::from_millis(4_999),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None,
            "5s 宽限内 MUST 零打扰"
        );
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0 + LINK_GRACE_INITIAL,
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            Some(LinkNotice::Disconnected),
            "超初始宽限 MUST 上通知条"
        );
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0 + LINK_GRACE_INITIAL + Duration::from_secs(1),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None,
            "断线通告 MUST 只发一次"
        );
    }

    #[test]
    fn reconnect_grace_is_shorter_after_first_connection() {
        let mut g = LinkGrace::new();
        let t0 = t(0);
        assert_eq!(
            g.tick(LinkHealth::Up, t0, LINK_GRACE_INITIAL, LINK_GRACE_RECONNECT),
            None
        );
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0 + Duration::from_millis(200),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None,
            "重连宽限内瞬时抖动 MUST 完全静默"
        );
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0 + Duration::from_millis(1_200),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            Some(LinkNotice::Disconnected),
            "超重连宽限 MUST 上通知条"
        );
        assert_eq!(
            g.tick(
                LinkHealth::Up,
                t0 + Duration::from_millis(1_400),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            Some(LinkNotice::Recovered),
            "恢复 MUST 上通知条收尾"
        );
        assert_eq!(
            g.tick(
                LinkHealth::Up,
                t0 + Duration::from_millis(1_500),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None
        );
    }

    #[test]
    fn transient_blip_within_grace_never_notices() {
        let mut g = LinkGrace::new();
        let t0 = t(0);
        assert_eq!(
            g.tick(LinkHealth::Up, t0, LINK_GRACE_INITIAL, LINK_GRACE_RECONNECT),
            None
        );
        assert_eq!(
            g.tick(
                LinkHealth::Down,
                t0 + Duration::from_millis(100),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None
        );
        assert_eq!(
            g.tick(
                LinkHealth::Up,
                t0 + Duration::from_millis(150),
                LINK_GRACE_INITIAL,
                LINK_GRACE_RECONNECT
            ),
            None,
            "宽限内自愈 MUST 无任何通告（含恢复通告）"
        );
    }
}
