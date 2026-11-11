use std::sync::Arc;

use async_trait::async_trait;
use xylitol_plugin_sdk::session::{CustomMessageEntry, EntryBase, SessionEntry};
use xylitol_plugin_sdk::{
    AfterStepCtx, BeforeStepCtx, LoopMiddleware, LoopStage, LoopStep, XySessionStore,
};

use crate::config::{DedupMode, InjectorPoint, Placement};
use crate::registry::{ContextInjector, ContextInjectorRegistry, InjectorContext, InjectorSource};

pub const CUSTOM_TYPE_CONTEXT_REMINDER: &str = "context_reminder";

pub struct ContextInjectMiddleware {
    registry: Arc<ContextInjectorRegistry>,
    store: Option<Arc<dyn XySessionStore>>,
    seen: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl ContextInjectMiddleware {
    pub fn new(
        registry: Arc<ContextInjectorRegistry>,
        store: Option<Arc<dyn XySessionStore>>,
    ) -> Self {
        Self {
            registry,
            store,
            seen: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn before_points(ctx: &BeforeStepCtx) -> Vec<InjectorPoint> {
        let mut points = vec![InjectorPoint::StepStart];
        if ctx.has_new_user_messages {
            points.push(InjectorPoint::TurnPre);
        }
        if ctx.step_index == 0 {
            points.push(InjectorPoint::RunStart);
            points.push(InjectorPoint::SessionStart);
        }
        points
    }

    fn after_points(ctx: &AfterStepCtx) -> Vec<InjectorPoint> {
        let mut points = vec![InjectorPoint::StepEnd];
        if !ctx.continued_tools {
            points.push(InjectorPoint::RunEnd);
        }
        points
    }

    async fn collect(
        &self,
        injector: &ContextInjector,
        ctx: Option<&BeforeStepCtx>,
        session_id: Option<&str>,
    ) -> Option<String> {
        let text = match &injector.source {
            InjectorSource::Native(native) => {
                let before = ctx?;
                let injector_ctx = InjectorContext {
                    before,
                    store: self.store.as_deref(),
                    session_id,
                };
                native.produce(&injector_ctx).await?
            }
            InjectorSource::Command {
                command,
                timeout_secs,
            } => {
                let timeout = std::time::Duration::from_secs(*timeout_secs);
                match xylitol_plugin_sdk::host_env::run_command_capture(command, timeout, 4096)
                    .await
                {
                    Ok(text) if !text.is_empty() => text,
                    Ok(_) => return None,
                    Err(error) => {
                        log::warn!(
                            target: "xylitol::context_inject",
                            "injector {} failed kind={} error={}",
                            injector.id,
                            error.kind,
                            error.message
                        );
                        return None;
                    }
                }
            }
        };

        let key = format!("{}:{}", session_id.unwrap_or("-"), injector.id);
        if injector.dedup != DedupMode::None {
            let mut seen = self
                .seen
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if seen.get(&key) == Some(&text) {
                return None;
            }
            seen.insert(key, text.clone());
        }

        if injector.placement == Placement::HistoryRow
            && let (Some(store), Some(session_id)) = (self.store.as_deref(), session_id)
        {
            let history_text = match injector.group.as_deref() {
                Some(group) => format!("<{group}>\n{text}\n</{group}>"),
                None => text.clone(),
            };
            append_history_row(store, session_id, &injector.id, &history_text).await;
        }
        Some(text)
    }

    async fn before_rows(&self, ctx: &BeforeStepCtx) -> Vec<(Option<String>, String)> {
        if self.registry.is_empty() {
            return Vec::new();
        }
        let points = Self::before_points(ctx);
        let mut rows = Vec::new();
        for injector in self.registry.snapshot() {
            if !points.contains(&injector.point) {
                continue;
            }
            if let Some(text) = self
                .collect(&injector, Some(ctx), ctx.session_id.as_deref())
                .await
            {
                rows.push((injector.group.clone(), text));
            }
        }
        rows
    }

    async fn after_rows(&self, ctx: &AfterStepCtx) -> Vec<(Option<String>, String)> {
        if self.registry.is_empty() {
            return Vec::new();
        }
        let points = Self::after_points(ctx);
        let mut rows = Vec::new();
        for injector in self.registry.snapshot() {
            if !points.contains(&injector.point) {
                continue;
            }
            if let Some(text) = self
                .collect(&injector, None, ctx.session_id.as_deref())
                .await
            {
                rows.push((injector.group.clone(), text));
            }
        }
        rows
    }
}

fn group_fragments(rows: Vec<(Option<String>, String)>) -> Vec<String> {
    let mut grouped: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut plain = Vec::new();
    for (group, text) in rows {
        match group {
            Some(group) => grouped.entry(group).or_default().push(text),
            None => plain.push(text),
        }
    }
    let mut fragments = Vec::new();
    for (group, lines) in grouped {
        fragments.push(format!("<{group}>\n{}\n</{group}>", lines.join("\n")));
    }
    fragments.extend(plain);
    fragments
}

async fn append_history_row(
    store: &dyn XySessionStore,
    session_id: &str,
    injector_id: &str,
    text: &str,
) {
    let entry = SessionEntry::CustomMessage(CustomMessageEntry {
        base: EntryBase {
            entry_type: "customMessage".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        custom_type: CUSTOM_TYPE_CONTEXT_REMINDER.into(),
        content: serde_json::json!(text),
        display: true,
        details: Some(serde_json::json!({ "injector": injector_id })),
    });
    if let Err(error) = store.append_session_entry(session_id, &entry).await {
        log::warn!(
            target: "xylitol::context_inject",
            "context reminder append failed session={session_id} injector={injector_id} error={error}"
        );
    }
}

#[async_trait]
impl LoopMiddleware for ContextInjectMiddleware {
    fn id(&self) -> &str {
        "context-inject"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::BeforeStep, LoopStage::AfterStep]
    }

    async fn before_step(&self, mut ctx: BeforeStepCtx) -> LoopStep<BeforeStepCtx> {
        let rows = self.before_rows(&ctx).await;
        if !rows.is_empty() {
            ctx.system_fragments.extend(group_fragments(rows));
        }
        LoopStep::continued(ctx)
    }

    async fn after_step(&self, ctx: AfterStepCtx) -> LoopStep<AfterStepCtx> {
        let _ = self.after_rows(&ctx).await;
        LoopStep::continued(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_points_follow_the_boundary() {
        let mut ctx = BeforeStepCtx {
            step_index: 0,
            has_new_user_messages: true,
            ..Default::default()
        };
        let points = ContextInjectMiddleware::before_points(&ctx);
        assert!(points.contains(&InjectorPoint::SessionStart));
        assert!(points.contains(&InjectorPoint::RunStart));
        assert!(points.contains(&InjectorPoint::TurnPre));
        assert!(points.contains(&InjectorPoint::StepStart));

        ctx.step_index = 1;
        ctx.has_new_user_messages = false;
        let points = ContextInjectMiddleware::before_points(&ctx);
        assert_eq!(points, vec![InjectorPoint::StepStart]);
    }

    #[test]
    fn after_points_include_run_end_at_settle() {
        let ctx = AfterStepCtx {
            step_index: 1,
            continued_tools: false,
            history_len: 0,
            session_id: None,
        };
        let points = ContextInjectMiddleware::after_points(&ctx);
        assert!(points.contains(&InjectorPoint::StepEnd));
        assert!(points.contains(&InjectorPoint::RunEnd));
    }

    #[test]
    fn group_fragments_aggregate_same_group() {
        let rows = vec![
            (Some("system-reminder".into()), "[skills] a".into()),
            (Some("system-reminder".into()), "[tools] b".into()),
            (None, "plain".into()),
        ];
        let fragments = group_fragments(rows);
        assert_eq!(fragments.len(), 2);
        assert_eq!(
            fragments[0],
            "<system-reminder>\n[skills] a\n[tools] b\n</system-reminder>"
        );
        assert_eq!(fragments[1], "plain");
    }
}
