//! Bang execution (`!cmd` / `!!cmd`) — app-surface collaborator.
//!
//! [`BangExecHandler`] owns the optional [`XyBashExecutor`] port and the
//! in-flight cancellation token. Held by [`XyInProcessDriver`](crate::driver::XyInProcessDriver);
//! session store is borrowed per call.

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::session::bash_execution_message_entry;
use xylitol_plugin_sdk::{BashExecOpts, XyBashExecutor, XyBashResult, XySessionStore};

/// Stateful bash-execution collaborator.
#[derive(Clone)]
pub struct BangExecHandler {
    /// Injected bash executor port. `None` means `!cmd` is unavailable.
    executor: Option<Arc<dyn XyBashExecutor>>,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    /// `Arc` so [`XyDriver::abort`](crate::wiring::driver::XyDriver::abort) can cancel
    /// without `&mut` (and tests can abort concurrent with [`Self::execute`]).
    cancel: Arc<Mutex<Option<CancellationToken>>>,
}

impl BangExecHandler {
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

    /// Execute a user-initiated bash command and record its lifecycle.
    ///
    /// `exclude_from_context=true` (the `!!` prefix) stores the entry on disk
    /// but omits it from LLM context (see `build_session_context`).
    ///
    /// `bash_id`: correlation id for the start/done rows and output chunks.
    ///
    /// `cancel`: run kill switch. Callers that inject a [`xylitol_plugin_sdk::BashOutputSink`]
    /// pass its token so host `abort` can reach the run out-of-band while the
    /// writer lock is held by this executing call; other callers pass a fresh
    /// token and cancel via [`Self::abort`].
    ///
    /// `chunk_tx`: optional live output uplink; the caller wires it to the
    /// session output-event sink (c2760) or consumes it locally.
    ///
    /// `cwd`: session workspace for the spawned shell (`None` inherits the
    /// process cwd). Attach writers pass the TUI-provided workspace so `!cmd`
    /// runs there, not in the server process directory.
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

        *crate::utils::lock_mutex(&self.cancel) = Some(cancel.clone());

        // c2760: start marker first, so a crash leaves a running row that
        // resumes as `interrupted`.
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

        *crate::utils::lock_mutex(&self.cancel) = None;

        // Record on disk.
        if let Some(sid) = session_id {
            record_bash_result(store, bash_id, command, &result, exclude_from_context, sid).await?;
        }

        Ok(result)
    }

    /// Abort any in-flight bash execution (`&self` for [`XyDriver::abort`](crate::wiring::driver::XyDriver::abort)).
    pub fn abort(&self) {
        if let Some(cancel) = crate::utils::lock_mutex(&self.cancel).take() {
            cancel.cancel();
        }
    }
}

/// Persist the interactive bang start marker (`status = Running`, c2760).
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

/// Persist a bash result as `SessionEntry::Message` with `role=bashExecution` (c1210).
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
