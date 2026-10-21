use super::AgentCapabilities;
use crate::agent::prompt;

impl AgentCapabilities {
    pub fn rebuild_system_prompt(&mut self) {
        let t0 = std::time::Instant::now();
        let tool_n = self.prompt_opts.selected_tools.len();
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

    pub fn apply_skills(&mut self, skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>) {
        self.prompt_opts.skills = skills;
        self.rebuild_system_prompt();
    }

    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.prompt_opts
            .skills
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    pub fn loaded_skills(&self) -> &[xylitol_plugin_sdk::resource::SkillInfo] {
        &self.prompt_opts.skills
    }

    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.prompt_opts.system_prompt = prompt.clone();
        self.system_prompt = prompt;
        self.rebuild_system_prompt();
    }
}
