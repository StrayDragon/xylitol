//! Compaction orchestration methods on [`AgentCapabilities`].

use super::{AgentCapabilities, observe_hook};
use crate::agent::compaction::CompactionError;
use crate::protocol::error::XySessionError;

impl AgentCapabilities {
    /// Check and perform auto-compaction if the context is full.
    /// Returns true if compaction was performed.
    pub async fn maybe_auto_compact(&self) -> Result<bool, CompactionError> {
        self.maybe_auto_compact_with(
            &crate::agent::compaction::EstimateOpts {
                model_id: self.current_model().map(|m| m.id.clone()),
                fixed_context: Some(self.fixed_request_context()),
                ..Default::default()
            },
            None,
        )
        .await
    }

    /// Auto-compact using the same estimate opts as the product footer (c1420).
    ///
    /// `last_assistant`: when set, abort / stale guards apply (pi `_checkCompaction`).
    pub async fn maybe_auto_compact_with(
        &self,
        estimate_opts: &crate::agent::compaction::EstimateOpts,
        last_assistant: Option<&crate::protocol::message::AgentMessage>,
    ) -> Result<bool, CompactionError> {
        let sid = self
            .session_id()
            .ok_or(CompactionError::from(XySessionError::NoActiveSession))?;

        let model = self.build_current_model()?;

        let ctx_window = self
            .current_model()
            .map(|m| m.context_window)
            .unwrap_or(128000);

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        let compacted = self
            .compaction_orchestrator
            .maybe_auto_compact(
                self.store.as_ref(),
                sid,
                model.as_ref(),
                self.sink.as_ref(),
                ctx_window,
                estimate_opts,
                last_assistant,
                None,
                Some(&self.fixed_request_context()),
                None,
                &self.obs_session_snapshot_from_store().await,
            )
            .await?;

        if compacted && let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(compacted)
    }

    /// Manual force compact (pi `compact(customInstructions?)`). Does not apply the reserve gate.
    pub async fn force_compact(&self, instructions: Option<String>) -> Result<(), CompactionError> {
        let sid = self
            .session_id()
            .ok_or(CompactionError::from(XySessionError::NoActiveSession))?;

        let model = self.build_current_model()?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        self.compaction_orchestrator
            .compact(
                self.store.as_ref(),
                sid,
                model.as_ref(),
                self.sink.as_ref(),
                instructions,
                self.current_model().map(|m| m.context_window).unwrap_or(0),
                Some(&self.fixed_request_context()),
                &self.obs_session_snapshot_from_store().await,
            )
            .await?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(())
    }

    /// Resume / session-activation settlement (c26 LeafChanged): one
    /// overhead-aware estimate so a freshly attached / switched client's footer
    /// and the next reserve gate see the real context size without a model call.
    pub async fn emit_leaf_changed_settlement(&self) {
        use crate::agent::compaction::{
            ContextTokenSettlementReason, EstimateOpts, settle_from_session_entries,
        };
        let Some(sid) = self.session_id().map(str::to_string) else {
            return;
        };
        let Ok(entries) = self.store.load_leaf_branch(&sid).await else {
            return;
        };
        let settled = settle_from_session_entries(
            &entries,
            &EstimateOpts {
                model_id: self.current_model().map(|m| m.id.clone()),
                fixed_context: Some(self.fixed_request_context()),
                ..Default::default()
            },
            ContextTokenSettlementReason::LeafChanged,
        );
        self.sink
            .emit(
                &crate::protocol::lifecycle::XyEvent::ContextTokenSettlement {
                    estimate: settled.estimate,
                    reason: settled.reason.as_str().to_string(),
                    generation: settled.generation,
                },
            )
            .await;
    }
}
