//! Session-bound run coordination — single-flight root turns.
//!
//! One [`AgentRuntime`](super::AgentRuntime) owns one bound session and at most
//! one live ReAct worker. [`RunPolicy`] only schedules the next root turn; it
//! never creates a second concurrent loop.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::agent::capabilities::ActiveTurnBinding;
use crate::protocol::message::AgentPart;

/// Opaque identity for one root-turn attempt (active or queued).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(u64);

impl RunId {
    pub fn get(self) -> u64 {
        self.0
    }

    #[cfg(test)]
    pub(crate) fn from_raw_for_test(raw: u64) -> Self {
        Self(raw)
    }
}

/// How to handle a root submit while another root turn is live or queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunPolicy {
    /// Refuse the new root turn; leave the active turn untouched.
    #[default]
    Reject,
    /// Cancel the active turn, then start this turn after it finishes cleanup.
    AbortAndReplace,
    /// Enqueue this turn FIFO; start after the active turn finishes naturally.
    QueueAfterRun,
}

/// Control-plane errors for bind / busy mutations (not stream `XyEvent::Error`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeControlError {
    #[error("agent run already active")]
    Busy,
    #[error("no session bound; call bind_session first")]
    NoSession,
    #[error("cannot rebind session while a run is active or queued")]
    SessionBusy,
}

impl RuntimeControlError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Busy | Self::SessionBusy => "Busy",
            Self::NoSession => "NoSession",
        }
    }
}

/// Coordinator phase for the single session actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePhase {
    Idle,
    Starting(RunId),
    Streaming(RunId),
    Cancelling(RunId),
}

impl RuntimePhase {
    pub fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn active_run_id(self) -> Option<RunId> {
        match self {
            Self::Idle => None,
            Self::Starting(id) | Self::Streaming(id) | Self::Cancelling(id) => Some(id),
        }
    }
}

/// Frozen capability snapshot for a queued / starting root turn.
#[derive(Clone)]
pub(crate) struct FrozenRootConfig {
    pub user_parts: Vec<AgentPart>,
    pub system_prompt: Option<String>,
    pub tools: crate::agent::tools::ToolSet,
    pub hooks: crate::agent::runtime::AgentHooks,
    pub batch_mode: crate::protocol::ports::XyBatchMode,
    pub skills: Vec<crate::protocol::resource::SkillInfo>,
    pub compaction_settings: crate::agent::compaction::CompactionSettings,
    pub permission: std::sync::Arc<dyn crate::protocol::ports::XyPermission>,
    pub hook_bus: Option<std::sync::Arc<dyn crate::protocol::ports::XyHookBus>>,
    /// Workspace cwd for session_env bootstrap (c1905).
    pub cwd: String,
}

struct PendingRoot {
    run_id: RunId,
    frozen: FrozenRootConfig,
    /// Wakes the waiting event stream when this turn may start.
    activate_tx: Option<tokio::sync::oneshot::Sender<FrozenRootConfig>>,
}

struct ActiveTurnSlot {
    run_id: RunId,
    binding: ActiveTurnBinding,
}

/// Single-flight run coordinator shared with leases and queue bind/unbind.
pub(crate) struct RunCoordinator {
    next_id: u64,
    phase: RuntimePhase,
    cancel: Option<(RunId, CancellationToken)>,
    active_turn: Option<ActiveTurnSlot>,
    pending: VecDeque<PendingRoot>,
}

