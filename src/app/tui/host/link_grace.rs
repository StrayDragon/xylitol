//! Attach 连接宽限分级（ath42/c2480）：断线是常态事件，宽限内零打扰，
//! 超宽限才以**通知条**提示；瞬时抖动不打扰用户。
//!
//! 纯状态机，无 IO——`tick` 由 host 循环每次 idle 步进驱动。

use std::time::{Duration, Instant};

use crate::app::core::driver::LinkHealth;

/// 初始连接宽限：attach 起步 Host 可能尚未监听，久等不弹。
pub const LINK_GRACE_INITIAL: Duration = Duration::from_secs(5);
/// 重连宽限：已连接过的会话断线，抖动在此窗口内自愈则完全静默。
pub const LINK_GRACE_RECONNECT: Duration = Duration::from_secs(1);

/// 通知条文案（toast notice，E 类运行态；词表 SSOT：
/// `docs/architecture/TUI信息呈现与固定区词汇.md`）。
pub const LINK_DOWN_NOTICE: &str = "连接断开，重连中…";
pub const LINK_RECOVERED_NOTICE: &str = "Host 连接已恢复";

/// 宽限期内的一次性通告请求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkNotice {
    /// 断线超过宽限，应上通知条（仅一次）。
    Disconnected,
    /// 从已通告的断线态恢复，应上通知条（仅一次）。
    Recovered,
}

/// 宽限分级状态机。
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

    /// 每个宿主 tick 调用一次；按当前链路健康返回需要的通告（若有）。
    ///
    /// 分级：从未连接过（attach 起步）用 [`LINK_GRACE_INITIAL`]，连接过之后
    /// 的断线用 [`LINK_GRACE_RECONNECT`]。宽限内不打扰；超宽限只通告一次；
    /// 恢复通告只在确曾通告过断线时给出。
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
        // attach 起步即 Down：宽限内静默
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
        // 已连接过的断线：宽限降为 1s
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
