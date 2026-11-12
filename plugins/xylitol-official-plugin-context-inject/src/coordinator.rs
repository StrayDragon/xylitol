use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use xylitol_plugin_sdk::message::{AgentMessage, EnvMessage};
use xylitol_plugin_sdk::session::{CustomMessageEntry, EntryBase, SessionEntry};
use xylitol_plugin_sdk::{
    AfterStepCtx, BeforeStepCtx, LoopMiddleware, LoopStage, LoopStep, XySessionStore,
    XyTurnInjector,
};

use crate::config::{DedupMode, InjectorPoint, Placement};
use crate::registry::{ContextInjector, ContextInjectorRegistry, InjectorContext, InjectorSource};

pub const CUSTOM_TYPE_CONTEXT_REMINDER: &str = "context_reminder";

pub struct ContextInjectMiddleware {
    registry: Arc<ContextInjectorRegistry>,
    store: Option<Arc<dyn XySessionStore>>,
    seen: Mutex<HashMap<String, String>>,
    started: Mutex<HashSet<String>>,
}

type GroupedInjectors = Vec<(Option<String>, Vec<(String, String, bool)>)>;

struct InjectorRow {
    group: Option<String>,
    text: String,
    injector_ids: Vec<String>,
    persist: bool,
}

impl ContextInjectMiddleware {
    pub fn new(
        registry: Arc<ContextInjectorRegistry>,
        store: Option<Arc<dyn XySessionStore>>,
    ) -> Self {
        Self {
            registry,
            store,
            seen: Mutex::new(HashMap::new()),
            started: Mutex::new(HashSet::new()),
        }
    }

    fn before_points(_ctx: &BeforeStepCtx) -> Vec<InjectorPoint> {
        vec![InjectorPoint::StepStart]
    }

    fn after_points(ctx: &AfterStepCtx) -> Vec<InjectorPoint> {
        let mut points = vec![InjectorPoint::StepEnd];
        if !ctx.continued_tools {
            points.push(InjectorPoint::RunEnd);
        }
        points
    }

    async fn submission_points(&self, ctx: &BeforeStepCtx) -> Vec<InjectorPoint> {
        let session_key = ctx.session_id.clone().unwrap_or_default();
        let first_of_session = {
            let mut started = self
                .started
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            started.insert(session_key)
        };
        if !first_of_session {
            return vec![InjectorPoint::RunStart, InjectorPoint::TurnPre];
        }
        let mut points = Vec::new();
        if self.session_has_transcript(ctx.session_id.as_deref()).await {
            points.push(InjectorPoint::SessionShutdown);
        }
        points.push(InjectorPoint::SessionStart);
        points.push(InjectorPoint::RunStart);
        points.push(InjectorPoint::TurnPre);
        points
    }

    async fn session_has_transcript(&self, session_id: Option<&str>) -> bool {
        let (Some(store), Some(session_id)) = (self.store.as_deref(), session_id) else {
            return false;
        };
        store
            .load_entries(session_id)
            .await
            .map(|entries| entries.iter().any(|entry| entry.anchors_transcript()))
            .unwrap_or(false)
    }

    async fn produce(
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

        Some(text)
    }

    async fn evaluate(
        &self,
        points: &[InjectorPoint],
        ctx: Option<&BeforeStepCtx>,
        session_id: Option<&str>,
    ) -> Vec<InjectorRow> {
        if self.registry.is_empty() {
            return Vec::new();
        }
        let mut grouped: GroupedInjectors = Vec::new();
        for injector in self.registry.snapshot() {
            if !points.contains(&injector.point) {
                continue;
            }
            let Some(text) = self.produce(&injector, ctx, session_id).await else {
                continue;
            };
            let persist = injector.placement == Placement::HistoryRow;
            if injector.group.is_none() {
                grouped.push((None, vec![(injector.id.clone(), text, persist)]));
                continue;
            }
            let slot = grouped
                .iter_mut()
                .find(|(group, _)| group.as_deref() == injector.group.as_deref());
            match slot {
                Some((_, items)) => items.push((injector.id.clone(), text, persist)),
                None => grouped.push((
                    injector.group.clone(),
                    vec![(injector.id.clone(), text, persist)],
                )),
            }
        }
        grouped
            .into_iter()
            .map(|(group, items)| {
                let persist = items.iter().any(|(_, _, persist)| *persist);
                let injector_ids = items
                    .iter()
                    .map(|(id, _, _)| id.clone())
                    .collect::<Vec<_>>();
                let lines = items
                    .into_iter()
                    .map(|(_, text, _)| text)
                    .collect::<Vec<_>>();
                let text = match group.as_deref() {
                    Some(group) => format!("<{group}>\n{}\n</{group}>", lines.join("\n")),
                    None => lines.join("\n"),
                };
                InjectorRow {
                    group,
                    text,
                    injector_ids,
                    persist,
                }
            })
            .collect()
    }

