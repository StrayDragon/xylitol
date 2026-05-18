//! Planning-Execution split: Architect → Editor → Validator flow.
//!
//! Feature: `agent-planning`.
//!
//! ## Architecture
//!
//! [MermaidChart:./docs/mmd/c55-orchestration.mmd]
//!
//! - **Planner** (strong model) decomposes tasks into structured JSON step plans.
//! - **Executor** (fast model) executes each step with access to tools.
//! - **Validator** runs static checks (compile/lint/test) on each step's output.
//! - **Orchestrator** ties the three together with retry and fallback logic.

use std::pin::Pin;
use std::sync::Arc;

use adk_core::{Content, Llm, LlmRequest, LlmResponse};
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::infra::config::types::{AppConfig, PlanningConfig, ValidationConfig};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during the planning-execution pipeline.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PlannerError {
    #[error("LLM call failed: {0}")]
    LlmError(String),

    #[error("Plan parsing failed: {0}")]
    ParseError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Step execution failed: {0}")]
    StepError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Max retries exceeded for step {step}")]
    MaxRetriesExceeded { step: u32 },
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single step in an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlanStep {
    pub step: u32,
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

/// A complete execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExecutionPlan {
    pub steps: Vec<PlanStep>,
}

/// Result of executing a single plan step.
#[derive(Debug, Clone)]
pub(crate) struct StepResult {
    pub step: u32,
    pub summary: String,
    pub success: bool,
}

/// Result from the validator.
#[derive(Debug, Clone)]
pub(crate) struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Prompt templates
// ---------------------------------------------------------------------------

/// Architect prompt — task decomposition with structured JSON output.
const ARCHITECT_PROMPT: &str = r#"You are an expert software architect. Your task is to analyze the given problem and decompose it into a structured, step-by-step execution plan.

Guidelines:
- Break down the task into the smallest meaningful steps
- Each step should produce a verifiable outcome
- Specify which tools each step needs (e.g., read, write, edit, bash, grep, find)
- Output ONLY valid JSON matching this schema: {"steps":[{"step":1,"description":"...","tools":["tool1"]}]}
- Use sequential step numbers starting from 1
- Keep steps focused and actionable"#;

/// Editor prompt — step-by-step execution without deviation.
const EDITOR_PROMPT: &str = r#"You are an expert software engineer executing a planned change. You must strictly follow the given plan step without adding extra scope or creative modifications.

Guidelines:
- Focus only on the current step's description
- Make minimal, precise changes
- Verify your changes before completing
- If you encounter unexpected issues, report them clearly
- Do NOT deviate from the specified step"#;

/// Return the reasoning instruction for the given depth.
fn reasoning_instruction(depth: &str) -> &'static str {
    match depth {
        "deep" => {
            "Think through multiple approaches, compare trade-offs, then pick the best \
             solution. Provide your reasoning in detail before concluding."
        }
        "quick" => "Make a direct, practical decision. Minimize deliberation.",
        _ => {
            "Balance thoroughness with efficiency. Consider alternatives briefly \
             before deciding."
        }
    }
}

// ---------------------------------------------------------------------------
// Planner
// ---------------------------------------------------------------------------

/// The Planner role — uses a strong model to decompose tasks into structured plans.
pub(crate) struct Planner {
    model: Arc<dyn Llm>,
    model_name: String,
    system_prompt: String,
    reasoning_depth: String,
}

impl Planner {
    pub(crate) fn new(model: Arc<dyn Llm>, model_name: String, config: &PlanningConfig) -> Self {
        Self {
            model,
            model_name,
            system_prompt: config
                .system_prompt
                .clone()
                .unwrap_or_else(|| "architect".into()),
            reasoning_depth: config.reasoning_depth.clone(),
        }
    }

