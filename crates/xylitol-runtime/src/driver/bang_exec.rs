use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::session::bash_execution_message_entry;
use xylitol_plugin_sdk::{BashExecOpts, XyBashExecutor, XyBashResult, XySessionStore};

struct CancelSlotGuard<'a>(&'a Mutex<Option<CancellationToken>>);

impl Drop for CancelSlotGuard<'_> {
    fn drop(&mut self) {
        *crate::utils::lock_mutex(self.0) = None;
    }
}

#[derive(Clone)]
pub struct BangExecHandler {
    executor: Option<Arc<dyn XyBashExecutor>>,
    cancel: Arc<Mutex<Option<CancellationToken>>>,
}

impl BangExecHandler {
    pub fn new(executor: Option<Arc<dyn XyBashExecutor>>) -> Self {
        Self {
            executor,
            cancel: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        store: &dyn XySessionStore,
        session_id: Option<&str>,
        bash_id: &str,
        command: &str,
        exclude_from_context: bool,
        cancel: CancellationToken,
        chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
        cwd: Option<String>,
    ) -> Result<XyBashResult, XyError> {
        let executor = self
            .executor
            .as_ref()
            .ok_or_else(|| XyError::Config("bash executor not configured".into()))?;

        {
            let mut slot = crate::utils::lock_mutex(&self.cancel);
            if slot.is_some() {
                return Err(XyError::Config("bash command already running".into()));
            }
            *slot = Some(cancel.clone());
        }
        let _slot_guard = CancelSlotGuard(&self.cancel);

        if let Some(sid) = session_id {
            record_bash_start(store, bash_id, command, exclude_from_context, sid).await?;
        }

        let result = executor
            .execute(
                command,
                BashExecOpts {
                    cancel: Some(cancel),
                    chunk_tx,
                    cwd: cwd.map(std::path::PathBuf::from),
                    ..Default::default()
                },
            )
            .await;

        if let Some(sid) = session_id {
            record_bash_result(store, bash_id, command, &result, exclude_from_context, sid).await?;
        }

        Ok(result)
    }

    pub fn abort(&self) {
        if let Some(cancel) = crate::utils::lock_mutex(&self.cancel).take() {
            cancel.cancel();
        }
    }
}

pub(crate) async fn record_bash_start(
    store: &dyn XySessionStore,
    bash_id: &str,
    command: &str,
    exclude_from_context: bool,
    session_id: &str,
) -> Result<(), XyError> {
    let entry = xylitol_plugin_sdk::session::bash_execution_message_entry(
        bash_id,
        command,
        String::new(),
        None,
        false,
        false,
        None,
        exclude_from_context,
        xylitol_plugin_sdk::message::BashExecutionStatus::Running,
    );
    store
        .append_session_entry(session_id, &entry)
        .await
        .map_err(XyError::from)
}

pub(crate) async fn record_bash_result(
    store: &dyn XySessionStore,
    bash_id: &str,
    command: &str,
    result: &XyBashResult,
    exclude_from_context: bool,
    session_id: &str,
) -> Result<(), XyError> {
    let entry = bash_execution_message_entry(
        bash_id,
        command,
        result.output.clone(),
        result.exit_code,
        result.cancelled,
        result.truncated,
        result.full_output_path.clone(),
        exclude_from_context,
        xylitol_plugin_sdk::message::BashExecutionStatus::Done,
    );
    store
        .append_session_entry(session_id, &entry)
        .await
        .map_err(XyError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct SlowBash;

    #[async_trait::async_trait]
    impl XyBashExecutor for SlowBash {
        async fn execute(&self, _command: &str, _opts: BashExecOpts) -> XyBashResult {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            XyBashResult {
                output: "ok".into(),
                exit_code: Some(0),
                cancelled: false,
                timed_out: false,
                truncated: false,
                full_output_path: None,
            }
        }
    }

    #[tokio::test]
    async fn concurrent_bang_execution_returns_busy() {
        let store: Arc<dyn XySessionStore> = Arc::new(crate::test_support::MemoryStore::new());
        let handler = BangExecHandler::new(Some(Arc::new(SlowBash)));
        let (first, second) = tokio::join!(
            handler.execute(
                store.as_ref(),
                None,
                "b1",
                "sleep 1",
                false,
                CancellationToken::new(),
                None,
                None,
            ),
            handler.execute(
                store.as_ref(),
                None,
                "b2",
                "sleep 1",
                false,
                CancellationToken::new(),
                None,
                None,
            ),
        );
        assert!(first.is_ok(), "first bang must run");
        match second {
            Err(XyError::Config(message)) => {
                assert!(message.contains("already running"), "{message}");
            }
            other => panic!("second concurrent bang must be busy: {other:?}"),
        }
    }

    #[tokio::test]
    async fn slot_is_released_after_completion() {
        let store: Arc<dyn XySessionStore> = Arc::new(crate::test_support::MemoryStore::new());
        let handler = BangExecHandler::new(Some(Arc::new(SlowBash)));
        handler
            .execute(
                store.as_ref(),
                None,
                "b1",
                "true",
                false,
                CancellationToken::new(),
                None,
                None,
            )
            .await
            .expect("first run");
        handler
            .execute(
                store.as_ref(),
                None,
                "b2",
                "true",
                false,
                CancellationToken::new(),
                None,
                None,
            )
            .await
            .expect("slot must be free again");
    }
}