    async fn append_row(&self, session_id: Option<&str>, row: &InjectorRow) {
        let (Some(store), Some(session_id)) = (self.store.as_deref(), session_id) else {
            return;
        };
        let entry = SessionEntry::CustomMessage(CustomMessageEntry {
            base: EntryBase {
                entry_type: "customMessage".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_CONTEXT_REMINDER.into(),
            content: serde_json::json!(row.text),
            display: true,
            details: Some(serde_json::json!({ "injectors": row.injector_ids })),
        });
        if let Err(error) = store.append_session_entry(session_id, &entry).await {
            log::warn!(
                target: "xylitol::context_inject",
                "context reminder append failed session={session_id} injectors={:?} error={error}",
                row.injector_ids
            );
        }
    }

    fn agent_message(row: &InjectorRow) -> AgentMessage {
        AgentMessage::Env(EnvMessage::CustomMessage {
            custom_type: CUSTOM_TYPE_CONTEXT_REMINDER.into(),
            content: serde_json::json!(row.text),
            display: serde_json::json!(true),
            details: serde_json::json!({ "injectors": row.injector_ids }),
        })
    }

    async fn before_rows(&self, ctx: &BeforeStepCtx) -> Vec<(Option<String>, String)> {
        let points = Self::before_points(ctx);
        let rows = self
            .evaluate(&points, Some(ctx), ctx.session_id.as_deref())
            .await;
        let mut out = Vec::new();
        for row in rows {
            if row.persist {
                self.append_row(ctx.session_id.as_deref(), &row).await;
            }
            out.push((row.group, row.text));
        }
        out
    }

    async fn after_rows(&self, ctx: &AfterStepCtx) -> Vec<(Option<String>, String)> {
        let points = Self::after_points(ctx);
        let rows = self
            .evaluate(&points, None, ctx.session_id.as_deref())
            .await;
        let mut out = Vec::new();
        for row in rows {
            if row.persist {
                self.append_row(ctx.session_id.as_deref(), &row).await;
            }
            out.push((row.group, row.text));
        }
        out
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

#[async_trait]
impl XyTurnInjector for ContextInjectMiddleware {
    fn id(&self) -> &str {
        "context-inject"
    }

    async fn on_turn_submission(&self, ctx: &BeforeStepCtx) -> Vec<AgentMessage> {
        let points = self.submission_points(ctx).await;
        let rows = self
            .evaluate(&points, Some(ctx), ctx.session_id.as_deref())
            .await;
        let mut messages = Vec::new();
        for row in rows {
            if row.persist {
                self.append_row(ctx.session_id.as_deref(), &row).await;
            }
            messages.push(Self::agent_message(&row));
        }
        messages
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
    use crate::config::{DedupMode, InjectorPoint, Placement};
    use crate::registry::NativeInjector;

    struct FixedInjector {
        id: &'static str,
        point: InjectorPoint,
        placement: Placement,
        group: Option<&'static str>,
        text: &'static str,
    }

    #[async_trait]
    impl NativeInjector for FixedInjector {
        fn id(&self) -> &str {
            self.id
        }

        fn point(&self) -> InjectorPoint {
            self.point
        }

        fn placement(&self) -> Placement {
            self.placement
        }

        fn dedup(&self) -> DedupMode {
            DedupMode::None
        }

        fn group(&self) -> Option<&str> {
            self.group
        }

        async fn produce(&self, _ctx: &InjectorContext<'_>) -> Option<String> {
            Some(self.text.to_string())
        }
    }

    fn injector(
        id: &'static str,
        point: InjectorPoint,
        placement: Placement,
        group: Option<&'static str>,
        text: &'static str,
    ) -> ContextInjector {
        ContextInjector {
            id: id.into(),
            point,
            priority: 0,
            placement,
            dedup: DedupMode::None,
            group: group.map(str::to_string),
            source: InjectorSource::Native(Arc::new(FixedInjector {
                id,
                point,
                placement,
                group,
                text,
            })),
        }
    }

    fn middleware(injectors: Vec<ContextInjector>) -> ContextInjectMiddleware {
        let registry = Arc::new(ContextInjectorRegistry::new());
        for injector in injectors {
            registry.register(injector).expect("register");
        }
        ContextInjectMiddleware::new(registry, None)
    }

    #[test]
    fn before_points_only_step_start() {
        let ctx = BeforeStepCtx {
            step_index: 0,
            has_new_user_messages: true,
            ..Default::default()
        };
        assert_eq!(
            ContextInjectMiddleware::before_points(&ctx),
            vec![InjectorPoint::StepStart]
        );
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

    #[tokio::test]
    async fn submission_points_first_of_session_once() {
        let middleware = middleware(Vec::new());
        let ctx = BeforeStepCtx {
            session_id: Some("s1".into()),
            ..Default::default()
        };
        assert_eq!(
            middleware.submission_points(&ctx).await,
            vec![
                InjectorPoint::SessionStart,
                InjectorPoint::RunStart,
                InjectorPoint::TurnPre
            ]
        );
        assert_eq!(
            middleware.submission_points(&ctx).await,
            vec![InjectorPoint::RunStart, InjectorPoint::TurnPre]
        );
    }

    fn message_texts(messages: Vec<AgentMessage>) -> Vec<String> {
        messages
            .into_iter()
            .map(|message| match message {
                AgentMessage::Env(EnvMessage::CustomMessage { content, .. }) => {
                    content.as_str().unwrap_or_default().to_string()
                }
                other => panic!("unexpected message: {other:?}"),
            })
            .collect()
    }

    async fn session_store_with_transcript(session_id: &str) -> Arc<dyn XySessionStore> {
        use xylitol_plugin_sdk::session::{
            EntryBase, MessageEntry, SessionEntry, fixture_message_json,
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let store: Arc<dyn XySessionStore> = Arc::new(
            xylitol_official_plugin_session::SessionManager::new(dir.keep()),
        );
        store.create(session_id, None, None).await.expect("create");
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            message: fixture_message_json("user", "hello"),
        });
        store
            .append_session_entry(session_id, &entry)
            .await
            .expect("append transcript");
        store
    }

    #[tokio::test]
    async fn fake_example_injector_joins_dispatch() {
        let registry = Arc::new(ContextInjectorRegistry::new());
        assert!(registry.is_empty());
        registry
            .register(injector(
                "example",
                InjectorPoint::TurnPre,
                Placement::Ephemeral,
                None,
                "example-reminder",
            ))
            .expect("register example injector");
        assert!(!registry.is_empty(), "registration must join the registry");
        let middleware = ContextInjectMiddleware::new(registry.clone(), None);
        let ctx = BeforeStepCtx {
            session_id: Some("example".into()),
            ..Default::default()
        };
        let texts = message_texts(middleware.on_turn_submission(&ctx).await);
        assert_eq!(texts, vec!["example-reminder".to_string()]);
        assert_eq!(
            registry
                .snapshot()
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["example"]
        );
    }

    #[tokio::test]
    async fn fresh_session_skips_shutdown_backfill() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store: Arc<dyn XySessionStore> = Arc::new(
            xylitol_official_plugin_session::SessionManager::new(dir.keep()),
        );
        store.create("fresh", None, None).await.expect("create");
        let registry = Arc::new(ContextInjectorRegistry::new());
        registry
            .register(injector(
                "shutdown",
                InjectorPoint::SessionShutdown,
                Placement::Ephemeral,
                None,
                "bye",
            ))
            .expect("register");
        let middleware = ContextInjectMiddleware::new(registry, Some(store));
        let ctx = BeforeStepCtx {
            session_id: Some("fresh".into()),
            ..Default::default()
        };
        let texts = message_texts(middleware.on_turn_submission(&ctx).await);
        assert!(
            texts.is_empty(),
            "fresh session must not backfill session_shutdown: {texts:?}"
        );
    }

    #[tokio::test]
    async fn reopened_session_backfills_shutdown_once() {
        let store = session_store_with_transcript("resumed").await;
        let registry = Arc::new(ContextInjectorRegistry::new());
        for (id, point, text) in [
            ("shutdown", InjectorPoint::SessionShutdown, "bye"),
            ("start", InjectorPoint::SessionStart, "hello-again"),
        ] {
            registry
                .register(injector(id, point, Placement::Ephemeral, None, text))
                .expect("register");
        }
        let middleware = ContextInjectMiddleware::new(registry, Some(store));
        let ctx = BeforeStepCtx {
            session_id: Some("resumed".into()),
            ..Default::default()
        };
        let first = message_texts(middleware.on_turn_submission(&ctx).await);
        assert_eq!(first, vec!["bye".to_string(), "hello-again".to_string()]);
        let second = message_texts(middleware.on_turn_submission(&ctx).await);
        assert!(
            second.is_empty(),
            "session boundary points fire only on the first turn: {second:?}"
        );
    }

    #[tokio::test]
    async fn turn_submission_aggregates_group_and_orders_rows() {
        let middleware = middleware(vec![
            injector(
                "skills",
                InjectorPoint::TurnPre,
                Placement::HistoryRow,
                Some("system-reminder"),
                "[skills] a",
            ),
            injector(
                "tools",
                InjectorPoint::TurnPre,
                Placement::Ephemeral,
                Some("system-reminder"),
                "[tools] b",
            ),
            injector(
                "plain",
                InjectorPoint::TurnPre,
                Placement::Ephemeral,
                None,
                "hi",
            ),
            injector(
                "step",
                InjectorPoint::StepStart,
                Placement::Ephemeral,
                None,
                "step-only",
            ),
        ]);
        let ctx = BeforeStepCtx {
            session_id: Some("s1".into()),
            ..Default::default()
        };
        let messages = middleware.on_turn_submission(&ctx).await;
        let texts: Vec<String> = messages
            .iter()
            .map(|message| match message {
                AgentMessage::Env(EnvMessage::CustomMessage { content, .. }) => {
                    content.as_str().unwrap_or_default().to_string()
                }
                other => panic!("unexpected message: {other:?}"),
            })
            .collect();
        assert_eq!(
            texts,
            vec![
                "hi".to_string(),
                "<system-reminder>\n[skills] a\n[tools] b\n</system-reminder>".to_string()
            ]
        );
        let again = middleware.on_turn_submission(&ctx).await;
        assert_eq!(again.len(), 2, "turn boundary points fire every submission");
    }
}
