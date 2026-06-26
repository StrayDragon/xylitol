//! BashExecHandler — `!cmd` / `!!cmd` execution and cancellation (spec c255 / as32).
//!
//! Extracted from `AgentSession` (per the revised `architecture/ar02` upper
//! bound). Owns the cancellation token for in-flight bash executions.
//! `AgentSession` composes a `BashExecHandler` and delegates, preserving the
//! public API (as31). Persistence stays on `AgentSession` (it owns session IO).

use tokio_util::sync::CancellationToken;

/// Bash execution collaborator: owns the in-flight cancellation token and the
/// raw executor invocation.
#[derive(Default)]
pub struct BashExecHandler {
    /// Active cancellation token (`Some` while a `!`/`!!` command runs).
    bash_cancel: Option<CancellationToken>,
}

impl BashExecHandler {
    pub fn new() -> Self {
        Self { bash_cancel: None }
    }

    /// Run a bash command under a fresh cancellation token, returning the
    /// raw result. Persistence is the caller's responsibility.
    pub async fn execute_raw(&mut self, command: &str) -> crate::agent::runtime::bash::BashResult {
        let cancel = CancellationToken::new();
        self.bash_cancel = Some(cancel.clone());

        let result = crate::agent::runtime::bash::execute(
            command,
            crate::agent::runtime::bash::BashExecutorOptions {
                cancel: Some(cancel),
                ..Default::default()
            },
        )
        .await;

        self.bash_cancel = None;
        result
    }

    /// Abort any in-flight bash execution.
    pub fn abort(&mut self) {
        if let Some(cancel) = self.bash_cancel.take() {
            cancel.cancel();
        }
    }
}
