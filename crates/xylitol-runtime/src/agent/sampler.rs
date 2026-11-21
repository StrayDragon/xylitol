use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;
use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::message::XyLlmMessage;
use xylitol_plugin_sdk::model::XyToolSchema;
use xylitol_plugin_sdk::{XyGenerateOptions, XyModel, XyStream};

use super::runtime::retry::{RetryState, is_retryable_xy_error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Turn,
    StageName,
    CompactionSummary,
}

impl SampleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Turn => "turn",
            Self::StageName => "stage_name",
            Self::CompactionSummary => "compaction_summary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerState {
    InFlight,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerRecord {
    pub id: u64,
    pub kind: SampleKind,
    pub state: LedgerState,
}

struct Shared {
    ledger: Mutex<Vec<LedgerRecord>>,
    cancels: Mutex<HashMap<u64, CancellationToken>>,
    next_id: AtomicU64,
    config: Mutex<SamplerConfig>,
    semaphore: Mutex<Option<Arc<tokio::sync::Semaphore>>>,
}

impl Shared {
    fn lock_ledger(&self) -> std::sync::MutexGuard<'_, Vec<LedgerRecord>> {
        self.ledger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn transition(&self, id: u64, state: LedgerState) {
        let mut ledger = self.lock_ledger();
        if let Some(record) = ledger.iter_mut().find(|record| record.id == id) {
            record.state = state;
        }
    }

    fn forget_cancel(&self, id: u64) {
        self.cancels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&id);
    }
}

/// Per-session sampling ledger and parallel model-call coordinator.
///
/// Every model call registers here at submission order; completion, failure or
/// cancellation flips the ledger state. Parallel calls run as independent
/// tasks with child cancellation tokens; aggregation stays in submission
/// order regardless of completion order.
#[derive(Clone)]
pub struct SamplerActor {
    shared: Arc<Shared>,
}

impl Default for SamplerActor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SampleHandle {
    id: u64,
    rx: tokio::sync::oneshot::Receiver<Result<XyStream, XyError>>,
}

impl SampleHandle {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub async fn resolve(self) -> Result<XyStream, XyError> {
        match self.rx.await {
            Ok(result) => result,
            Err(_) => Err(XyError::Config(
                "sampler request task dropped before answering".into(),
            )),
        }
    }
}

impl SamplerActor {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                ledger: Mutex::new(Vec::new()),
                cancels: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
                config: Mutex::new(SamplerConfig::default()),
                semaphore: Mutex::new(None),
            }),
        }
    }

    /// Applies to new submissions only; in-flight requests keep the config
    /// they were submitted under (turn-start binding principle).
    pub fn update_config(&self, config: SamplerConfig) {
        *self
            .shared
            .config
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
    }

    /// Process-wide concurrency injection point; unlimited by default.
    pub fn set_semaphore(&self, semaphore: Arc<tokio::sync::Semaphore>) {
        *self
            .shared
            .semaphore
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(semaphore);
    }

    pub fn ledger(&self) -> Vec<LedgerRecord> {
        self.shared.lock_ledger().clone()
    }

    pub fn active_count(&self) -> usize {
        self.shared
            .lock_ledger()
            .iter()
            .filter(|record| record.state == LedgerState::InFlight)
            .count()
    }

    pub fn is_active(&self, id: u64) -> bool {
        self.shared
            .lock_ledger()
            .iter()
            .any(|record| record.id == id && record.state == LedgerState::InFlight)
    }

    pub fn cancel(&self, id: u64) -> bool {
        let cancel = self
            .shared
            .cancels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&id)
            .cloned();
        match cancel {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// Registers an externally driven model call in the same ledger.
    pub fn register_external(&self, kind: SampleKind) -> ExternalSample {
        let id = self.shared.next_id.fetch_add(1, Ordering::SeqCst);
        self.shared.lock_ledger().push(LedgerRecord {
            id,
            kind,
            state: LedgerState::InFlight,
        });
        ExternalSample {
            id,
            shared: Arc::clone(&self.shared),
        }
    }

    pub async fn submit_stream(
        &self,
        kind: SampleKind,
        model: Arc<dyn XyModel>,
        messages: Vec<XyLlmMessage>,
        tools: Vec<XyToolSchema>,
        options: XyGenerateOptions,
        parent: Option<&CancellationToken>,
    ) -> SampleHandle {
        let id = self.shared.next_id.fetch_add(1, Ordering::SeqCst);
        self.shared.lock_ledger().push(LedgerRecord {
            id,
            kind,
            state: LedgerState::InFlight,
        });
        let child = parent
            .map(CancellationToken::child_token)
            .unwrap_or_default();
        self.shared
            .cancels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, child.clone());
        let (tx, rx) = tokio::sync::oneshot::channel();
        let shared = Arc::clone(&self.shared);
        let config = *shared
            .config
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let semaphore = shared
            .semaphore
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        tokio::spawn(async move {
            let result =
                run_sampled_request(model, messages, tools, options, &child, config, semaphore)
                    .await;
            let state = match &result {
                Ok(_) => LedgerState::Completed,
                Err(error) if error.to_string().contains("cancelled") => LedgerState::Cancelled,
                Err(_) => LedgerState::Failed,
            };
            shared.transition(id, state);
            shared.forget_cancel(id);
            let _ = tx.send(result);
        });
        SampleHandle { id, rx }
    }
}

