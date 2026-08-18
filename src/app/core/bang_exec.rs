//! Bang execution (`!cmd` / `!!cmd`) — app-surface collaborator.
//!
//! [`BangExecHandler`] owns the optional [`XyBashExecutor`] port and the
//! in-flight cancellation token. Held by [`XyInProcessDriver`](crate::app::core::driver::XyInProcessDriver);
//! session store is borrowed per call.

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::protocol::error::XyError;
use crate::protocol::ports::{BashExecOpts, XyBashExecutor, XyBashResult, XySessionStore};
use crate::protocol::session::bash_execution_message_entry;

/// Stateful bash-execution collaborator.
pub struct BangExecHandler {
    /// Injected bash executor port. `None` means `!cmd` is unavailable.
    executor: Option<Arc<dyn XyBashExecutor>>,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    /// `Arc` so [`XyDriver::abort`](crate::app::core::driver::XyDriver::abort) can cancel
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
    ) -> Result<XyBashResult, XyError> {
        let executor = self
            .executor
            .as_ref()
            .ok_or_else(|| XyError::Config("bash executor not configured".into()))?;

        let cancel = CancellationToken::new();
        *crate::utils::lock_mutex(&self.cancel) = Some(cancel.clone());

        let result = executor
            .execute(
                command,
                BashExecOpts {
                    cancel: Some(cancel),
                    chunk_tx,
                    ..Default::default()
                },
            )
            .await;

        *crate::utils::lock_mutex(&self.cancel) = None;

        // Record on disk.
        if let Some(sid) = session_id {
            record_bash_result(store, command, &result, exclude_from_context, sid).await?;
        }

        Ok(result)
    }

    /// Abort any in-flight bash execution (`&self` for [`XyDriver::abort`](crate::app::core::driver::XyDriver::abort)).
    pub fn abort(&self) {
        if let Some(cancel) = crate::utils::lock_mutex(&self.cancel).take() {
            cancel.cancel();
        }
    }
}

/// Persist a bash result as `SessionEntry::Message` with `role=bashExecution` (c1210).
pub(crate) async fn record_bash_result(
    store: &dyn XySessionStore,
    command: &str,
    result: &XyBashResult,
    exclude_from_context: bool,
    session_id: &str,
) -> Result<(), XyError> {
    let entry = bash_execution_message_entry(
        command,
        result.output.clone(),
        result.exit_code,
        result.cancelled,
        result.truncated,
        result.full_output_path.clone(),
        exclude_from_context,
    );
    store
        .append_session_entry(session_id, &entry)
        .await
        .map_err(XyError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::bash_exec::InfraBashExecutor;
    use crate::protocol::message::{AgentMessage, EnvMessage};
    use crate::protocol::session::{SessionEntry, message_role};
    use std::time::Duration;

    #[tokio::test]
    async fn abort_cancels_in_flight_sleep() {
        let handler = BangExecHandler::new(Some(Arc::new(InfraBashExecutor::new())));
        let store = crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        );
        let store: Arc<dyn XySessionStore> = Arc::new(store);

        let handler_exec = BangExecHandler {
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

    #[tokio::test]
    async fn record_bash_writes_nested_message_not_top_level() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::infra::session::SessionManager::new(dir.path().join("sessions"));
        let sid = "s-bash-nested";
        store.create(sid, Some("/tmp"), None).await.expect("create");
        let result = XyBashResult {
            output: "hello\n".into(),
            exit_code: Some(0),
            cancelled: false,
            timed_out: false,
            truncated: false,
            full_output_path: None,
        };
        record_bash_result(&store, "echo hello", &result, false, sid)
            .await
            .expect("record");
        let entries = store.load(sid).await.expect("load");
        let bash = entries
            .iter()
            .find(|e| {
                matches!(e, SessionEntry::Message(m) if message_role(&m.message) == Some("bashExecution"))
            })
            .expect("bash message entry");
        match bash {
            SessionEntry::Message(m) => {
                assert_eq!(message_role(&m.message), Some("bashExecution"));
                let msg: AgentMessage = serde_json::from_value(m.message.clone()).unwrap();
                assert!(matches!(
                    msg,
                    AgentMessage::Env(EnvMessage::BashExecutionMessage { .. })
                ));
            }
            other => panic!("expected Message, got {other:?}"),
        }
        // Guard (logically dead, removed): the old assertion compared the entry kind
        // string against "bashExecution", a kind `entry_type()` can never return — bash
        // execution is a Message *role* (`message_role(...) == Some("bashExecution")`),
        // not an entry kind. The real invariant — bash executions are persisted
        // Message-typed, never a top-level non-Message entry — is asserted by the
        // role-based match above.
    }
}
