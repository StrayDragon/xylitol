use std::sync::{Arc, Mutex};

use chrono::Local;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage};
use xylitol_plugin_sdk::model::XyChunk;
use xylitol_plugin_sdk::session::{
    AutomationFact, GoalProjection, GoalState, GoalVerdict, goal_transition, project_goals,
};
use xylitol_plugin_sdk::{XyGenerateOptions, XyModel, XySessionStore};

use crate::agent::model::manager::ModelManager;

pub const ACCEPTANCE_MAX_CHARS: usize = 4000;
const TRANSCRIPT_MAX_CHARS: usize = 12000;
const INVALID_BLOCK_THRESHOLD: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalDecision {
    Continue { guidance: String },
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalEvalVerdict {
    Met,
    NotMet,
    Impossible,
    Invalid,
}

impl GoalEvalVerdict {
    pub fn as_goal_verdict(self) -> GoalVerdict {
        match self {
            GoalEvalVerdict::Met => GoalVerdict::Met,
            GoalEvalVerdict::NotMet => GoalVerdict::NotMet,
            GoalEvalVerdict::Impossible => GoalVerdict::Impossible,
            GoalEvalVerdict::Invalid => GoalVerdict::Invalid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalEvalOutcome {
    pub verdict: GoalEvalVerdict,
    pub evidence: String,
    pub raw: String,
}

pub struct GoalEvaluator;

impl GoalEvaluator {
    pub async fn evaluate(
        model: Arc<dyn XyModel>,
        acceptance: &str,
        transcript: &str,
        prior_evidence: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<GoalEvalOutcome, XyError> {
        if cancel.is_cancelled() {
            return Err(XyError::Aborted);
        }
        let prompt = build_prompt(acceptance, transcript, prior_evidence);
        let messages = vec![XyLlmMessage::user(prompt)];
        let mut stream = model
            .generate_stream(messages, &[], false, XyGenerateOptions::default())
            .await?;
        let mut text = String::new();
        while let Some(item) = stream.next().await {
            if cancel.is_cancelled() {
                return Err(XyError::Aborted);
            }
            match item? {
                XyChunk::TextDelta(delta) => text.push_str(&delta),
                XyChunk::Done { .. } => break,
                _ => {}
            }
        }
        Ok(parse_outcome(&text))
    }
}

pub async fn decide_active_goal(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    history: &[AgentMessage],
    model_manager: &Arc<Mutex<ModelManager>>,
    cancel: &CancellationToken,
) -> Result<Option<GoalDecision>, XyError> {
    if cancel.is_cancelled() {
        return Ok(None);
    }
    let entries = store
        .load_leaf_branch(session_id)
        .await
        .map_err(|error| XyError::Session(error.into()))?;
    let Some(goal) = project_goals(&entries, session_id)
        .into_iter()
        .find(|goal| matches!(goal.state, GoalState::Open | GoalState::Running))
    else {
        return Ok(None);
    };
    let mut current_state = goal.state;
    if current_state == GoalState::Open {
        let next = goal_transition(current_state, GoalState::Running)
            .map_err(|error| XyError::Config(format!("goal transition rejected: {error}")))?;
        append(
            store,
            session_id,
            &AutomationFact::GoalStateChanged {
                goal_id: goal.goal_id.clone(),
                state: next,
                reason: Some("first_turn".into()),
                actor: "evaluator".into(),
                at: Local::now().timestamp(),
            },
        )
        .await?;
        current_state = next;
    }

    let transcript = transcript_text(history);
    let model = {
        let manager = crate::utils::lock_mutex(model_manager);
        manager.build_current_model()
    };
    let outcome = match model {
        Ok(model) => {
            GoalEvaluator::evaluate(
                model,
                &goal.acceptance,
                &transcript,
                goal.last_evidence.as_deref(),
                cancel,
            )
            .await
        }
        Err(error) => Err(error),
    };
    let now = Local::now().timestamp();
    let next_iterations = goal.iterations_used + 1;
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => GoalEvalOutcome {
            verdict: GoalEvalVerdict::Invalid,
            evidence: format!("evaluator error: {error}"),
            raw: String::new(),
        },
    };
    append(
        store,
        session_id,
        &AutomationFact::GoalEvaluated {
            goal_id: goal.goal_id.clone(),
            verdict: outcome.verdict.as_goal_verdict(),
            evidence: outcome.evidence.clone(),
            iterations_used: next_iterations,
            at: now,
        },
    )
    .await?;

    match outcome.verdict {
        GoalEvalVerdict::Met => {
            set_goal_state(
                store,
                session_id,
                &goal,
                current_state,
                GoalState::Done,
                "met",
            )
            .await?;
            Ok(Some(GoalDecision::Stop))
        }
        GoalEvalVerdict::Impossible => {
            set_goal_state(
                store,
                session_id,
                &goal,
                current_state,
                GoalState::Failed,
                "impossible",
            )
            .await?;
            Ok(Some(GoalDecision::Stop))
        }
        GoalEvalVerdict::NotMet if next_iterations >= goal.max_iterations => {
            set_goal_state(
                store,
                session_id,
                &goal,
                current_state,
                GoalState::Blocked,
                "max_iterations",
            )
            .await?;
            Ok(Some(GoalDecision::Stop))
        }
        GoalEvalVerdict::NotMet => Ok(Some(GoalDecision::Continue {
            guidance: outcome.evidence.clone(),
        })),
        GoalEvalVerdict::Invalid => {
            if goal.consecutive_invalid + 1 >= INVALID_BLOCK_THRESHOLD {
                set_goal_state(
                    store,
                    session_id,
                    &goal,
                    current_state,
                    GoalState::Blocked,
                    "invalid",
                )
                .await?;
                Ok(Some(GoalDecision::Stop))
            } else {
                Ok(Some(GoalDecision::Continue {
                    guidance: outcome.evidence.clone(),
                }))
            }
        }
    }
}

fn transcript_text(history: &[AgentMessage]) -> String {
    let mut out = String::new();
    for message in history.iter().rev().take(24).rev() {
        out.push_str(message.role_name());
        out.push_str(": ");
        out.push_str(&message.text());
        out.push('\n');
    }
    out
}

async fn set_goal_state(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    goal: &GoalProjection,
    from: GoalState,
    to: GoalState,
    reason: &str,
) -> Result<(), XyError> {
    let next = goal_transition(from, to)
        .map_err(|error| XyError::Config(format!("goal transition rejected: {error}")))?;
    append(
        store,
        session_id,
        &AutomationFact::GoalStateChanged {
            goal_id: goal.goal_id.clone(),
            state: next,
            reason: Some(reason.into()),
            actor: "evaluator".into(),
            at: Local::now().timestamp(),
        },
    )
    .await
}

async fn append(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    fact: &AutomationFact,
) -> Result<(), XyError> {
    store
        .append_session_entry(session_id, &fact.to_custom_entry())
        .await
        .map_err(|error| XyError::Session(error.into()))
}

fn build_prompt(acceptance: &str, transcript: &str, prior_evidence: Option<&str>) -> String {
    let acceptance: String = acceptance.chars().take(ACCEPTANCE_MAX_CHARS).collect();
    let transcript: String = transcript.chars().take(TRANSCRIPT_MAX_CHARS).collect();
    let prior = prior_evidence.unwrap_or("");
    format!(
        "You are the goal evaluator. Judge whether the goal is achieved.\n\
         Return only JSON: {{\"verdict\":\"met|not_met|impossible|invalid\",\"evidence\":\"...\"}}.\n\
         met requires concrete evidence from the transcript. If evidence is missing, use not_met.\n\
         Acceptance:\n{acceptance}\n\
         Prior evidence:\n{prior}\n\
         Transcript:\n{transcript}"
    )
}

fn parse_outcome(raw: &str) -> GoalEvalOutcome {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let verdict = value
            .get("verdict")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let evidence = value
            .get("evidence")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(verdict) = verdict {
            let parsed = match verdict.as_str() {
                "met" if !evidence.trim().is_empty() => GoalEvalVerdict::Met,
                "met" => GoalEvalVerdict::NotMet,
                "not_met" => GoalEvalVerdict::NotMet,
                "impossible" => GoalEvalVerdict::Impossible,
                _ => GoalEvalVerdict::Invalid,
            };
            return GoalEvalOutcome {
                verdict: parsed,
                evidence,
                raw: raw.to_string(),
            };
        }
    }
    GoalEvalOutcome {
        verdict: GoalEvalVerdict::Invalid,
        evidence: "evaluator returned non-JSON or missing verdict".into(),
        raw: raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use std::pin::Pin;
    use xylitol_plugin_sdk::XyStream;
    use xylitol_plugin_sdk::message::XyStopReason;

    struct FakeModel {
        text: String,
    }

    #[async_trait::async_trait]
    impl XyModel for FakeModel {
        fn name(&self) -> &str {
            "fake-goal"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let text = self.text.clone();
            let chunks: Vec<Result<XyChunk, XyError>> = vec![
                Ok(XyChunk::TextDelta(text)),
                Ok(XyChunk::Done {
                    finish_reason: XyStopReason::Stop,
                    usage: None,
                }),
            ];
            Ok(Box::pin(stream::iter(chunks)) as Pin<Box<_>>)
        }
    }

    async fn evaluate_text(text: &str) -> GoalEvalOutcome {
        GoalEvaluator::evaluate(
            Arc::new(FakeModel { text: text.into() }),
            "tests pass",
            "assistant: cargo test exits 0",
            None,
            &CancellationToken::new(),
        )
        .await
        .expect("evaluate")
    }

    #[tokio::test]
    async fn parses_met_with_evidence() {
        let out = evaluate_text(r#"{"verdict":"met","evidence":"cargo test exits 0"}"#).await;
        assert_eq!(out.verdict, GoalEvalVerdict::Met);
    }

    #[tokio::test]
    async fn met_without_evidence_is_not_met() {
        let out = evaluate_text(r#"{"verdict":"met","evidence":""}"#).await;
        assert_eq!(out.verdict, GoalEvalVerdict::NotMet);
    }

    #[tokio::test]
    async fn parses_not_met_and_impossible() {
        assert_eq!(
            evaluate_text(r#"{"verdict":"not_met","evidence":"still failing"}"#)
                .await
                .verdict,
            GoalEvalVerdict::NotMet
        );
        assert_eq!(
            evaluate_text(r#"{"verdict":"impossible","evidence":"missing tool"}"#)
                .await
                .verdict,
            GoalEvalVerdict::Impossible
        );
    }

    #[tokio::test]
    async fn invalid_output_is_invalid() {
        let out = evaluate_text("not json").await;
        assert_eq!(out.verdict, GoalEvalVerdict::Invalid);
    }

    use crate::agent::model::manager::ModelManager;
    use crate::agent::model::registry::ModelRegistry;
    use serde_json::json;
    use std::sync::Mutex as StdMutex;
    use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
    use xylitol_plugin_sdk::session::{
        CUSTOM_TYPE_AUTOMATION, CustomEntry, EntryBase, ForkPosition, GoalState, SessionContext,
        SessionEntry, project_goals,
    };
    use xylitol_plugin_sdk::{
        XyModelBuilder, XyModelConfig, XyModelKind, XyModelMeta, XySessionStore,
    };

    struct MemStore {
        entries: StdMutex<Vec<SessionEntry>>,
    }

    impl MemStore {
        fn new() -> Self {
            Self {
                entries: StdMutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _session_id: &str) -> bool {
            true
        }

        async fn load_entries(
            &self,
            _session_id: &str,
        ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().map(|e| e.clone()).unwrap_or_default())
        }

        async fn append_session_entry(
            &self,
            _session_id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries
                .lock()
                .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
                .push(entry.clone());
            Ok(())
        }

        async fn build_session_context(
            &self,
            _session_id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Ok(SessionContext {
                messages: Vec::new(),
                thinking_level: String::new(),
                model: None,
            })
        }

        async fn create(
            &self,
            _id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }

        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            _at_entry_id: &str,
            _position: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::busy("fork unsupported"))
        }

        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}

        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    fn goal_entry(max_iterations: u64) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "goal_defined",
                "goal_id": "g1",
                "branch_id": "s1",
                "objective": "tests pass",
                "acceptance": "cargo test exits 0",
                "max_iterations": max_iterations,
                "created_at": 0
            }),
        })
    }

    fn manager(text: &str) -> Arc<Mutex<ModelManager>> {
        let mut registry = ModelRegistry::new();
        registry.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
                compat: None,
                prompt_cache: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        });
        let text = text.to_string();
        let builder: XyModelBuilder =
            Arc::new(move |_| Arc::new(FakeModel { text: text.clone() }) as Arc<dyn XyModel>);
        let mut manager = ModelManager::new(registry, builder);
        manager.select_model("mock").expect("select mock");
        Arc::new(Mutex::new(manager))
    }

    async fn run_decide(
        store: Arc<dyn XySessionStore>,
        manager: Arc<Mutex<ModelManager>>,
    ) -> Option<GoalDecision> {
        decide_active_goal(&store, "s1", &[], &manager, &CancellationToken::new())
            .await
            .expect("decide")
    }

    #[tokio::test]
    async fn met_marks_done_and_stops() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        store
            .append_session_entry("s1", &goal_entry(3))
            .await
            .unwrap();
        let control = run_decide(
            store.clone(),
            manager(r#"{"verdict":"met","evidence":"cargo test exits 0"}"#),
        )
        .await;
        assert_eq!(control, Some(GoalDecision::Stop));
        let goals = project_goals(&store.load_entries("s1").await.unwrap(), "s1");
        assert_eq!(goals[0].state, GoalState::Done);
    }

    #[tokio::test]
    async fn not_met_at_max_blocks_and_stops() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        store
            .append_session_entry("s1", &goal_entry(1))
            .await
            .unwrap();
        let control = run_decide(
            store.clone(),
            manager(r#"{"verdict":"not_met","evidence":"still failing"}"#),
        )
        .await;
        assert_eq!(control, Some(GoalDecision::Stop));
        let goals = project_goals(&store.load_entries("s1").await.unwrap(), "s1");
        assert_eq!(goals[0].state, GoalState::Blocked);
    }

    #[tokio::test]
    async fn impossible_marks_failed_and_stops() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        store
            .append_session_entry("s1", &goal_entry(3))
            .await
            .unwrap();
        let control = run_decide(
            store.clone(),
            manager(r#"{"verdict":"impossible","evidence":"missing tool"}"#),
        )
        .await;
        assert_eq!(control, Some(GoalDecision::Stop));
        let goals = project_goals(&store.load_entries("s1").await.unwrap(), "s1");
        assert_eq!(goals[0].state, GoalState::Failed);
    }

    #[tokio::test]
    async fn paused_goal_is_not_evaluated() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        store
            .append_session_entry("s1", &goal_entry(3))
            .await
            .unwrap();
        store
            .append_session_entry(
                "s1",
                &SessionEntry::Custom(CustomEntry {
                    base: EntryBase {
                        entry_type: "custom".into(),
                        id: String::new(),
                        parent_id: None,
                        timestamp: 0,
                    },
                    custom_type: CUSTOM_TYPE_AUTOMATION.into(),
                    data: json!({
                        "kind": "goal_state_changed",
                        "goal_id": "g1",
                        "state": "paused",
                        "actor": "test",
                        "at": 0
                    }),
                }),
            )
            .await
            .unwrap();
        assert_eq!(run_decide(store, manager("{}")).await, None);
    }

    #[tokio::test]
    async fn three_invalid_results_block_goal() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        store
            .append_session_entry("s1", &goal_entry(10))
            .await
            .unwrap();
        let manager = manager("not json");
        let first = run_decide(store.clone(), manager.clone()).await;
        assert!(
            matches!(first, Some(GoalDecision::Continue { ref guidance }) if !guidance.is_empty()),
            "invalid verdict must continue with guidance: {first:?}"
        );
        let second = run_decide(store.clone(), manager.clone()).await;
        assert!(
            matches!(second, Some(GoalDecision::Continue { .. })),
            "second invalid verdict must still continue: {second:?}"
        );
        assert_eq!(
            run_decide(store.clone(), manager.clone()).await,
            Some(GoalDecision::Stop)
        );
        let goals = project_goals(&store.load_entries("s1").await.unwrap(), "s1");
        assert_eq!(goals[0].state, GoalState::Blocked);
        assert_eq!(goals[0].consecutive_invalid, 3);
    }
}
