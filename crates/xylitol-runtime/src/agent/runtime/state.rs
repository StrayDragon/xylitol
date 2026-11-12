use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::agent::capabilities::ActiveTurnBinding;
use xylitol_plugin_sdk::message::AgentPart;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(u64);

impl RunId {
    #[cfg(test)]
    pub(crate) fn from_raw_for_test(raw: u64) -> Self {
        Self(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunPolicy {
    #[default]
    Reject,
    AbortAndReplace,
    QueueAfterRun,
}

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

#[derive(Clone)]
pub struct FrozenRootConfig {
    pub user_parts: Vec<AgentPart>,
    pub system_prompt: Option<String>,
    pub tools: crate::agent::tools::ToolSet,
    pub hooks: crate::agent::runtime::AgentHooks,
    pub batch_mode: xylitol_plugin_sdk::XyBatchMode,
    pub skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>,
    pub compaction_settings: crate::agent::compaction::CompactionSettings,
    pub compaction: std::sync::Arc<dyn crate::agent::compaction::XyCompaction>,
    pub permission: std::sync::Arc<dyn xylitol_plugin_sdk::XyPermission>,
    pub hook_bus: Option<std::sync::Arc<dyn xylitol_plugin_sdk::XyHookBus>>,
    pub cwd: String,
    pub workspaces: Vec<String>,
    pub pending_prepare: Option<xylitol_plugin_sdk::SessionPrepareSnapshot>,
    pub loop_chain: xylitol_plugin_sdk::LoopChain,
}

pub struct PendingRoot {
    run_id: RunId,
    frozen: FrozenRootConfig,
    activate_tx: Option<tokio::sync::oneshot::Sender<FrozenRootConfig>>,
}

struct ActiveTurnSlot {
    run_id: RunId,
    binding: ActiveTurnBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunEvent {
    Begin,
    StreamStarted(RunId),
    Cancel { clear_pending: bool },
    Finish(RunId),
}

#[derive(Clone)]
pub enum RunTransition {
    Began {
        run_id: RunId,
        cancel: CancellationToken,
    },
    Streaming,
    Cancelling,
    Finished {
        next: Option<(RunId, Box<FrozenRootConfig>)>,
    },
    Rejected,
}

pub struct RunCoordinator {
    next_id: u64,
    phase: RuntimePhase,
    cancel: Option<(RunId, CancellationToken)>,
    active_turn: Option<ActiveTurnSlot>,
    turn_in_flight: bool,
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
            turn_in_flight: false,
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

    pub fn transition(&mut self, event: RunEvent) -> RunTransition {
        match event {
            RunEvent::Begin => {
                if !self.phase.is_idle() {
                    return RunTransition::Rejected;
                }
                let run_id = self.allocate_run_id();
                let token = CancellationToken::new();
                self.phase = RuntimePhase::Starting(run_id);
                self.cancel = Some((run_id, token.clone()));
                self.turn_in_flight = false;
                RunTransition::Began {
                    run_id,
                    cancel: token,
                }
            }
            RunEvent::StreamStarted(run_id) => match self.phase {
                RuntimePhase::Starting(active) | RuntimePhase::Streaming(active)
                    if active == run_id =>
                {
                    self.phase = RuntimePhase::Streaming(run_id);
                    RunTransition::Streaming
                }
                _ => RunTransition::Rejected,
            },
            RunEvent::Cancel { clear_pending } => {
                if clear_pending {
                    self.pending.clear();
                }
                let Some((run_id, token)) = self.cancel.as_ref() else {
                    return RunTransition::Rejected;
                };
                let run_id = *run_id;
                match self.phase {
                    RuntimePhase::Starting(active) | RuntimePhase::Streaming(active)
                        if active == run_id =>
                    {
                        token.cancel();
                        self.phase = RuntimePhase::Cancelling(run_id);
                        RunTransition::Cancelling
                    }
                    RuntimePhase::Cancelling(active) if active == run_id => {
                        token.cancel();
                        RunTransition::Cancelling
                    }
                    _ => RunTransition::Rejected,
                }
            }
            RunEvent::Finish(run_id) => {
                if self.phase.active_run_id() != Some(run_id) {
                    self.pending.retain(|p| p.run_id != run_id);
                    return RunTransition::Finished { next: None };
                }
                self.clear_active_turn_if(run_id);
                if self.cancel.as_ref().is_some_and(|(id, _)| *id == run_id) {
                    self.cancel = None;
                }
                self.phase = RuntimePhase::Idle;
                RunTransition::Finished {
                    next: self
                        .activate_next_pending()
                        .map(|(run_id, frozen)| (run_id, Box::new(frozen))),
                }
            }
        }
    }

    pub fn try_begin_immediate(&mut self) -> Option<(RunId, CancellationToken)> {
        match self.transition(RunEvent::Begin) {
            RunTransition::Began { run_id, cancel } => Some((run_id, cancel)),
            _ => None,
        }
    }

    pub fn mark_streaming(&mut self, run_id: RunId) {
        let _ = self.transition(RunEvent::StreamStarted(run_id));
    }

    pub fn enqueue_pending(
        &mut self,
        frozen: FrozenRootConfig,
        replace_pending: bool,
    ) -> (RunId, tokio::sync::oneshot::Receiver<FrozenRootConfig>) {
        if replace_pending {
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

    pub fn cancel_active(&mut self, clear_pending: bool) -> bool {
        matches!(
            self.transition(RunEvent::Cancel { clear_pending }),
            RunTransition::Cancelling
        )
    }

    pub fn set_turn_in_flight(&mut self, in_flight: bool) {
        self.turn_in_flight = in_flight;
    }

    pub fn turn_in_flight(&self) -> bool {
        self.turn_in_flight
    }

    pub fn rearm_cancel_after_abort(&mut self, run_id: RunId) -> Option<CancellationToken> {
        if self.phase.active_run_id() != Some(run_id) {
            return None;
        }
        let token = CancellationToken::new();
        self.cancel = Some((run_id, token.clone()));
        if matches!(self.phase, RuntimePhase::Cancelling(_)) {
            self.phase = RuntimePhase::Streaming(run_id);
        }
        Some(token)
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

    pub fn finish_run(&mut self, run_id: RunId) -> Option<(RunId, FrozenRootConfig)> {
        match self.transition(RunEvent::Finish(run_id)) {
            RunTransition::Finished { next } => next.map(|(run_id, frozen)| (run_id, *frozen)),
            _ => None,
        }
    }

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
            let _ = tx.send(frozen.clone());
        }
        Some((run_id, frozen))
    }

    pub fn take_started_token(&self, run_id: RunId) -> Option<CancellationToken> {
        match &self.cancel {
            Some((id, token)) if *id == run_id => Some(token.clone()),
            _ => None,
        }
    }
}

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

    pub fn phase(&self) -> RuntimePhase {
        self.with(|c| c.phase())
    }

    pub fn with<R>(&self, f: impl FnOnce(&RunCoordinator) -> R) -> R {
        f(&self.lock())
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut RunCoordinator) -> R) -> R {
        f(&mut self.lock())
    }
}

pub struct RunLease {
    coordinator: SharedRunCoordinator,
    run_id: RunId,
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

    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        if let Some(queues) = &self.queues {
            queues.unbind_event_tx(self.run_id);
        }
        let _activated = self.coordinator.with_mut(|c| c.finish_run(self.run_id));
    }
}

impl Drop for RunLease {
    fn drop(&mut self) {
        if !self.finished {
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
    fn mark_streaming_does_not_overwrite_cancelling() {
        let mut c = RunCoordinator::new();
        let (id, _) = c.try_begin_immediate().expect("idle begin");
        c.mark_streaming(id);
        assert!(c.cancel_active(false));
        assert_eq!(c.phase(), RuntimePhase::Cancelling(id));
        c.mark_streaming(id);
        assert_eq!(
            c.phase(),
            RuntimePhase::Cancelling(id),
            "mark_streaming must not overwrite Cancelling"
        );
        assert!(c.finish_run(id).is_none());
        assert_eq!(c.phase(), RuntimePhase::Idle);
    }

    #[test]
    fn transition_closed_set_guards_stale_and_busy() {
        let mut c = RunCoordinator::new();
        let stale = RunId::from_raw_for_test(99);
        assert!(matches!(
            c.transition(RunEvent::StreamStarted(stale)),
            RunTransition::Rejected
        ));
        assert!(matches!(
            c.transition(RunEvent::Cancel {
                clear_pending: false
            }),
            RunTransition::Rejected
        ));
        let (id, _) = c.try_begin_immediate().expect("idle begin");
        assert_eq!(c.phase(), RuntimePhase::Starting(id));
        assert!(c.try_begin_immediate().is_none(), "busy begin rejected");
        assert!(matches!(
            c.transition(RunEvent::StreamStarted(stale)),
            RunTransition::Rejected
        ));
        c.mark_streaming(id);
        assert_eq!(c.phase(), RuntimePhase::Streaming(id));
        c.mark_streaming(id);
        assert_eq!(c.phase(), RuntimePhase::Streaming(id));
        assert!(matches!(
            c.transition(RunEvent::Finish(stale)),
            RunTransition::Finished { next: None }
        ));
        assert_eq!(c.phase(), RuntimePhase::Streaming(id));
        assert!(c.finish_run(id).is_none());
        assert_eq!(c.phase(), RuntimePhase::Idle);
    }

    #[test]
    fn rearm_after_abort_issues_fresh_token_and_restores_streaming() {
        let mut c = RunCoordinator::new();
        let (id, token) = c.try_begin_immediate().unwrap();
        c.mark_streaming(id);
        assert!(c.cancel_active(false));
        assert!(token.is_cancelled());
        let fresh = c.rearm_cancel_after_abort(id).expect("rearm");
        assert_eq!(c.phase(), RuntimePhase::Streaming(id));
        assert!(!fresh.is_cancelled());
        assert!(!c.cancel_token().is_cancelled());
        c.cancel_active(false);
        assert!(fresh.is_cancelled());
        c.finish_run(id);
        assert!(c.rearm_cancel_after_abort(id).is_none());
        assert_eq!(c.phase(), RuntimePhase::Idle);
    }

    #[test]
    fn turn_in_flight_resets_on_begin_and_tracks_probe() {
        let mut c = RunCoordinator::new();
        assert!(!c.turn_in_flight());
        c.set_turn_in_flight(true);
        assert!(c.turn_in_flight());
        let (id, _) = c.try_begin_immediate().unwrap();
        assert!(
            !c.turn_in_flight(),
            "a fresh begin starts with no turn in flight"
        );
        c.finish_run(id);
        c.set_turn_in_flight(true);
        c.set_turn_in_flight(false);
        assert!(!c.turn_in_flight());
    }

    #[test]
    fn stale_finish_does_not_clear_newer_run() {
        let mut c = RunCoordinator::new();
        let (old, _) = c.try_begin_immediate().unwrap();
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
        assert!(c.finish_run(old).is_none());
        assert!(c.has_active_turn());
        assert_eq!(c.phase().active_run_id(), Some(new));
    }
}
