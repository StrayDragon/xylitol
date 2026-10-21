use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::runtime::AgentHooks;
use crate::agent::tools::{ToolFreezePhase, ToolSet, ToolTableFingerprint};
use xylitol_plugin_sdk::XyPermission;

use super::AgentCapabilities;

impl AgentCapabilities {
    pub fn set_tools(&mut self, tools: ToolSet) {
        if self.tool_freeze == ToolFreezePhase::Frozen {
            log::warn!(
                target: "xylitol::agent",
                "set_tools ignored: tool table FROZEN (use freeze_tools / reopen_tools_for_regate)"
            );
            return;
        }
        if !self.allow_tools_rewrite_now("set_tools") {
            return;
        }
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.rebuild_system_prompt();
    }

    pub fn set_tools_defer_prompt(&mut self, tools: ToolSet) -> SystemPromptOpts {
        if self.tool_freeze == ToolFreezePhase::Frozen {
            log::warn!(
                target: "xylitol::agent",
                "set_tools_defer_prompt ignored: tool table FROZEN"
            );
            return self.prompt_opts.clone();
        }
        if !self.allow_tools_rewrite_now("set_tools_defer_prompt") {
            return self.prompt_opts.clone();
        }
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.prompt_opts.clone()
    }

    pub fn tool_freeze_phase(&self) -> ToolFreezePhase {
        self.tool_freeze
    }

    pub fn is_tools_frozen(&self) -> bool {
        self.tool_freeze == ToolFreezePhase::Frozen
    }

    pub fn frozen_tool_fingerprint(&self) -> Option<&ToolTableFingerprint> {
        self.tool_fingerprint.as_ref()
    }

    pub fn begin_tool_gating(&mut self) {
        if self.tool_freeze != ToolFreezePhase::Frozen {
            self.tool_freeze = ToolFreezePhase::Gating;
        }
    }

    pub fn reopen_tools_for_regate(&mut self) {
        self.tool_freeze = ToolFreezePhase::Gating;
        self.tool_fingerprint = None;
    }

    pub fn clear_tool_freeze(&mut self) {
        self.tool_freeze = ToolFreezePhase::Unfrozen;
        self.tool_fingerprint = None;
    }

    pub fn freeze_tools(&mut self, tools: ToolSet) {
        let fp = ToolTableFingerprint::from_toolset(&tools);
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.tool_fingerprint = Some(fp);
        self.tool_freeze = ToolFreezePhase::Frozen;
        self.rebuild_system_prompt();
    }

    pub fn freeze_tools_defer_prompt(&mut self, tools: ToolSet) -> SystemPromptOpts {
        let fp = ToolTableFingerprint::from_toolset(&tools);
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.tool_fingerprint = Some(fp);
        self.tool_freeze = ToolFreezePhase::Frozen;
        self.prompt_opts.clone()
    }

    pub fn frozen_fingerprint_matches_set(&self, candidate: &ToolSet) -> bool {
        match &self.tool_fingerprint {
            Some(fp) => fp.matches(&ToolTableFingerprint::from_toolset(candidate)),
            None => false,
        }
    }

    fn allow_tools_rewrite_now(&self, op: &str) -> bool {
        if self.has_active_turn() && !self.context_policy.allows_midturn_tools_rewrite() {
            log::warn!(
                target: "xylitol::agent",
                "{op} ignored: mid-turn tools rewrite denied by ContextPolicy (tools_mode={:?})",
                self.context_policy.tools_mode
            );
            return false;
        }
        true
    }

    pub fn install_system_prompt_text(&mut self, prompt: String) {
        let chars = prompt.len();
        let tool_n = self.prompt_opts.selected_tools.len();
        self.system_prompt.replace(prompt);
        log::debug!(
            target: "xylitol::lag",
            "install_system_prompt_text tools={tool_n} chars={chars}"
        );
    }

    fn apply_tools_metadata(&mut self, tools: &ToolSet) {
        self.prompt_opts.selected_tools = tools.iter().map(|t| t.name().to_string()).collect();
        self.prompt_opts.tool_snippets =
            prompt::collect_tool_snippets(tools, &self.prompt_opts.selected_tools);
        self.prompt_opts.prompt_guidelines =
            prompt::collect_tool_guidelines(tools, &self.prompt_opts.selected_tools);
    }

    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.hooks = hooks;
    }

    pub fn get_permission(&self) -> std::sync::Arc<dyn XyPermission> {
        self.permission.clone()
    }

    pub fn set_permission(&mut self, permission: std::sync::Arc<dyn XyPermission>) {
        self.permission = permission;
    }
}