    /// Decompose a task into an execution plan.
    pub(crate) async fn plan(&self, task: &str) -> Result<ExecutionPlan, PlannerError> {
        let depth_instr = reasoning_instruction(&self.reasoning_depth);
        let prompt = format!(
            "{ARCHITECT_PROMPT}\n\n{system}\n\nReasoning: {depth}\n\nTask: {task}",
            system = self.system_prompt,
            depth = depth_instr,
        );

        let text = call_llm(self.model.as_ref(), &self.model_name, &prompt).await?;

        let json_str = extract_json(&text)
            .ok_or_else(|| PlannerError::ParseError("no JSON found in planner response".into()))?;

        let plan: ExecutionPlan = serde_json::from_str(&json_str)
            .map_err(|e| PlannerError::ParseError(format!("invalid plan JSON: {e}")))?;

        if plan.steps.is_empty() {
            return Err(PlannerError::ParseError("plan has zero steps".into()));
        }

        // Re-number steps sequentially.
        let steps: Vec<PlanStep> = plan
            .steps
            .into_iter()
            .enumerate()
            .map(|(i, mut s)| {
                s.step = (i + 1) as u32;
                s
            })
            .collect();

        Ok(ExecutionPlan { steps })
    }
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/// The Executor role — uses a fast model to execute individual plan steps.
pub(crate) struct Executor {
    model: Arc<dyn Llm>,
    model_name: String,
    system_prompt: String,
    max_retries: u8,
}

impl Executor {
    pub(crate) fn new(
        model: Arc<dyn Llm>,
        model_name: String,
        system_prompt: Option<String>,
        max_retries: u8,
    ) -> Self {
        Self {
            model,
            model_name,
            system_prompt: system_prompt.unwrap_or_else(|| "editor".into()),
            max_retries,
        }
    }

    /// Execute a single plan step with the given context.
    pub(crate) async fn execute_step(
        &self,
        step: &PlanStep,
        context: &str,
    ) -> Result<StepResult, PlannerError> {
        let prompt = format!(
            "{EDITOR_PROMPT}\n\nSystem: {system}\n\nStep {n}: {desc}\n\nContext:\n{ctx}",
            system = self.system_prompt,
            n = step.step,
            desc = step.description,
            ctx = context,
        );

        let text = call_llm(self.model.as_ref(), &self.model_name, &prompt).await?;

        Ok(StepResult {
            step: step.step,
            summary: truncate_summary(&text),
            success: true,
        })
    }

    pub(crate) fn max_retries(&self) -> u8 {
        self.max_retries
    }
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

/// The Validator role — runs static check commands (compile/lint/test).
pub(crate) struct Validator {
    commands: Vec<String>,
}

impl Validator {
    pub(crate) fn new(commands: Vec<String>) -> Self {
        Self { commands }
    }

    pub(crate) fn from_config(config: &ValidationConfig) -> Self {
        let commands = if config.enabled {
            vec!["cargo check".into(), "cargo clippy".into()]
        } else {
            Vec::new()
        };
        Self { commands }
    }

    /// Run all validation commands. Returns Ok if all pass.
    pub(crate) async fn validate(&self) -> Result<ValidationResult, PlannerError> {
        if self.commands.is_empty() {
            return Ok(ValidationResult {
                passed: true,
                errors: Vec::new(),
            });
        }

        let mut errors = Vec::new();
        for cmd in &self.commands {
            tracing::info!("validator running: {cmd}");
            let output = tokio::process::Command::new("sh")
                .args(["-c", cmd])
                .output()
                .await
                .map_err(|e| PlannerError::ValidationError(format!("command error: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                errors.push(format!(
                    "Command '{cmd}' failed:\nstdout: {stdout}\nstderr: {stderr}"
                ));
            }
        }

        Ok(ValidationResult {
            passed: errors.is_empty(),
            errors,
        })
    }
}

// ---------------------------------------------------------------------------
// PlanningOrchestrator
// ---------------------------------------------------------------------------

/// The orchestrator that coordinates Planner → Executor → Validator flow.
///
/// Handles step sequencing, retries, and model fallback.
pub(crate) struct PlanningOrchestrator {
    planner: Planner,
    executor: Executor,
    validator: Validator,
    max_steps: u16,
    planner_fallback: Option<Arc<dyn Llm>>,
    planner_fallback_name: Option<String>,
    executor_fallback: Option<Arc<dyn Llm>>,
    executor_fallback_name: Option<String>,
}

impl PlanningOrchestrator {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        planner: Planner,
        executor: Executor,
        validator: Validator,
        max_steps: u16,
        planner_fallback: Option<Arc<dyn Llm>>,
        planner_fallback_name: Option<String>,
        executor_fallback: Option<Arc<dyn Llm>>,
        executor_fallback_name: Option<String>,
    ) -> Self {
        Self {
            planner,
            executor,
            validator,
            max_steps,
            planner_fallback,
            planner_fallback_name,
            executor_fallback,
            executor_fallback_name,
        }
    }

