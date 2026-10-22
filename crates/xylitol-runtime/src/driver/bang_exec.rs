use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::session::bash_execution_message_entry;
use xylitol_plugin_sdk::{BashExecOpts, XyBashExecutor, XyBashResult, XySessionStore};

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

        *crate::utils::lock_mutex(&self.cancel) = Some(cancel.clone());

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
