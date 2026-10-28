use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::{
    AfterLlmResponseCtx, AfterToolCtx, BeforeLlmRequestCtx, BeforeToolCtx, DecideContinueCtx,
    LoopMiddleware, LoopStage, LoopStep,
};

use crate::config::{
    GeneratorCriticConfig, LoopPoliciesConfig, SelfHealConfig, ToolExposureConfig,
};

const STAGES: &[LoopStage] = &[LoopStage::BeforeTool, LoopStage::DecideContinue];
const TOOL_EXPOSURE_STAGES: &[LoopStage] = &[LoopStage::BeforeLlmRequest];
const SELF_HEAL_STAGES: &[LoopStage] = &[LoopStage::AfterTool, LoopStage::DecideContinue];
const CRITIC_STAGES: &[LoopStage] = &[
    LoopStage::BeforeLlmRequest,
    LoopStage::AfterLlmResponse,
    LoopStage::DecideContinue,
];
const DEFAULT_CRITIC_RUBRIC: &[&str] = &[
    "correctness: the answer matches what the tool results and code actually show",
    "evidence: cite concrete tool results, file paths or commands",
    "boundaries: call out unverified assumptions and missing checks",
    "verifiability: state how the result can be verified",
];
fn preview_text(parts: &[AgentPart], limit: usize) -> String {
    let text = parts
        .iter()
        .find_map(|part| match part {
            AgentPart::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let mut out: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        out.push('…');
    }
    out
}

const DEFAULT_SELF_HEAL_SIGNATURES: &[&str] = &[
    "test failed",
    "tests failed",
    "assertion failed",
    "error[e",
    "compilation failed",
    "lint failed",
    "panicked at",
];

pub struct LoopPoliciesMiddleware {
    config: LoopPoliciesConfig,
}

impl LoopPoliciesMiddleware {
    pub fn new(config: LoopPoliciesConfig) -> Self {
        Self { config }
    }

    fn denies(&self, command: &str) -> bool {
        self.config
            .deny_bash_patterns
            .iter()
            .any(|pattern| command.contains(pattern.as_str()))
    }
}

#[async_trait]
impl LoopMiddleware for LoopPoliciesMiddleware {
    fn id(&self) -> &str {
        "loop-policies"
    }

    fn order(&self) -> i64 {
        100
    }

    fn stages(&self) -> &'static [LoopStage] {
        STAGES
    }

    async fn before_tool(&self, mut ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        if ctx.tool_name == "bash"
            && let Some(command) = ctx.args.get("command").and_then(Value::as_str)
            && self.denies(command)
        {
            let denied_tool = ctx.tool_name.clone();
            ctx.args = serde_json::json!({
                "command": "true",
                "blocked_by": "loop-policies",
            });
            return LoopStep::stopped(ctx)
                .with_property("loop.middleware.loop_policies.denied_tool", denied_tool);
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let step = match self.config.max_settles {
            Some(limit) if ctx.settle_count >= limit => LoopStep::stopped(ctx),
            _ => LoopStep::continued(ctx),
        };
        step.with_property(
            "loop.middleware.loop_policies.max_settles",
            self.config
                .max_settles
                .map(|limit| limit.to_string())
                .unwrap_or_else(|| "none".into()),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CriticPhase {
    Idle,
    Critiquing,
    Revising,
    Approved,
    Exhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CriticVerdict {
    Approve,
    Revise,
}

#[derive(Debug)]
struct CriticState {
    phase: CriticPhase,
    rounds: u32,
    revision: Option<String>,
    unparsed: u32,
    instruction_injected: bool,
}

impl Default for CriticState {
    fn default() -> Self {
        Self {
            phase: CriticPhase::Idle,
            rounds: 0,
            revision: None,
            unparsed: 0,
            instruction_injected: false,
        }
    }
}

pub struct GeneratorCriticMiddleware {
    rubric: Vec<String>,
    max_rounds: u32,
    state: Mutex<CriticState>,
}

impl GeneratorCriticMiddleware {
    pub fn new(config: &GeneratorCriticConfig) -> Self {
        let rubric = if config.rubric.is_empty() {
            DEFAULT_CRITIC_RUBRIC
                .iter()
                .map(|item| item.to_string())
                .collect()
        } else {
            config.rubric.clone()
        };
        Self {
            rubric,
            max_rounds: config.max_rounds.max(1),
            state: Mutex::new(CriticState::default()),
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, CriticState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn critic_instruction(&self) -> String {
        let rubric = self
            .rubric
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "<critic>\nReview the previous assistant answer against this rubric:\n{rubric}\n\
Cite concrete evidence (tool results, file paths or commands). Do not modify files or call tools.\n\
End with exactly one line:\nCRITIC_VERDICT: APPROVE\nor\nCRITIC_VERDICT: REVISE\n</critic>"
        )
    }

    fn revision_instruction(&self, revision: Option<&str>) -> String {
        let revision = revision.unwrap_or("No structured verdict was returned.");
        let mut excerpt: String = revision.chars().take(2000).collect();
        if revision.chars().count() > 2000 {
            excerpt.push('…');
        }
        format!(
            "<critic_revision>\nThe critic requested a revision:\n{excerpt}\n\
Revise the answer and address each issue. Do not claim completion without evidence.\n</critic_revision>"
        )
    }
}

fn assistant_has_tool_calls(message: &AgentMessage) -> bool {
    message
        .content()
        .iter()
        .any(|part| matches!(part, AgentPart::ToolCall { .. }))
}

fn parse_critic_verdict(text: &str) -> Option<CriticVerdict> {
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(idx) = lower.find("critic_verdict:") {
            let rest = &line[idx + "critic_verdict:".len()..];
            if let Some(verdict) = verdict_from_token(rest.trim()) {
                return Some(verdict);
            }
        }
    }
    None
}

fn verdict_from_token(token: &str) -> Option<CriticVerdict> {
    let token = token
        .trim()
        .trim_matches(|c: char| !c.is_ascii_alphabetic());
    let upper = token.to_ascii_uppercase();
    if upper.starts_with("APPROVE") {
        Some(CriticVerdict::Approve)
    } else if upper.starts_with("REVIS") {
        Some(CriticVerdict::Revise)
    } else {
        None
    }
}

#[async_trait]
impl LoopMiddleware for GeneratorCriticMiddleware {
    fn id(&self) -> &str {
        "generator-critic"
    }

    fn order(&self) -> i64 {
        170
    }

    fn stages(&self) -> &'static [LoopStage] {
        CRITIC_STAGES
    }

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
        let mut state = self.lock_state();
        if state.instruction_injected {
            return LoopStep::continued(ctx);
        }
        match state.phase {
            CriticPhase::Critiquing => {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    self.critic_instruction(),
                )));
                state.instruction_injected = true;
            }
            CriticPhase::Revising => {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    self.revision_instruction(state.revision.as_deref()),
                )));
                state.instruction_injected = true;
            }
            _ => {}
        }
        LoopStep::continued(ctx)
    }

    async fn after_llm_response(&self, ctx: AfterLlmResponseCtx) -> LoopStep<AfterLlmResponseCtx> {
        let mut state = self.lock_state();
        match state.phase {
            CriticPhase::Critiquing => {
                let text = ctx.assistant.text();
                match parse_critic_verdict(&text) {
                    Some(CriticVerdict::Approve) => {
                        state.phase = CriticPhase::Approved;
                        state.revision = None;
                        state.unparsed = 0;
                    }
                    Some(CriticVerdict::Revise) => {
                        state.phase = CriticPhase::Revising;
                        state.revision = Some(text);
                        state.unparsed = 0;
                        state.instruction_injected = false;
                    }
                    None => {
                        state.unparsed += 1;
                        if state.unparsed >= 2 {
                            state.phase = CriticPhase::Exhausted;
                        } else {
                            state.phase = CriticPhase::Revising;
                            state.revision = None;
                            state.instruction_injected = false;
                        }
                    }
                }
            }
            CriticPhase::Idle | CriticPhase::Revising => {
                if assistant_has_tool_calls(&ctx.assistant) {
                    return LoopStep::continued(ctx);
                }
                if state.rounds >= self.max_rounds {
                    state.phase = CriticPhase::Exhausted;
                } else {
                    state.rounds += 1;
                    state.phase = CriticPhase::Critiquing;
                    state.revision = None;
                    state.instruction_injected = false;
                }
            }
            _ => {}
        }
        let verdict = match state.phase {
            CriticPhase::Approved => "approve",
            CriticPhase::Revising => "revise",
            CriticPhase::Exhausted => "exhausted",
            CriticPhase::Critiquing => "critiquing",
            CriticPhase::Idle => "idle",
        };
        LoopStep::continued(ctx).with_properties([
            (
                "loop.middleware.critic.verdict".to_string(),
                verdict.to_string(),
            ),
            (
                "loop.middleware.critic.round".to_string(),
                state.rounds.to_string(),
            ),
        ])
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let state = self.lock_state();
        match state.phase {
            CriticPhase::Critiquing | CriticPhase::Revising => LoopStep::continued(ctx),
            CriticPhase::Approved => LoopStep::stopped(ctx),
            CriticPhase::Exhausted => {
                log::warn!(
                    target: "xylitol::loop-policies",
                    "generator-critic exhausted after {} round(s)",
                    state.rounds
                );
                LoopStep::stopped(ctx)
            }
            CriticPhase::Idle => LoopStep::continued(ctx),
        }
    }
}

