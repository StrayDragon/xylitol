use super::super::commands::{PendingBash, PendingSlash};
use super::super::mcp_attach::{McpPromptTarget, PendingMcpAttachments};

#[derive(Debug)]
pub struct PendingSubmit {
    pub prompt: String,
    pub attachments: PendingMcpAttachments,
    pub gated: bool,
}

#[derive(Debug, Default)]
pub struct PendingOps {
    pub submit: Option<PendingSubmit>,
    pub mcp_prompt_args_result: Option<(McpPromptTarget, String)>,
    pub steer: Vec<String>,
    pub follow_up: Vec<String>,
    pub revoke_queue_item: Option<String>,
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
    pub fn peek_submit(&self) -> bool {
        self.submit.is_some()
    }
    pub fn push_steer(&mut self, text: String) {
        self.steer.push(text);
    }
    pub fn push_follow_up(&mut self, text: String) {
        self.follow_up.push(text);
    }
    pub fn take_all_steer(&mut self) -> Vec<String> {
        std::mem::take(&mut self.steer)
    }
    pub fn take_all_follow_up(&mut self) -> Vec<String> {
        std::mem::take(&mut self.follow_up)
    }
    pub fn forget_steer_matching(&mut self, text: &str) {
        self.steer.retain(|entry| entry != text);
    }
    pub fn forget_follow_up_matching(&mut self, text: &str) {
        self.follow_up.retain(|entry| entry != text);
    }
    pub fn take_revoke_queue_item(&mut self) -> Option<String> {
        self.revoke_queue_item.take()
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
