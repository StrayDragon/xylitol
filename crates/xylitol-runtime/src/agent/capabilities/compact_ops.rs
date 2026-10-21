use super::{AgentCapabilities, observe_hook};
use crate::agent::compaction::CompactionError;
use xylitol_plugin_sdk::error::XySessionError;

impl AgentCapabilities {
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

    pub async fn maybe_auto_compact_with(
        &self,
        estimate_opts: &crate::agent::compaction::EstimateOpts,
        last_assistant: Option<&xylitol_plugin_sdk::message::AgentMessage>,
    ) -> Result<bool, CompactionError> {
        let sid = self
            .session_id()
            .ok_or(CompactionError::from(XySessionError::NoActiveSession))?;

        let obs_session = self.obs_session_snapshot_from_store().await;
        let summary = self.with_models(|mm| {
            crate::agent::model::task_model::resolve_compaction_summary(
                self.compaction_orchestrator.settings(),
                mm,
                &obs_session,
            )
        })?;

        let ctx_window = self
            .current_model()
            .map(|m| m.context_window)
            .unwrap_or(128000);

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        let mut fallback_notice_emitted = false;
        let compacted = self
            .compaction_orchestrator
            .maybe_auto_compact(
                self.store.as_ref(),
                sid,
                &summary,
                self.sink.as_ref(),
                ctx_window,
                estimate_opts,
                last_assistant,
                None,
                Some(&self.fixed_request_context()),
                None,
                &mut fallback_notice_emitted,
                None,
            )
            .await?;

        if compacted && let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(compacted)
    }

    pub async fn force_compact(&self, instructions: Option<String>) -> Result<(), CompactionError> {
        let sid = self
            .session_id()
            .ok_or(CompactionError::from(XySessionError::NoActiveSession))?;

        let obs_session = self.obs_session_snapshot_from_store().await;
        let summary = self.with_models(|mm| {
            crate::agent::model::task_model::resolve_compaction_summary(
                self.compaction_orchestrator.settings(),
                mm,
                &obs_session,
            )
        })?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        let mut fallback_notice_emitted = false;
        self.compaction_orchestrator
            .compact(
                self.store.as_ref(),
                sid,
                &summary,
                self.sink.as_ref(),
                instructions,
                self.current_model().map(|m| m.context_window).unwrap_or(0),
                Some(&self.fixed_request_context()),
                &mut fallback_notice_emitted,
            )
            .await?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(())
    }

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
                &xylitol_plugin_sdk::lifecycle::XyEvent::ContextTokenSettlement {
                    estimate: settled.estimate,
                    reason: settled.reason.as_str().to_string(),
                    generation: settled.generation,
                },
            )
            .await;
    }
}