async fn run_sampled_request(
    model: Arc<dyn XyModel>,
    messages: Vec<XyLlmMessage>,
    tools: Vec<XyToolSchema>,
    options: XyGenerateOptions,
    cancel: &CancellationToken,
    config: SamplerConfig,
    semaphore: Option<Arc<tokio::sync::Semaphore>>,
) -> Result<XyStream, XyError> {
    let _permit = match semaphore {
        Some(semaphore) => Some(tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return Err(XyError::Config("sampler request cancelled".into()));
            }
            permit = semaphore.acquire_owned() => permit.map_err(|_| {
                XyError::Config("sampler semaphore closed".into())
            })?,
        }),
        None => None,
    };
    let retry_state = RetryState::new(config.max_retries, config.base_delay_ms);
    loop {
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return Err(XyError::Config("sampler request cancelled".into()));
            }
            result = model.generate_stream(
                messages.clone(),
                &tools,
                true,
                options.clone(),
            ) => result,
        };
        match result {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                if is_retryable_xy_error(&error) && retry_state.can_retry() {
                    let delay = retry_state
                        .next_delay_with_hint(error.provider_detail().and_then(|d| d.retry_after));
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            return Err(XyError::Config("sampler request cancelled".into()));
                        }
                        _ = retry_state.backoff(delay) => continue,
                    }
                }
                return Err(error);
            }
        }
    }
}

/// Aggregates parallel submissions in submission order.
pub async fn aggregate(handles: Vec<SampleHandle>) -> Vec<Result<XyStream, XyError>> {
    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.resolve().await);
    }
    results
}

pub struct ExternalSample {
    id: u64,
    shared: Arc<Shared>,
}

impl ExternalSample {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn complete(self) {
        self.shared.transition(self.id, LedgerState::Completed);
    }

