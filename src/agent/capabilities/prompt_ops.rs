//! System prompt / skills / context-policy methods on [`AgentCapabilities`].

use super::AgentCapabilities;
use crate::agent::context_policy::DatePlacement;
use crate::agent::prompt;

impl AgentCapabilities {
    // ── Dynamic system prompt ────────────────────────────────────

    /// Rebuild the system prompt from current options.
    ///
    /// Applies [`ContextPolicy::date_placement`](crate::agent::context_policy::ContextPolicy):
    /// pins calendar day for `SystemPinnedAtSession`, refreshes for `SystemAsToday`,
    /// omits the date line for `Omit`.
    pub fn rebuild_system_prompt(&mut self) {
        let t0 = std::time::Instant::now();
        let tool_n = self.prompt_opts.selected_tools.len();
        self.apply_date_placement_before_build();
        self.system_prompt = Some(prompt::build_system_prompt(&self.prompt_opts));
        let ms = t0.elapsed().as_millis();
        let chars = self.system_prompt.as_ref().map(|s| s.len()).unwrap_or(0);
        if ms >= 80 {
            log::warn!(
                target: "xylitol::lag",
                "rebuild_system_prompt {ms}ms tools={tool_n} chars={chars}"
            );
        } else if ms >= 16 {
            log::info!(
                target: "xylitol::lag",
                "rebuild_system_prompt {ms}ms tools={tool_n} chars={chars}"
            );
        } else {
            log::debug!(
                target: "xylitol::lag",
                "rebuild_system_prompt {ms}ms tools={tool_n} chars={chars}"
            );
        }
    }

    /// Resolve `date_placement` into `prompt_opts` before assemble (c1905).
    fn apply_date_placement_before_build(&mut self) {
        let placement = self.context_policy.date_placement;
        self.prompt_opts.date_placement = placement;
        match placement {
            DatePlacement::SystemPinnedAtSession => {
                if self.system_date_pin.is_none() {
                    let pin = self
                        .prompt_opts
                        .date
                        .clone()
                        .unwrap_or_else(crate::utils::today_yyyy_mm_dd);
                    self.system_date_pin = Some(pin);
                }
                self.prompt_opts.date = self.system_date_pin.clone();
            }
            DatePlacement::SystemAsToday => {
                // Leave explicit inject for tests; production leaves `date: None` → today.
            }
            DatePlacement::Omit => {
                // build_system_prompt ignores date when Omit.
            }
        }
    }

    /// Ablation / lab (unit tests): set session-pinned calendar day for
    /// [`DatePlacement::SystemPinnedAtSession`] and rebuild.
    ///
    /// Product default is [`DatePlacement::Omit`] (session_env user-fold); Driver
    /// resume does **not** call this.
    #[cfg(test)]
    pub(crate) fn restore_system_date_pin(&mut self, pin: impl Into<String>) {
        self.system_date_pin = Some(pin.into());
        self.rebuild_system_prompt();
    }

    /// Replace context / SYSTEM / APPEND resources and rebuild the system prompt (c1100).
    ///
    /// Affects the **next** `run` only. Does **not** mutate session history or store entries.
    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.prompt_opts.context_files = context_files;
        self.prompt_opts.system_prompt = system_prompt;
        self.prompt_opts.append_system_prompt = append_system_prompt;
        self.rebuild_system_prompt();
    }

    /// Replace the skills catalog in the system prompt (c1085).
    ///
    /// Affects the **next** `run` only. Does **not** mutate session history.
    pub fn apply_skills(&mut self, skills: Vec<crate::protocol::resource::SkillInfo>) {
        self.prompt_opts.skills = skills;
        self.rebuild_system_prompt();
    }

    /// Names of skills currently injected into the system prompt (for `$` expand / reload).
    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.prompt_opts
            .skills
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    /// Full skill catalog (paths for `$` SKILL.md expand; c1130).
    pub fn loaded_skills(&self) -> &[crate::protocol::resource::SkillInfo] {
        &self.prompt_opts.skills
    }

    /// Set the active system prompt text and rebuild.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.prompt_opts.system_prompt = prompt.clone();
        self.system_prompt = prompt;
        self.rebuild_system_prompt();
    }
}