pub fn generator_critic_middleware(config: &GeneratorCriticConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(GeneratorCriticMiddleware::new(config))
}

#[derive(Clone, Debug)]
struct PendingFailure {
    tool_name: String,
    signature: String,
    attempts: u32,
}

pub struct SelfHealMiddleware {
    signatures: Vec<String>,
    max_attempts: u32,
    pending: Mutex<Option<PendingFailure>>,
}

impl SelfHealMiddleware {
    pub fn new(config: &SelfHealConfig) -> Self {
        let signatures = if config.signatures.is_empty() {
            DEFAULT_SELF_HEAL_SIGNATURES
                .iter()
                .map(|signature| signature.to_string())
                .collect()
        } else {
            config
                .signatures
                .iter()
                .map(|signature| signature.to_lowercase())
                .collect()
        };
        Self {
            signatures,
            max_attempts: config.max_attempts.max(1),
            pending: Mutex::new(None),
        }
    }

    fn lock_pending(&self) -> std::sync::MutexGuard<'_, Option<PendingFailure>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[async_trait]
impl LoopMiddleware for SelfHealMiddleware {
    fn id(&self) -> &str {
        "self-heal"
    }

    fn order(&self) -> i64 {
        210
    }

    fn stages(&self) -> &'static [LoopStage] {
        SELF_HEAL_STAGES
    }