    /// Run the full planning-execution pipeline.
    pub(crate) async fn run(&self, task: &str) -> Result<Vec<StepResult>, PlannerError> {
        // Phase 1: Plan
        let plan = self.plan_with_fallback(task).await?;

        if plan.steps.len() > self.max_steps as usize {
            return Err(PlannerError::ConfigError(format!(
                "plan has {} steps, max is {}",
                plan.steps.len(),
                self.max_steps
            )));
        }

        // Phase 2: Execute + Validate per step
        let mut results = Vec::new();
        let mut context = task.to_string();

        for step in &plan.steps {
            tracing::info!("executing step {}/{}", step.step, plan.steps.len());

            let step_result = self.execute_step_with_retry(step, &context).await?;

            context = format!(
                "{}\n\nStep {} result: {}",
                context, step.step, step_result.summary
            );
            results.push(step_result);
        }

        Ok(results)
    }

    /// Plan with fallback support.
    async fn plan_with_fallback(&self, task: &str) -> Result<ExecutionPlan, PlannerError> {
        match self.planner.plan(task).await {
            Ok(plan) => Ok(plan),
            Err(e) => {
                tracing::warn!("planner failed, trying fallback: {e}");
                if let Some(ref fallback_model) = self.planner_fallback {
                    let fallback_name = self.planner_fallback_name.as_deref().unwrap_or("fallback");
                    Planner {
                        model: fallback_model.clone(),
                        model_name: fallback_name.to_string(),
                        system_prompt: "architect".into(),
                        reasoning_depth: "standard".into(),
                    }
                    .plan(task)
                    .await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Execute a step with retry and fallback.
    async fn execute_step_with_retry(
        &self,
        step: &PlanStep,
        context: &str,
    ) -> Result<StepResult, PlannerError> {
        let max_retries = self.executor.max_retries();

        for attempt in 0..=max_retries {
            let result = if attempt == 0 {
                self.executor.execute_step(step, context).await
            } else if let Some(ref fallback_model) = self.executor_fallback {
                let fallback_name = self.executor_fallback_name.as_deref().unwrap_or("fallback");
                tracing::warn!(
                    "executor attempt {attempt}/{max_retries} for step {}, trying fallback",
                    step.step
                );
                Executor {
                    model: fallback_model.clone(),
                    model_name: fallback_name.to_string(),
                    system_prompt: "editor".into(),
                    max_retries: 0,
                }
                .execute_step(step, context)
                .await
            } else {
                tracing::warn!(
                    "executor attempt {attempt}/{max_retries} for step {}",
                    step.step
                );
                self.executor.execute_step(step, context).await
            };

            match result {
                Ok(r) => {
                    // Validate after successful execution
                    let validation = self.validator.validate().await?;
                    if validation.passed {
                        return Ok(r);
                    }
                    tracing::warn!(
                        "step {} validation failed (attempt {}): {}",
                        step.step,
                        attempt,
                        validation.errors.join("; "),
                    );
                }
                Err(e) => {
                    tracing::warn!("step {} failed (attempt {}): {e}", step.step, attempt);
                }
            }
        }

        Err(PlannerError::MaxRetriesExceeded { step: step.step })
    }
}

// ---------------------------------------------------------------------------
// Fallback resolution
// ---------------------------------------------------------------------------

/// Resolve a fallback model for the given model ID.
///
/// Checks the model entry's `fallback` field. Returns `None` if no fallback
/// is configured or resolution fails.
fn resolve_fallback(
    model_id: &str,
    app_config: &AppConfig,
) -> (Option<Arc<dyn Llm>>, Option<String>) {
    let entry = match app_config.model.models.get(model_id) {
        Some(e) => e,
        None => return (None, None),
    };

    let fallback_id = match entry.fallback.as_ref() {
        Some(f) if !f.is_empty() && f != model_id => f.clone(),
        _ => return (None, None),
    };

    match build_model(&fallback_id, app_config) {
        Ok(model) => (Some(model), Some(fallback_id)),
        Err(e) => {
            tracing::warn!("failed to build fallback model '{fallback_id}': {e}");
            (None, None)
        }
    }
}

/// Build an LLM model reference from a model ID and app config.
fn build_model(model_id: &str, app_config: &AppConfig) -> Result<Arc<dyn Llm>, PlannerError> {
    let cfg = app_config
        .resolve_model(model_id)
        .map_err(PlannerError::ConfigError)?;
    cfg.build()
        .map_err(|e| PlannerError::ConfigError(format!("build model '{model_id}': {e}")))
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Build a `PlanningOrchestrator` from app config.
pub(crate) async fn build_orchestrator(
    app_config: &AppConfig,
) -> Result<PlanningOrchestrator, PlannerError> {
    let planning_config = app_config
        .planning
        .as_ref()
        .ok_or_else(|| PlannerError::ConfigError("planning config not found".into()))?;
    let validation_config = app_config
        .validation
        .as_ref()
        .ok_or_else(|| PlannerError::ConfigError("validation config not found".into()))?;

    // Resolve planning model
    let planner_model_id = planning_config
        .model
        .as_deref()
        .unwrap_or(&app_config.model.default_model);
    let planner_model = build_model(planner_model_id, app_config)?;
    let planner = Planner::new(planner_model, planner_model_id.to_string(), planning_config);

    // Resolve execution model
    let executor_model_id = app_config
        .execution
        .model
        .as_deref()
        .unwrap_or(&app_config.model.default_model);
    let executor_model = build_model(executor_model_id, app_config)?;
    let executor = Executor::new(
        executor_model,
        executor_model_id.to_string(),
        app_config.execution.system_prompt.clone(),
        app_config.execution.max_retries,
    );

    // Build validator
    let validator = Validator::from_config(validation_config);

    // Resolve fallbacks
    let (planner_fallback, planner_fallback_name) = resolve_fallback(planner_model_id, app_config);
    let (executor_fallback, executor_fallback_name) =
        resolve_fallback(executor_model_id, app_config);

    Ok(PlanningOrchestrator::new(
        planner,
        executor,
        validator,
        planning_config.max_steps,
        planner_fallback,
        planner_fallback_name,
        executor_fallback,
        executor_fallback_name,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Call an LLM with a single user prompt and return the text response.
async fn call_llm(model: &dyn Llm, model_name: &str, prompt: &str) -> Result<String, PlannerError> {
    let content = Content::new("user").with_text(prompt);
    let req = LlmRequest::new(model_name, vec![content]);

    let mut stream: Pin<
        Box<dyn Stream<Item = std::result::Result<LlmResponse, adk_core::AdkError>> + Send>,
    > = model
        .generate_content(req, false)
        .await
        .map_err(|e| PlannerError::LlmError(e.to_string()))?;

    let mut text = String::new();
    use futures::StreamExt;
    while let Some(result) = stream.next().await {
        let response = result.map_err(|e| PlannerError::LlmError(e.to_string()))?;
        if let Some(ref content) = response.content {
            for part in &content.parts {
                if let adk_core::Part::Text { text: t } = part {
                    text.push_str(t);
                }
            }
        }
    }

    if text.is_empty() {
        return Err(PlannerError::ParseError("empty response from LLM".into()));
    }
    Ok(text)
}

/// Extract JSON from text, handling markdown code blocks and raw JSON objects.
fn extract_json(text: &str) -> Option<String> {
    let trimmed = text.trim();

    // Try markdown code block with language tag.
    if let Some(start) = trimmed.find("```json") {
        let start = start + "```json".len();
        let remaining = &trimmed[start..];
        if let Some(end) = remaining.find("```") {
            let json = remaining[..end].trim();
            if !json.is_empty() {
                return Some(json.to_string());
            }
        }
    }

    // Try generic markdown code block.
    if let Some(start) = trimmed.find("```") {
        let start = start + 3;
        let remaining = &trimmed[start..];
        if let Some(end) = remaining.find("```") {
            let json = remaining[..end].trim();
            if !json.is_empty() {
                return Some(json.to_string());
            }
        }
    }

    // Try raw JSON object — find the outermost balanced braces.
    if let Some(start) = trimmed.find('{') {
        let mut depth = 0u32;
        let mut in_string = false;
        let mut escaped = false;
        for (i, ch) in trimmed[start..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(trimmed[start..=start + i].to_string());
                    }
                }
                _ => {}
            }
        }
    }

    None
}

/// Truncate text to a reasonable summary length (first line, max 120 chars).
fn truncate_summary(s: &str) -> String {
    let first_line = s.lines().next().unwrap_or("");
    if first_line.len() > 120 {
        let end = first_line
            .char_indices()
            .take_while(|(i, _)| *i < 117)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(117);
        format!("{}…", &first_line[..end])
    } else {
        first_line.to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::types::ModelEntry;
    use adk_model::MockLlm;

    // ── Helpers ──────────────────────────────────────────────────────

    fn mock_response(text: &str) -> LlmResponse {
        LlmResponse::new(Content::new("assistant").with_text(text))
    }

    fn test_planning_config() -> PlanningConfig {
        PlanningConfig {
            model: None,
            system_prompt: None,
            max_steps: 10,
            reasoning_depth: "standard".into(),
        }
    }

    fn test_validation_config() -> ValidationConfig {
        ValidationConfig { enabled: false }
    }

    fn test_app_config() -> AppConfig {
        AppConfig::default()
    }

    // ── extract_json ─────────────────────────────────────────────────

    #[test]
    fn test_extract_json_raw_object() {
        let input = r#"prefix {"steps":[{"step":1,"description":"test"}]} suffix"#;
        let result = extract_json(input);
        assert!(result.is_some(), "expected to extract JSON from raw text");
        let inner = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&inner).unwrap();
        assert!(parsed.get("steps").is_some());
    }

    #[test]
    fn test_extract_json_markdown_block() {
        let input =
            "Here's the plan:\n```json\n{\"steps\":[{\"step\":1,\"description\":\"test\"}]}\n```";
        let result = extract_json(input);
        assert!(result.is_some());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(parsed.get("steps").is_some());
    }

    #[test]
    fn test_extract_json_no_json() {
        assert!(extract_json("just some text").is_none());
    }

    #[test]
    fn test_extract_json_nested_braces() {
        let input = r#"{"outer":{"inner":"value"},"steps":[{"step":1,"tools":["bash"]}]}"#;
        let result = extract_json(input);
        assert!(result.is_some());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(parsed.get("steps").is_some());
    }

    // ── reasoning_instruction ────────────────────────────────────────

    #[test]
    fn test_reasoning_deep() {
        let instr = reasoning_instruction("deep");
        assert!(instr.contains("multiple approaches"));
        assert!(instr.contains("compare trade-offs"));
    }

    #[test]
    fn test_reasoning_quick() {
        let instr = reasoning_instruction("quick");
        assert!(instr.contains("direct"));
    }

    #[test]
    fn test_reasoning_standard() {
        let instr = reasoning_instruction("standard");
        assert!(instr.contains("Balance"));
    }

    #[test]
    fn test_reasoning_default() {
        let instr = reasoning_instruction("unknown");
        assert!(instr.contains("Balance"));
    }

    // ── truncate_summary ─────────────────────────────────────────────

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate_summary("Hello world"), "Hello world");
    }

    #[test]
    fn test_truncate_long() {
        let long = "x".repeat(200);
        let result = truncate_summary(&long);
        assert!(result.len() < 130);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_multiline() {
        let multi = "First line\nsecond line";
        assert_eq!(truncate_summary(multi), "First line");
    }

    // ── Validator ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_validator_disabled() {
        let config = ValidationConfig { enabled: false };
        let validator = Validator::from_config(&config);
        let result = validator.validate().await.unwrap();
        assert!(result.passed);
        assert!(result.errors.is_empty());
    }

    // ── Planner ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_planner_parses_valid_json_response() {
        let json = r#"{"steps":[{"step":1,"description":"Read main.rs","tools":["read"]},{"step":2,"description":"Fix the bug","tools":["edit"]}]}"#;
        let mock = MockLlm::new("test-planner").with_response(mock_response(json));
        let config = test_planning_config();
        let planner = Planner::new(Arc::new(mock), "test".into(), &config);

        let plan = planner.plan("fix the bug in main.rs").await.unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].description, "Read main.rs");
        assert_eq!(plan.steps[1].tools, vec!["edit"]);
    }