    pub fn fail(self) {
        self.shared.transition(self.id, LedgerState::Failed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use xylitol_plugin_sdk::model::XyChunk;

    struct GateModel {
        name: String,
        hold: tokio::sync::Mutex<()>,
        release: tokio::sync::Notify,
        text: &'static str,
    }

    impl GateModel {
        fn new(name: &str, text: &'static str) -> Arc<Self> {
            Arc::new(Self {
                name: name.to_string(),
                hold: tokio::sync::Mutex::new(()),
                release: tokio::sync::Notify::new(),
                text,
            })
        }
    }

    #[async_trait::async_trait]
    impl XyModel for GateModel {
        fn name(&self) -> &str {
            &self.name
        }

        async fn generate_stream(
            &self,
            _messages: Vec<XyLlmMessage>,
            _tools: &[XyToolSchema],
            _stream: bool,
            _options: XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let _guard = self.hold.lock().await;
            self.release.notified().await;
            let text = self.text;
            Ok(Box::pin(futures::stream::iter(vec![
                Ok(XyChunk::TextDelta(text.to_string())),
                Ok(XyChunk::Done {
                    usage: None,
                    finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
                }),
            ])))
        }
    }

    fn plain_model(text: &'static str) -> Arc<dyn XyModel> {
        struct Plain(&'static str);
        #[async_trait::async_trait]
        impl XyModel for Plain {
            fn name(&self) -> &str {
                "plain"
            }
            async fn generate_stream(
                &self,
                _messages: Vec<XyLlmMessage>,
                _tools: &[XyToolSchema],
                _stream: bool,
                _options: XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                Ok(Box::pin(futures::stream::iter(vec![
                    Ok(XyChunk::TextDelta(self.0.to_string())),
                    Ok(XyChunk::Done {
                        usage: None,
                        finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
                    }),
                ])))
            }
        }
        Arc::new(Plain(text))
    }

    fn options() -> XyGenerateOptions {
        XyGenerateOptions::default()
    }

    #[tokio::test]
    async fn parallel_submissions_aggregate_in_submission_order() {
        let sampler = SamplerActor::new();
        let first = GateModel::new("slow", "slow-answer");
        let second = GateModel::new("fast", "fast-answer");
        let handle_a = sampler
            .submit_stream(
                SampleKind::Turn,
                first.clone(),
                Vec::new(),
                Vec::new(),
                options(),
                None,
            )
            .await;
        let handle_b = sampler
            .submit_stream(
                SampleKind::Turn,
                second.clone(),
                Vec::new(),
                Vec::new(),
                options(),
                None,
            )
            .await;
        assert_eq!(sampler.active_count(), 2);
        // Release the second (later-submitted) request first and let it
        // finish; completion order must not reorder aggregation or the ledger.
        second.release.notify_one();
        tokio::task::yield_now().await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(
            sampler.is_active(handle_a.id()),
            "the first request must still be in flight"
        );
        first.release.notify_one();
        let results = aggregate(vec![handle_a, handle_b]).await;
        let mut first_text = String::new();
        if let Ok(stream) = results.into_iter().next().unwrap() {
            first_text = collect_text(stream).await;
        }
        assert!(
            first_text.contains("slow-answer"),
            "aggregation must stay in submission order, got {first_text:?}"
        );
        let ledger = sampler.ledger();
        assert_eq!(
            ledger.iter().map(|r| r.kind.as_str()).collect::<Vec<_>>(),
            vec!["turn", "turn"]
        );
        assert!(
            ledger.iter().all(|r| r.state == LedgerState::Completed),
            "{ledger:?}"
        );
        assert_eq!(sampler.active_count(), 0);
    }

    async fn collect_text(mut stream: XyStream) -> String {
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let Ok(XyChunk::TextDelta(delta)) = chunk {
                text.push_str(&delta);
            }
        }
        text
    }

    #[tokio::test]
    async fn cancel_flips_ledger_to_cancelled() {
        let sampler = SamplerActor::new();
        let model = GateModel::new("held", "never");
        let handle = sampler
            .submit_stream(
                SampleKind::StageName,
                model,
                Vec::new(),
                Vec::new(),
                options(),
                None,
            )
            .await;
        assert!(sampler.cancel(handle.id()));
        let result = handle.resolve().await;
        assert!(
            result.is_err(),
            "a cancelled request must resolve to an error"
        );
        let ledger = sampler.ledger();
        assert_eq!(ledger[0].kind, SampleKind::StageName);
        assert_eq!(ledger[0].state, LedgerState::Cancelled);
        assert_eq!(sampler.active_count(), 0);
    }

    #[tokio::test]
    async fn external_registration_shares_the_ledger() {
        let sampler = SamplerActor::new();
        let bypass = sampler.register_external(SampleKind::CompactionSummary);
        assert!(sampler.is_active(bypass.id()));
        let handle = sampler
            .submit_stream(
                SampleKind::Turn,
                plain_model("turn-answer"),
                Vec::new(),
                Vec::new(),
                options(),
                None,
            )
            .await;
        bypass.complete();
        let _ = handle.resolve().await;
        let ledger = sampler.ledger();
        assert_eq!(
            ledger
                .iter()
                .map(|r| (r.kind.as_str(), r.state))
                .collect::<Vec<_>>(),
            vec![
                ("compaction_summary", LedgerState::Completed),
                ("turn", LedgerState::Completed),
            ]
        );
    }

    #[tokio::test]
    async fn update_config_applies_to_new_submissions_only() {
        let sampler = SamplerActor::new();
        sampler.update_config(SamplerConfig {
            max_retries: 0,
            base_delay_ms: 1,
        });
        let handle = sampler
            .submit_stream(
                SampleKind::Turn,
                plain_model("ok"),
                Vec::new(),
                Vec::new(),
                options(),
                None,
            )
            .await;
        assert!(handle.resolve().await.is_ok());
        assert_eq!(sampler.ledger()[0].state, LedgerState::Completed);
    }

    #[test]
    fn config_defaults_match_react_ladder() {
        let config = SamplerConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 1000);
        let _ = json!(config.max_retries);
    }
}