    async fn after_tool(&self, mut ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        let mut pending = self.lock_pending();
        if ctx.is_error {
            let text = preview_text(&ctx.parts, 4000).to_lowercase();
            if let Some(signature) = self
                .signatures
                .iter()
                .find(|signature| text.contains(signature.as_str()))
                .cloned()
            {
                let attempts = match pending.as_ref() {
                    Some(previous)
                        if previous.tool_name == ctx.tool_name
                            && previous.signature == signature =>
                    {
                        previous.attempts.saturating_add(1)
                    }
                    _ => 1,
                };
                *pending = Some(PendingFailure {
                    tool_name: ctx.tool_name.clone(),
                    signature: signature.clone(),
                    attempts,
                });
                ctx.parts.push(AgentPart::text(format!(
                    "<self_heal>\nDeterministic failure detected ({signature}). \\
Do not repeat the same action; diagnose, patch, then re-run the same verification.\n</self_heal>"
                )));
                return LoopStep::continued(ctx).with_properties([
                    ("loop.middleware.self_heal.signature".to_string(), signature),
                    (
                        "loop.middleware.self_heal.attempt".to_string(),
                        attempts.to_string(),
                    ),
                ]);
            }
        } else if pending
            .as_ref()
            .is_some_and(|previous| previous.tool_name == ctx.tool_name)
        {
            *pending = None;
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let pending = self.lock_pending();
        match pending.as_ref() {
            Some(failure) if failure.attempts >= self.max_attempts => {
                log::warn!(
                    target: "xylitol::loop-policies",
                    "self-heal stopped after {} attempts on {} ({})",
                    failure.attempts,
                    failure.tool_name,
                    failure.signature
                );
                LoopStep::stopped(ctx).with_properties([
                    (
                        "loop.middleware.self_heal.signature".to_string(),
                        failure.signature.clone(),
                    ),
                    (
                        "loop.middleware.self_heal.attempt".to_string(),
                        failure.attempts.to_string(),
                    ),
                ])
            }
            _ => LoopStep::continued(ctx),
        }
    }
}

pub fn self_heal_middleware(config: &SelfHealConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(SelfHealMiddleware::new(config))
}

pub struct ToolExposureMiddleware {
    config: ToolExposureConfig,
}

impl ToolExposureMiddleware {
    pub fn new(config: ToolExposureConfig) -> Self {
        Self { config }
    }