    #[tokio::test]
    async fn test_planner_parses_markdown_wrapped_json() {
        let md = "Here's the plan:\n```json\n{\"steps\":[{\"step\":1,\"description\":\"Do something\",\"tools\":[]}]}\n```";
        let mock = MockLlm::new("test-planner").with_response(mock_response(md));
        let config = test_planning_config();
        let planner = Planner::new(Arc::new(mock), "test".into(), &config);

        let plan = planner.plan("do something").await.unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].description, "Do something");
    }

    #[tokio::test]
    async fn test_planner_renumbers_steps() {
        let json = r#"{"steps":[{"step":99,"description":"First","tools":[]},{"step":100,"description":"Second","tools":[]}]}"#;
        let mock = MockLlm::new("test-planner").with_response(mock_response(json));
        let config = test_planning_config();
        let planner = Planner::new(Arc::new(mock), "test".into(), &config);

        let plan = planner.plan("test").await.unwrap();
        assert_eq!(plan.steps[0].step, 1);
        assert_eq!(plan.steps[1].step, 2);
    }

    #[tokio::test]
    async fn test_planner_empty_response_error() {
        let mock =
            MockLlm::new("test-planner").with_response(mock_response("I cannot help with that."));
        let config = test_planning_config();
        let planner = Planner::new(Arc::new(mock), "test".into(), &config);

        let result = planner.plan("test").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PlannerError::ParseError(_)));
    }

    // ── Executor ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_executor_returns_step_result() {
        let mock = MockLlm::new("test-executor").with_response(mock_response("Fixed the bug."));
        let executor = Executor::new(Arc::new(mock), "test".into(), None, 2);
        let step = PlanStep {
            step: 1,
            description: "Fix the bug".into(),
            tools: vec!["edit".into()],
        };

        let result = executor.execute_step(&step, "context").await.unwrap();
        assert_eq!(result.step, 1);
        assert!(result.success);
        assert_eq!(result.summary, "Fixed the bug.");
    }

    #[tokio::test]
    async fn test_executor_empty_response() {
        let mock = MockLlm::new("test-executor").with_response(mock_response(""));
        let executor = Executor::new(Arc::new(mock), "test".into(), None, 2);
        let step = PlanStep {
            step: 1,
            description: "test".into(),
            tools: vec![],
        };

        let result = executor.execute_step(&step, "context").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_executor_with_custom_system_prompt() {
        let mock = MockLlm::new("test-executor").with_response(mock_response("Done."));
        let executor = Executor::new(
            Arc::new(mock),
            "test".into(),
            Some("custom prompt".into()),
            0,
        );
        let step = PlanStep {
            step: 1,
            description: "test".into(),
            tools: vec![],
        };

        let result = executor.execute_step(&step, "context").await.unwrap();
        assert!(result.success);
    }

    // ── ExecutionPlan deserialization ─────────────────────────────────

    #[test]
    fn test_execution_plan_deserialize() {
        let json = r#"{"steps":[{"step":1,"description":"Read","tools":["read"]}]}"#;
        let plan: ExecutionPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].description, "Read");
    }

    #[test]
    fn test_execution_plan_empty_steps_default_tools() {
        let json = r#"{"steps":[{"step":1,"description":"Test"}]}"#;
        let plan: ExecutionPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert!(plan.steps[0].tools.is_empty());
    }

    // ── Fallback resolution ──────────────────────────────────────────

    #[test]
    fn test_resolve_fallback_no_entry() {
        let config = AppConfig::default();
        let (model, name) = resolve_fallback("nonexistent", &config);
        assert!(model.is_none());
        assert!(name.is_none());
    }

    #[test]
    fn test_resolve_fallback_no_fallback_field() {
        let mut config = AppConfig::default();
        config.model.models.insert(
            "gpt-4o".into(),
            ModelEntry {
                provider: crate::infra::config::types::ProviderKind::OpenAI,
                model: "gpt-4o".into(),
                fallback: None,
            },
        );
        let (model, name) = resolve_fallback("gpt-4o", &config);
        assert!(model.is_none());
        assert!(name.is_none());
    }

    // ── Orchestrator ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_orchestrator_plan_too_many_steps() {
        let json = r#"{"steps":[
            {"step":1,"description":"s1","tools":[]},
            {"step":2,"description":"s2","tools":[]},
            {"step":3,"description":"s3","tools":[]},
            {"step":4,"description":"s4","tools":[]},
            {"step":5,"description":"s5","tools":[]},
            {"step":6,"description":"s6","tools":[]},
            {"step":7,"description":"s7","tools":[]},
            {"step":8,"description":"s8","tools":[]},
            {"step":9,"description":"s9","tools":[]},
            {"step":10,"description":"s10","tools":[]},
            {"step":11,"description":"s11","tools":[]}
        ]}"#;

        let mock = MockLlm::new("test-orchestrator").with_response(mock_response(json));
        let config = PlanningConfig {
            model: None,
            system_prompt: None,
            max_steps: 5,
            reasoning_depth: "standard".into(),
        };
        let planner = Planner::new(Arc::new(mock), "test".into(), &config);
        let executor = Executor::new(Arc::new(MockLlm::new("exec")), "test".into(), None, 0);
        let validator = Validator::new(vec![]);

        let orchestrator =
            PlanningOrchestrator::new(planner, executor, validator, 5, None, None, None, None);

        let result = orchestrator.run("test").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PlannerError::ConfigError(_)));
    }

    #[tokio::test]
    async fn test_orchestrator_happy_path() {
        let plan_json = r#"{"steps":[{"step":1,"description":"Step one","tools":[]}]}"#;

        let plan_mock = MockLlm::new("planner").with_response(mock_response(plan_json));
        let exec_mock = MockLlm::new("executor").with_response(mock_response("Executed."));

        let config = PlanningConfig {
            model: None,
            system_prompt: None,
            max_steps: 10,
            reasoning_depth: "standard".into(),
        };
        let planner = Planner::new(Arc::new(plan_mock), "planner".into(), &config);
        let executor = Executor::new(Arc::new(exec_mock), "executor".into(), None, 0);
        let validator = Validator::new(vec![]);

        let orchestrator =
            PlanningOrchestrator::new(planner, executor, validator, 10, None, None, None, None);

        let results = orchestrator.run("test task").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].step, 1);
        assert!(results[0].success);
    }
}
