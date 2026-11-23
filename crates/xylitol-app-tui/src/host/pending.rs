use super::super::commands::{PendingBash, PendingSlash};
use super::super::mcp_attach::{McpPromptTarget, PendingMcpAttachments};

#[derive(Debug)]
pub struct PendingSubmit {
    pub prompt: String,
    pub attachments: PendingMcpAttachments,
    pub gated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneOrigin {
    Gated,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneEntry {
    pub text: String,
    pub origin: LaneOrigin,
}

impl LaneEntry {
    fn gated(text: String) -> Self {
        Self {
            text,
            origin: LaneOrigin::Gated,
        }
    }
    fn busy(text: String) -> Self {
        Self {
            text,
            origin: LaneOrigin::Busy,
        }
    }
}

#[derive(Debug, Default)]
pub struct PendingOps {
    pub submit: Option<PendingSubmit>,
    pub mcp_prompt_args_result: Option<(McpPromptTarget, String)>,
    pub steer: Vec<LaneEntry>,
    pub follow_up: Vec<LaneEntry>,
    pub abort: bool,
    pub bash_cancelled: bool,
    pub dequeue: bool,
    pub dequeue_one: bool,
    pub slash: Option<PendingSlash>,
    pub bash: Option<PendingBash>,
    pub reload: bool,
    pub footer_token_refresh: bool,
    pub queue_refresh: bool,
    pub paste_image: bool,
}

impl PendingOps {
    pub fn take_submit(&mut self) -> Option<String> {
        match self.submit.take() {
            Some(entry) if !entry.gated => Some(entry.prompt),
            other => {
                self.submit = other;
                None
            }
        }
    }
    pub fn take_submit_with_attachments(&mut self) -> Option<(String, PendingMcpAttachments)> {
        match self.submit.take() {
            Some(entry) if !entry.gated => Some((entry.prompt, entry.attachments)),
            other => {
                self.submit = other;
                None
            }
        }
    }
    pub fn take_gated_submit(&mut self) -> Option<String> {
        match self.submit.take() {
            Some(entry) if entry.gated => Some(entry.prompt),
            other => {
                self.submit = other;
                None
            }
        }
    }
    pub fn take_gated_submit_with_attachments(
        &mut self,
    ) -> Option<(String, PendingMcpAttachments)> {
        match self.submit.take() {
            Some(entry) if entry.gated => Some((entry.prompt, entry.attachments)),
            other => {
                self.submit = other;
                None
            }
        }
    }
    pub fn peek_gated_submit(&self) -> bool {
        self.submit.as_ref().is_some_and(|entry| entry.gated)
    }
    pub fn push_steer(&mut self, text: String) {
        self.steer.push(LaneEntry::busy(text));
    }
    pub fn push_follow_up(&mut self, text: String) {
        self.follow_up.push(LaneEntry::busy(text));
    }
    pub fn push_gated_follow_up(&mut self, text: String) {
        self.follow_up.push(LaneEntry::gated(text));
    }
    pub fn take_steer(&mut self) -> Option<LaneEntry> {
        if self.steer.is_empty() {
            None
        } else {
            Some(self.steer.remove(0))
        }
    }
    pub fn take_follow_up(&mut self) -> Option<LaneEntry> {
        if self.follow_up.is_empty() {
            None
        } else {
            Some(self.follow_up.remove(0))
        }
    }
    pub fn take_busy_follow_up(&mut self) -> Option<LaneEntry> {
        let pos = self
            .follow_up
            .iter()
            .position(|entry| entry.origin == LaneOrigin::Busy)?;
        Some(self.follow_up.remove(pos))
    }
    pub fn take_gated_follow_up(&mut self) -> Option<LaneEntry> {
        let pos = self
            .follow_up
            .iter()
            .position(|entry| entry.origin == LaneOrigin::Gated)?;
        Some(self.follow_up.remove(pos))
    }
    pub fn gated_follow_up_count(&self) -> usize {
        self.follow_up
            .iter()
            .filter(|entry| entry.origin == LaneOrigin::Gated)
            .count()
    }
    pub fn remove_follow_up_matching(&mut self, text: &str) -> bool {
        if let Some(pos) = self.follow_up.iter().position(|e| e.text == text) {
            self.follow_up.remove(pos);
            true
        } else {
            false
        }
    }
    pub fn clear_lanes(&mut self) {
        self.steer.clear();
        self.follow_up.clear();
    }
    pub fn take_abort(&mut self) -> bool {
        std::mem::take(&mut self.abort)
    }
    pub fn take_bash_cancelled(&mut self) -> bool {
        std::mem::take(&mut self.bash_cancelled)
    }
    pub fn take_dequeue(&mut self) -> bool {
        std::mem::take(&mut self.dequeue)
    }
    pub fn take_dequeue_one(&mut self) -> bool {
        std::mem::take(&mut self.dequeue_one)
    }
    pub fn take_slash(&mut self) -> Option<PendingSlash> {
        self.slash.take()
    }
    pub fn put_slash(&mut self, slash: PendingSlash) {
        self.slash = Some(slash);
    }
    pub fn take_bash(&mut self) -> Option<PendingBash> {
        self.bash.take()
    }
    pub fn take_reload(&mut self) -> bool {
        std::mem::take(&mut self.reload)
    }
    pub fn take_footer_token_refresh(&mut self) -> bool {
        std::mem::take(&mut self.footer_token_refresh)
    }
    pub fn take_paste_image(&mut self) -> bool {
        std::mem::take(&mut self.paste_image)
    }
}