    fn visible(&self, name: &str) -> bool {
        if matches!(name, "todo_rewrite" | "todo_update") {
            return true;
        }
        if self.config.deny.iter().any(|tool| tool == name) {
            return false;
        }
        if self.config.read_only && self.config.write_tools.iter().any(|tool| tool == name) {
            return false;
        }
        if !self.config.allow.is_empty() && !self.config.allow.iter().any(|tool| tool == name) {
            return false;
        }
        true
    }
}

#[async_trait]
impl LoopMiddleware for ToolExposureMiddleware {
    fn id(&self) -> &str {
        "tool-exposure"
    }

    fn order(&self) -> i64 {
        50
    }

    fn stages(&self) -> &'static [LoopStage] {
        TOOL_EXPOSURE_STAGES
    }

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
        if !self.config.enabled {
            return LoopStep::continued(ctx);
        }
        let before = ctx.tool_schemas.len();
        ctx.tool_schemas.retain(|schema| self.visible(&schema.name));
        let after = ctx.tool_schemas.len();
        if after != before {
            log::info!(
                target: "xylitol::loop-policies",
                "tool exposure filtered {} of {} tools",
                before - after,
                before
            );
        }
        LoopStep::continued(ctx).with_properties([
            (
                "loop.middleware.tools.before".to_string(),
                before.to_string(),
            ),
            ("loop.middleware.tools.after".to_string(), after.to_string()),
        ])
    }
}

pub fn tool_exposure_middleware(config: &ToolExposureConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(ToolExposureMiddleware::new(config.clone()))
}

