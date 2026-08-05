//! Pending side-effect flags for [`super::HostSession`] (c730 / ath12).

use super::super::commands::{PendingBash, PendingSlash};

/// Side effects queued by sync `step` and consumed by [`crate::app::tui::effects::drain_pending`].
#[derive(Debug, Default)]
pub struct PendingOps {
    pub submit: Option<String>,
    /// Submit waiting for tool freeze (c1900 non-blocking gate).
    pub gated_submit: Option<String>,
    pub steer: Option<String>,
    pub follow_up: Option<String>,
    pub abort: bool,
    /// Alt+Up: restore queued messages to editor and clear both driver queues.
    pub dequeue: bool,
    pub slash: Option<PendingSlash>,
    pub bash: Option<PendingBash>,
    /// Refresh footer token estimate via XyDriver (c1035: turn end / stream close).
    pub footer_token_refresh: bool,
    /// Stage clipboard image into editor as tempfile path (c1155).
    pub paste_image: bool,
}

impl PendingOps {
    pub fn take_submit(&mut self) -> Option<String> {
        self.submit.take()
    }
    pub fn take_gated_submit(&mut self) -> Option<String> {
        self.gated_submit.take()
    }
    pub fn peek_gated_submit(&self) -> bool {
        self.gated_submit.is_some()
    }
    pub fn take_steer(&mut self) -> Option<String> {
        self.steer.take()
    }
    pub fn take_follow_up(&mut self) -> Option<String> {
        self.follow_up.take()
    }
    pub fn take_abort(&mut self) -> bool {
        std::mem::take(&mut self.abort)
    }
    pub fn take_dequeue(&mut self) -> bool {
        std::mem::take(&mut self.dequeue)
    }
    pub fn take_slash(&mut self) -> Option<PendingSlash> {
        self.slash.take()
    }
    pub fn take_bash(&mut self) -> Option<PendingBash> {
        self.bash.take()
    }
    pub fn take_footer_token_refresh(&mut self) -> bool {
        std::mem::take(&mut self.footer_token_refresh)
    }
    pub fn take_paste_image(&mut self) -> bool {
        std::mem::take(&mut self.paste_image)
    }
}