impl Default for RunCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl RunCoordinator {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            phase: RuntimePhase::Idle,
            cancel: None,
            active_turn: None,
            pending: VecDeque::new(),
        }
    }

    pub fn phase(&self) -> RuntimePhase {
        self.phase
    }

    pub fn has_active_run(&self) -> bool {
        !self.phase.is_idle()
    }

    pub fn has_work(&self) -> bool {
        self.has_active_run() || !self.pending.is_empty()
    }

    pub fn allocate_run_id(&mut self) -> RunId {
        let id = RunId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Begin a root turn immediately. Returns `None` when not idle (caller applies policy).
    pub fn try_begin_immediate(&mut self) -> Option<(RunId, CancellationToken)> {
        if !self.phase.is_idle() {
            return None;
        }
        let run_id = self.allocate_run_id();
        let token = CancellationToken::new();
        self.phase = RuntimePhase::Starting(run_id);
        self.cancel = Some((run_id, token.clone()));
        Some((run_id, token))
    }

    pub fn mark_streaming(&mut self, run_id: RunId) {
        if self.phase.active_run_id() == Some(run_id) {
            self.phase = RuntimePhase::Streaming(run_id);
        }
    }

    /// Enqueue a root turn; returns the waiting receiver used by the event stream.
    pub fn enqueue_pending(
        &mut self,
        frozen: FrozenRootConfig,
        replace_pending: bool,
    ) -> (RunId, tokio::sync::oneshot::Receiver<FrozenRootConfig>) {
        if replace_pending {
            // Drop queued roots (their streams observe closed oneshot → no start).
            self.pending.clear();
        }
        let run_id = self.allocate_run_id();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.push_back(PendingRoot {
            run_id,
            frozen,
            activate_tx: Some(tx),
        });
        (run_id, rx)
    }

    /// Cancel the active run token (AbortAndReplace / abort()). Keeps pending roots
    /// unless `clear_pending` is set.
    pub fn cancel_active(&mut self, clear_pending: bool) -> bool {
        if clear_pending {
            self.pending.clear();
        }
        let Some((run_id, token)) = self.cancel.as_ref() else {
            return false;
        };
        let run_id = *run_id;
        token.cancel();
        self.phase = RuntimePhase::Cancelling(run_id);
        true
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel
            .as_ref()
            .map(|(_, t)| t.clone())
            .unwrap_or_default()
    }

    pub fn set_active_turn(&mut self, run_id: RunId, binding: ActiveTurnBinding) {
        if self.phase.active_run_id() == Some(run_id) {
            self.active_turn = Some(ActiveTurnSlot { run_id, binding });
        }
    }

    pub fn clear_active_turn_if(&mut self, run_id: RunId) {
        if self
            .active_turn
            .as_ref()
            .is_some_and(|s| s.run_id == run_id)
        {
            self.active_turn = None;
        }
    }

    pub fn inflight_turn_binding(&self) -> Option<ActiveTurnBinding> {
        self.active_turn.as_ref().map(|s| s.binding.clone())
    }

    pub fn has_active_turn(&self) -> bool {
        self.active_turn.is_some()
    }

    /// Finish a run lease. Activates the next pending root when the actor becomes free.
    pub fn finish_run(&mut self, run_id: RunId) -> Option<(RunId, FrozenRootConfig)> {
        // Ignore stale leases from older runs.
        if self.phase.active_run_id() != Some(run_id) {
            // Still allow cancelling a queued ticket that never started.
            self.pending.retain(|p| p.run_id != run_id);
            return None;
        }

        self.clear_active_turn_if(run_id);
        if self.cancel.as_ref().is_some_and(|(id, _)| *id == run_id) {
            self.cancel = None;
        }
        self.phase = RuntimePhase::Idle;

        self.activate_next_pending()
    }

    /// Drop a queued (not yet started) root without affecting the active run.
    pub fn revoke_pending(&mut self, run_id: RunId) {
        self.pending.retain(|p| p.run_id != run_id);
    }

    fn activate_next_pending(&mut self) -> Option<(RunId, FrozenRootConfig)> {
        let pending = self.pending.pop_front()?;
        let run_id = pending.run_id;
        let frozen = pending.frozen;
        let token = CancellationToken::new();
        self.phase = RuntimePhase::Starting(run_id);
        self.cancel = Some((run_id, token));
        if let Some(tx) = pending.activate_tx {
            // Receiver builds the live stream; include frozen config.
            let _ = tx.send(frozen.clone());
        }
        Some((run_id, frozen))
    }

    /// Token + run id for a pending turn that just became Starting via finish_run.
    pub fn take_started_token(&self, run_id: RunId) -> Option<CancellationToken> {
        match &self.cancel {
            Some((id, token)) if *id == run_id => Some(token.clone()),
            _ => None,
        }
    }
}

/// Shared handle used by AgentRuntime, leases, and ReAct helpers.
#[derive(Clone, Default)]
pub struct SharedRunCoordinator {
    inner: Arc<Mutex<RunCoordinator>>,
}

impl SharedRunCoordinator {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RunCoordinator::new())),
        }
    }

    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, RunCoordinator> {
        crate::utils::lock_mutex(&self.inner)
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&RunCoordinator) -> R) -> R {
        f(&self.lock())
    }

    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(&mut RunCoordinator) -> R) -> R {
        f(&mut self.lock())
    }
}

/// Held by [`super::XyEventStream`]; finishes the run on Drop / AgentEnd.
pub struct RunLease {
    coordinator: SharedRunCoordinator,
    run_id: RunId,
    /// Queue runtime to unbind EventTx for this run only.
    queues: Option<std::sync::Arc<crate::agent::capabilities::AsyncQueueRuntime>>,
    finished: bool,
}

impl RunLease {
    pub fn new(
        coordinator: SharedRunCoordinator,
        run_id: RunId,
        queues: Option<std::sync::Arc<crate::agent::capabilities::AsyncQueueRuntime>>,
    ) -> Self {
        Self {
            coordinator,
            run_id,
            queues,
            finished: false,
        }
    }

    pub fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Idempotent finish — safe from AgentEnd poll and Drop.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        if let Some(queues) = &self.queues {
            queues.unbind_event_tx(self.run_id);
        }
        let _activated = self.coordinator.with_mut(|c| c.finish_run(self.run_id));
        // Pending activation is delivered via oneshot inside finish_run; the
        // waiting stream owns starting ReAct after recv.
    }
}

impl Drop for RunLease {
    fn drop(&mut self) {
        if !self.finished {
            // Queued-but-not-started streams only revoke the pending ticket.
            let phase_id = self.coordinator.with(|c| c.phase().active_run_id());
            if phase_id != Some(self.run_id) {
                self.coordinator.with_mut(|c| c.revoke_pending(self.run_id));
                self.finished = true;
                return;
            }
            self.finish();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_path_try_begin_only_when_idle() {
        let mut c = RunCoordinator::new();
        let (id, _) = c.try_begin_immediate().expect("first");
        assert!(c.try_begin_immediate().is_none());
        assert_eq!(c.phase().active_run_id(), Some(id));
        assert!(c.finish_run(id).is_none());
        assert!(!c.has_work());
        assert!(c.try_begin_immediate().is_some());
    }

    #[test]
    fn stale_finish_does_not_clear_newer_run() {
        let mut c = RunCoordinator::new();
        let (old, _) = c.try_begin_immediate().unwrap();
        // Simulate replace: cancel + finish old, begin new.
        c.cancel_active(true);
        assert!(c.finish_run(old).is_none());
        let (new, _) = c.try_begin_immediate().unwrap();
        c.set_active_turn(
            new,
            ActiveTurnBinding {
                model_id: "m".into(),
                display_name: "m".into(),
                thinking: "off".into(),
                omit_thinking: true,
            },
        );
        // Stale finish must not wipe new active turn.
        assert!(c.finish_run(old).is_none());
        assert!(c.has_active_turn());
        assert_eq!(c.phase().active_run_id(), Some(new));
    }
}