pub fn loop_policies_middleware(config: &LoopPoliciesConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(LoopPoliciesMiddleware::new(config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        DEFAULT_CRITIC_ROUNDS, DEFAULT_SELF_HEAL_ATTEMPTS, GeneratorCriticConfig, SelfHealConfig,
    };
    use xylitol_plugin_sdk::LoopControl;
    use xylitol_plugin_sdk::message::XyStopReason;
    use xylitol_plugin_sdk::model::XyToolSchema;
    use xylitol_plugin_sdk::session::TodoList;

    fn ctx(command: &str) -> BeforeToolCtx {
        BeforeToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({ "command": command }),
        }
    }

    #[tokio::test]
    async fn denies_configured_pattern_only() {
        let config = LoopPoliciesConfig {
            enabled: true,
            max_settles: None,
            deny_bash_patterns: vec!["rm -rf /".into()],
            self_heal: Default::default(),
            generator_critic: Default::default(),
            tool_exposure: Default::default(),
        };
        let mw = LoopPoliciesMiddleware::new(config);
        let safe = mw.before_tool(ctx("echo hi")).await;
        assert_eq!(safe.control, LoopControl::Continue);
        let risky = mw.before_tool(ctx("rm -rf / --no-preserve-root")).await;
        assert_eq!(risky.control, LoopControl::Stop);
        assert_eq!(risky.ctx.args["blocked_by"], "loop-policies");
        assert!(
            risky.properties.iter().any(|(key, value)| {
                key == "loop.middleware.loop_policies.denied_tool" && value == "bash"
            }),
            "denial must report the denied tool"
        );
    }

    #[tokio::test]
    async fn stops_at_configured_settle_count() {
        let config = LoopPoliciesConfig {
            enabled: true,
            max_settles: Some(2),
            deny_bash_patterns: Vec::new(),
            self_heal: Default::default(),
            generator_critic: Default::default(),
            tool_exposure: Default::default(),
        };
        let mw = LoopPoliciesMiddleware::new(config);
        let base = DecideContinueCtx {
            turn_index: 1,
            settle_count: 1,
            has_new_user_messages: false,
            assistant: None,
            history_len: 2,
            compact: None,
        };
        assert_eq!(
            mw.decide_continue(base.clone()).await.control,
            LoopControl::Continue
        );
        let mut at_limit = base;
        at_limit.settle_count = 2;
        assert_eq!(
            mw.decide_continue(at_limit).await.control,
            LoopControl::Stop
        );
    }

    #[test]
    fn stages_and_identity_are_stable() {
        let mw = LoopPoliciesMiddleware::new(LoopPoliciesConfig::default());
        assert_eq!(mw.id(), "loop-policies");
        assert_eq!(mw.order(), 100);
        assert_eq!(mw.stages(), STAGES);
    }

    fn schema(name: &str) -> XyToolSchema {
        XyToolSchema {
            name: name.into(),
            description: format!("{name} tool"),
            parameters: serde_json::json!({ "type": "object" }),
        }
    }

    fn schemas(names: &[&str]) -> Vec<XyToolSchema> {
        names.iter().map(|name| schema(name)).collect()
    }

    fn exposure_ctx(names: &[&str]) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: schemas(names),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            plan_mode: None,
            session_id: None,
        }
    }

    fn visible_names(ctx: &BeforeLlmRequestCtx) -> Vec<String> {
        ctx.tool_schemas
            .iter()
            .map(|schema| schema.name.clone())
            .collect()
    }

    #[tokio::test]
    async fn tool_exposure_disabled_keeps_shape() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig::default());
        let out = mw
            .before_llm_request(exposure_ctx(&["read", "bash", "todo_rewrite"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read", "bash", "todo_rewrite"]);
    }

    #[tokio::test]
    async fn tool_exposure_empty_rules_keep_shape() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            ..Default::default()
        });
        let out = mw.before_llm_request(exposure_ctx(&["read", "bash"])).await;
        assert_eq!(visible_names(&out.ctx), ["read", "bash"]);
    }

    #[tokio::test]
    async fn tool_exposure_allow_keeps_todo_tools_visible() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            allow: vec!["read".into()],
            ..Default::default()
        });
        let out = mw
            .before_llm_request(exposure_ctx(&[
                "read",
                "bash",
                "todo_rewrite",
                "todo_update",
            ]))
            .await;
        assert_eq!(
            visible_names(&out.ctx),
            ["read", "todo_rewrite", "todo_update"],
            "todo tools must stay visible so the plan can be updated"
        );
        let get = |key: &str| {
            out.properties
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("loop.middleware.tools.before"), Some("4"));
        assert_eq!(get("loop.middleware.tools.after"), Some("3"));
    }

    #[tokio::test]
    async fn tool_exposure_deny_hides_only_listed_tools() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            deny: vec!["bash".into()],
            ..Default::default()
        });
        let out = mw
            .before_llm_request(exposure_ctx(&["read", "bash", "write"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read", "write"]);
    }

    #[tokio::test]
    async fn tool_exposure_read_only_hides_write_tools() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            read_only: true,
            write_tools: vec!["write".into(), "edit".into()],
            ..Default::default()
        });
        let out = mw
            .before_llm_request(exposure_ctx(&["read", "write", "edit"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read"]);
    }

    #[tokio::test]
    async fn tool_exposure_deny_wins_over_allow() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            allow: vec!["read".into(), "bash".into()],
            deny: vec!["bash".into()],
            ..Default::default()
        });
        let out = mw
            .before_llm_request(exposure_ctx(&["read", "bash", "todo_update"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read", "todo_update"]);
    }

    #[test]
    fn tool_exposure_identity_is_stable() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig::default());
        assert_eq!(mw.id(), "tool-exposure");
        assert_eq!(mw.order(), 50);
        assert_eq!(mw.stages(), TOOL_EXPOSURE_STAGES);
    }

    fn heal_ctx(name: &str, text: &str, is_error: bool) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error,
        }
    }

    #[tokio::test]
    async fn self_heal_detects_deterministic_failure() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            ..Default::default()
        });
        let out = mw
            .after_tool(heal_ctx("bash", "test failed: two assertions", true))
            .await;
        assert!(
            out.ctx.parts.iter().any(
                |part| matches!(part, AgentPart::Text { text } if text.contains("<self_heal>"))
            ),
            "a deterministic failure must add repair guidance"
        );
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue,
            "a pending failure must force another model call"
        );
    }

    #[tokio::test]
    async fn self_heal_ignores_non_deterministic_failure() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            ..Default::default()
        });
        let out = mw
            .after_tool(heal_ctx("bash", "network timeout", true))
            .await;
        assert!(
            !out.ctx.parts.iter().any(
                |part| matches!(part, AgentPart::Text { text } if text.contains("<self_heal>"))
            )
        );
        assert!(mw.pending.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn self_heal_clears_after_successful_verification() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            ..Default::default()
        });
        mw.after_tool(heal_ctx("bash", "tests failed", true)).await;
        assert!(mw.pending.lock().unwrap().is_some());
        mw.after_tool(heal_ctx("bash", "all tests passed", false))
            .await;
        assert!(
            mw.pending.lock().unwrap().is_none(),
            "re-running the failing tool successfully must clear the pending failure"
        );
    }

    #[tokio::test]
    async fn self_heal_stops_at_attempt_cap() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            max_attempts: 2,
            ..Default::default()
        });
        mw.after_tool(heal_ctx("bash", "tests failed", true)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        mw.after_tool(heal_ctx("bash", "tests failed", true)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "self-heal must be bounded"
        );
    }

    #[tokio::test]
    async fn self_heal_custom_signature_is_case_insensitive() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            signatures: vec!["BUILD BROKEN".into()],
            ..Default::default()
        });
        mw.after_tool(heal_ctx("bash", "build broken: missing symbol", true))
            .await;
        assert!(mw.pending.lock().unwrap().is_some());
    }

    #[test]
    fn self_heal_identity_is_stable() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig::default());
        assert_eq!(mw.id(), "self-heal");
        assert_eq!(mw.order(), 210);
        assert_eq!(mw.stages(), SELF_HEAL_STAGES);
    }

    #[test]
    fn self_heal_default_attempt_cap_is_below_core_limit() {
        assert_eq!(DEFAULT_SELF_HEAL_ATTEMPTS, 3);
        assert!(DEFAULT_SELF_HEAL_ATTEMPTS < 32);
    }

    fn model_ctx(todo: Option<TodoList>, last: Option<AgentMessage>) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 0,
            messages: last.into_iter().collect(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo,
            plan_mode: None,
            session_id: None,
        }
    }

    fn decide_ctx(settle_count: u32, has_new_user_messages: bool) -> DecideContinueCtx {
        DecideContinueCtx {
            turn_index: 0,
            settle_count,
            has_new_user_messages,
            assistant: None,
            history_len: 2,
            compact: None,
        }
    }

    fn critic_config(max_rounds: u32) -> GeneratorCriticConfig {
        GeneratorCriticConfig {
            enabled: true,
            max_rounds,
            ..Default::default()
        }
    }

    fn model_after(text: &str) -> AfterLlmResponseCtx {
        AfterLlmResponseCtx {
            turn_index: 0,
            assistant: AgentMessage::Llm(XyLlmMessage::assistant(text)),
            session_id: None,
        }
    }

    fn model_after_tool_call() -> AfterLlmResponseCtx {
        AfterLlmResponseCtx {
            turn_index: 0,
            assistant: AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({ "path": "a.rs" }),
                }],
                stop_reason: Some(XyStopReason::ToolUse),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            }),
            session_id: None,
        }
    }

    #[tokio::test]
    async fn critic_approve_stops_the_run() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_llm_response(model_after("final answer")).await;
        let injected = mw.before_llm_request(model_ctx(None, None)).await;
        assert_eq!(injected.ctx.messages.len(), 1);
        assert!(injected.ctx.messages[0].text().contains("<critic>"));
        mw.after_llm_response(model_after("CRITIC_VERDICT: APPROVE"))
            .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop
        );
    }

    #[tokio::test]
    async fn critic_revise_continues_and_injects_revision() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_llm_response(model_after("final answer")).await;
        mw.before_llm_request(model_ctx(None, None)).await;
        mw.after_llm_response(model_after(
            "CRITIC_VERDICT: REVISE\nThe answer cites no evidence.",
        ))
        .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        let injected = mw.before_llm_request(model_ctx(None, None)).await;
        assert_eq!(injected.ctx.messages.len(), 1);
        let text = injected.ctx.messages[0].text();
        assert!(text.contains("<critic_revision>"));
        assert!(text.contains("cites no evidence"));
        mw.after_llm_response(model_after("revised answer")).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "max_rounds=1 must stop after one revision"
        );
    }

    #[tokio::test]
    async fn critic_without_verdict_retries_once_then_stops() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(2));
        mw.after_llm_response(model_after("final answer")).await;
        mw.before_llm_request(model_ctx(None, None)).await;
        mw.after_llm_response(model_after("no structured verdict"))
            .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        mw.before_llm_request(model_ctx(None, None)).await;
        mw.after_llm_response(model_after("revised answer")).await;
        mw.before_llm_request(model_ctx(None, None)).await;
        mw.after_llm_response(model_after("still no verdict")).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "a second missing verdict must stop"
        );
    }

    #[tokio::test]
    async fn critic_ignores_tool_call_turns() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_llm_response(model_after_tool_call()).await;
        let out = mw.before_llm_request(model_ctx(None, None)).await;
        assert!(
            out.ctx.messages.is_empty(),
            "tool-call turns must not start a critique"
        );
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
    }

    #[tokio::test]
    async fn critic_injects_instruction_only_once_per_phase() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_llm_response(model_after("final answer")).await;
        let first = mw.before_llm_request(model_ctx(None, None)).await;
        assert_eq!(first.ctx.messages.len(), 1);
        let second = mw.before_llm_request(model_ctx(None, None)).await;
        assert!(
            second.ctx.messages.is_empty(),
            "the critic instruction must not be repeated within one phase"
        );
    }

    #[test]
    fn critic_identity_is_stable() {
        let mw = GeneratorCriticMiddleware::new(&GeneratorCriticConfig::default());
        assert_eq!(mw.id(), "generator-critic");
        assert_eq!(mw.order(), 170);
        assert_eq!(mw.stages(), CRITIC_STAGES);
    }

    #[test]
    fn critic_default_rounds_are_bounded() {
        assert_eq!(DEFAULT_CRITIC_ROUNDS, 1);
        assert!(DEFAULT_CRITIC_ROUNDS < 32);
    }
}
