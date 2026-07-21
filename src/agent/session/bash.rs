//! Bash execution collaborator — runs user-initiated `!cmd` / `!!cmd`.
//!
//! [`BashExecHandler`] owns the optional [`XyBashExecutor`] port and the
//! in-flight cancellation token. The session store is borrowed per call so the
//! [`crate::agent::session::AgentCapabilities`] remains the single holder of session
//! context (design §4.1).

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::domain::session_types::{BashExecutionEntry, EntryBase, SessionEntry};
use crate::runtime_protocol::{BashExecOpts, XyBashExecutor, XyBashResult, XySessionStore};

/// Stateful bash-execution collaborator.
pub struct BashExecHandler {
    /// Injected bash executor port. `None` means `!cmd` is unavailable.
    executor: Option<Arc<dyn XyBashExecutor>>,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    /// `Arc` so [`crate::agent::AgentRuntime::abort`] can cancel without `&mut`
    /// (and tests can abort concurrent with [`Self::execute`]).
    cancel: Arc<Mutex<Option<CancellationToken>>>,
}

impl BashExecHandler {
    /// Construct with an optional bash executor port.
    pub fn new(executor: Option<Arc<dyn XyBashExecutor>>) -> Self {
        Self {
            executor,
            cancel: Arc::new(Mutex::new(None)),
        }
    }

    /// Shared cancel slot (test / future AbortToken clones).
    #[cfg(test)]
    pub(crate) fn cancel_slot(&self) -> Arc<Mutex<Option<CancellationToken>>> {
        Arc::clone(&self.cancel)
    }

    /// Execute a user-initiated bash command and record the result.
    ///
    /// `exclude_from_context=true` (the `!!` prefix) stores the entry on disk
    /// but omits it from LLM context (see `build_session_context`).
    ///
    /// `chunk_tx`: optional live output uplink for product TUI (c669).
    pub async fn execute(
        &self,
        store: &dyn XySessionStore,
        session_id: Option<&str>,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, String> {
        let executor = self
            .executor
            .as_ref()
            .ok_or("bash executor not configured")?;

        let cancel = CancellationToken::new();
        *self.cancel.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel.clone());

        let result = executor
            .execute(
                command,
                BashExecOpts {
                    cancel: Some(cancel),
                    chunk_tx,
                },
            )
            .await;

        *self.cancel.lock().unwrap_or_else(|e| e.into_inner()) = None;

        // Record on disk.
        if let Some(sid) = session_id {
            record_bash_result(store, command, &result, exclude_from_context, sid).await?;
        }

        Ok(result)
    }

    /// Abort any in-flight bash execution (`&self` for XyDriver / AgentRuntime abort).
    pub fn abort(&self) {
        if let Some(cancel) = self.cancel.lock().unwrap_or_else(|e| e.into_inner()).take() {
            cancel.cancel();
        }
    }
}

/// Persist a bash result as a `BashExecution` session entry.
pub(crate) async fn record_bash_result(
    store: &dyn XySessionStore,
    command: &str,
    result: &XyBashResult,
    exclude_from_context: bool,
    session_id: &str,
) -> Result<(), String> {
    let entry = SessionEntry::BashExecution(BashExecutionEntry {
        base: EntryBase {
            entry_type: "bash".into(),
            id: String::new(),
            parent_id: None,
            timestamp: String::new(),
        },
        command: command.to_string(),
        output: result.output.clone(),
        exit_code: result.exit_code,
        cancelled: result.cancelled,
        truncated: result.truncated,
        full_output_path: result.full_output_path.clone(),
        exclude_from_context,
    });
    store.append_session_entry(session_id, &entry).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::bash_exec::InfraBashExecutor;
    use std::time::Duration;

    #[tokio::test]
    async fn abort_cancels_in_flight_sleep() {
        let handler = BashExecHandler::new(Some(Arc::new(InfraBashExecutor::new())));
        let store = crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        );
        let store: Arc<dyn XySessionStore> = Arc::new(store);

        let handler_exec = BashExecHandler {
            executor: handler.executor.clone(),
            cancel: handler.cancel_slot(),
        };
        let store_exec = Arc::clone(&store);
        let join = tokio::spawn(async move {
            handler_exec
                .execute(store_exec.as_ref(), None, "sleep 30", false, None)
                .await
        });

        tokio::time::sleep(Duration::from_millis(150)).await;
        handler.abort();
        let result = join.await.expect("join").expect("execute");
        assert!(
            result.cancelled,
            "abort must cancel in-flight interactive bash"
        );
    }
}
