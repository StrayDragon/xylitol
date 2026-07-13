//! Bash execution collaborator — runs user-initiated `!cmd` / `!!cmd`.
//!
//! [`BashExecHandler`] owns the optional [`XyBashExecutor`] port and the
//! in-flight cancellation token. The session store is borrowed per call so the
//! [`crate::agent::session::AgentCapabilities`] remains the single holder of session
//! context (design §4.1).

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::domain::session_types::{BashExecutionEntry, EntryBase, SessionEntry};
use crate::runtime_protocol::{XyBashExecutor, XyBashResult, XySessionStore};

/// Stateful bash-execution collaborator.
pub struct BashExecHandler {
    /// Injected bash executor port. `None` means `!cmd` is unavailable.
    executor: Option<Arc<dyn XyBashExecutor>>,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    cancel: Option<CancellationToken>,
}

impl BashExecHandler {
    /// Construct with an optional bash executor port.
    pub fn new(executor: Option<Arc<dyn XyBashExecutor>>) -> Self {
        Self {
            executor,
            cancel: None,
        }
    }

    /// Execute a user-initiated bash command and record the result.
    ///
    /// `exclude_from_context=true` (the `!!` prefix) stores the entry on disk
    /// but omits it from LLM context (see `build_session_context`).
    pub async fn execute(
        &mut self,
        store: &dyn XySessionStore,
        session_id: Option<&str>,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<XyBashResult, String> {
        let executor = self
            .executor
            .as_ref()
            .ok_or("bash executor not configured")?;

        let cancel = CancellationToken::new();
        self.cancel = Some(cancel.clone());

        let result = executor.execute(command, Some(cancel)).await;

        self.cancel = None;

        // Record on disk.
        if let Some(sid) = session_id {
            record_bash_result(store, command, &result, exclude_from_context, sid).await?;
        }

        Ok(result)
    }

    /// Abort any in-flight bash execution.
    pub fn abort(&mut self) {
        if let Some(cancel) = self.cancel.take() {
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
