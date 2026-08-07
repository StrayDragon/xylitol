//! Compaction orchestration methods on [`AgentCapabilities`].

use super::{AgentCapabilities, observe_hook};
use crate::protocol::error::XyError;

impl AgentCapabilities {
    /// Check and perform auto-compaction if the context is full.
    /// Returns true if compaction was performed.
    pub async fn maybe_auto_compact(&self) -> Result<bool, XyError> {
        self.maybe_auto_compact_with(
            &crate::agent::compaction::EstimateOpts {
                model_id: self.current_model().map(|m| m.id.clone()),
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
    ) -> Result<bool, XyError> {
        let sid = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?;

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
            )
            .await
            .map_err(|e| XyError::Session(anyhow::anyhow!(e)))?;

        if compacted && let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(compacted)
    }

    /// Manual force compact (pi `compact(customInstructions?)`). Does not apply the reserve gate.
    pub async fn force_compact(&self, instructions: Option<String>) -> Result<(), XyError> {
        let sid = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?;

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
            )
            .await
            .map_err(|e| XyError::Session(anyhow::anyhow!(e)))?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(())
    }
}
