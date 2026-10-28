use std::sync::{Arc, Mutex, Weak};

use async_trait::async_trait;
use serde_json::Value;

use crate::error::XyError;
use crate::generate::XyGenerateOptions;
use crate::message::{AgentMessage, AgentPart};
use crate::model::XyToolSchema;
use crate::session::TodoList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopStage {
    BeforeTurn,
    BeforeLlmRequest,
    AfterLlmResponse,
    BeforeTool,
    AfterTool,
    DecideContinue,
    AfterTurn,
}

impl LoopStage {
    pub const ALL: [LoopStage; 7] = [
        LoopStage::BeforeTurn,
        LoopStage::BeforeLlmRequest,
        LoopStage::AfterLlmResponse,
        LoopStage::BeforeTool,
        LoopStage::AfterTool,
        LoopStage::DecideContinue,
        LoopStage::AfterTurn,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            LoopStage::BeforeTurn => "before_turn",
            LoopStage::BeforeLlmRequest => "before_llm_request",
            LoopStage::AfterLlmResponse => "after_llm_response",
            LoopStage::BeforeTool => "before_tool",
            LoopStage::AfterTool => "after_tool",
            LoopStage::DecideContinue => "decide_continue",
            LoopStage::AfterTurn => "after_turn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopControl {
    Continue,
    Stop,
    Compact,
    Handoff,
}

impl LoopControl {
    pub fn as_str(self) -> &'static str {
        match self {
            LoopControl::Continue => "continue",
            LoopControl::Stop => "stop",
            LoopControl::Compact => "compact",
            LoopControl::Handoff => "handoff",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoopStep<C> {
    pub ctx: C,
    pub control: LoopControl,
}

impl<C> LoopStep<C> {
    pub fn continued(ctx: C) -> Self {
        Self {
            ctx,
            control: LoopControl::Continue,
        }
    }

    pub fn stopped(ctx: C) -> Self {
        Self {
            ctx,
            control: LoopControl::Stop,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BeforeTurnCtx {
    pub turn_index: u32,
    pub system_prompt: Option<String>,
    pub system_fragments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BeforeLlmRequestCtx {
    pub turn_index: u32,
    pub messages: Vec<AgentMessage>,
    pub tool_schemas: Vec<XyToolSchema>,
    pub options: XyGenerateOptions,
    pub todo: Option<TodoList>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AfterLlmResponseCtx {
    pub turn_index: u32,
    pub assistant: AgentMessage,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BeforeToolCtx {
    pub turn_index: u32,
    pub tool_call_id: String,
    pub tool_name: String,
    pub args: Value,
}

#[derive(Debug, Clone)]
pub struct AfterToolCtx {
    pub turn_index: u32,
    pub tool_call_id: String,
    pub tool_name: String,
    pub parts: Vec<AgentPart>,
    pub details: Option<Value>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactRequest {
    pub instructions: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DecideContinueCtx {
    pub turn_index: u32,
    pub settle_count: u32,
    pub has_new_user_messages: bool,
    pub assistant: Option<AgentMessage>,
    pub history_len: usize,
    pub compact: Option<CompactRequest>,
}

#[derive(Debug, Clone)]
pub struct AfterTurnCtx {
    pub turn_index: u32,
    pub continued_tools: bool,
    pub history_len: usize,
    pub session_id: Option<String>,
}

#[async_trait]
pub trait LoopMiddleware: Send + Sync {
    fn id(&self) -> &str;

    fn order(&self) -> i64 {
        0
    }

    fn stages(&self) -> &'static [LoopStage];

    async fn before_turn(&self, ctx: BeforeTurnCtx) -> LoopStep<BeforeTurnCtx> {
        LoopStep::continued(ctx)
    }

    async fn before_llm_request(&self, ctx: BeforeLlmRequestCtx) -> LoopStep<BeforeLlmRequestCtx> {
        LoopStep::continued(ctx)
    }

    async fn after_llm_response(&self, ctx: AfterLlmResponseCtx) -> LoopStep<AfterLlmResponseCtx> {
        LoopStep::continued(ctx)
    }

    async fn before_tool(&self, ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        LoopStep::continued(ctx)
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        LoopStep::continued(ctx)
    }

    async fn after_turn(&self, ctx: AfterTurnCtx) -> LoopStep<AfterTurnCtx> {
        LoopStep::continued(ctx)
    }
}

struct Entry {
    id: String,
    order: i64,
    seq: u64,
    stages: &'static [LoopStage],
    middleware: Arc<dyn LoopMiddleware>,
}

#[derive(Default)]
struct RegistryInner {
    next_seq: u64,
    entries: Vec<Entry>,
}

pub struct LoopMiddlewareRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

impl Default for LoopMiddlewareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopMiddlewareRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryInner::default())),
        }
    }

    pub fn register(
        &self,
        middleware: Arc<dyn LoopMiddleware>,
    ) -> Result<LoopMiddlewareHandle, XyError> {
        let id = middleware.id().to_string();
        let order = middleware.order();
        let stages = middleware.stages();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.entries.iter().any(|entry| entry.id == id) {
            return Err(XyError::Config(format!(
                "loop middleware id already registered: {id}"
            )));
        }
        let seq = inner.next_seq;
        inner.next_seq += 1;
        inner.entries.push(Entry {
            id: id.clone(),
            order,
            seq,
            stages,
            middleware,
        });
        drop(inner);
        Ok(LoopMiddlewareHandle {
            inner: Arc::downgrade(&self.inner),
            id,
        })
    }

    pub fn unregister(&self, id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let before = inner.entries.len();
        inner.entries.retain(|entry| entry.id != id);
        inner.entries.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .is_empty()
    }

    pub fn ids(&self) -> Vec<String> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<&Entry> = inner.entries.iter().collect();
        entries.sort_by(|a, b| a.order.cmp(&b.order).then(a.seq.cmp(&b.seq)));
        entries.into_iter().map(|entry| entry.id.clone()).collect()
    }

    pub fn snapshot(&self) -> LoopChain {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<&Entry> = inner.entries.iter().collect();
        entries.sort_by(|a, b| a.order.cmp(&b.order).then(a.seq.cmp(&b.seq)));
        let mut chain = LoopChain::default();
        for entry in entries {
            for stage in entry.stages {
                chain.bucket_mut(*stage).push(Arc::clone(&entry.middleware));
            }
        }
        chain
    }

    pub fn snapshot_scoped(&self, scope: LoopScope) -> LoopChain {
        let mut chain = self.snapshot();
        chain.scope = scope;
        chain
    }

    pub fn derive(&self) -> LoopMiddlewareRegistry {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<&Entry> = inner.entries.iter().collect();
        entries.sort_by(|a, b| a.order.cmp(&b.order).then(a.seq.cmp(&b.seq)));
        let derived = RegistryInner {
            next_seq: inner.next_seq,
            entries: entries
                .into_iter()
                .map(|entry| Entry {
                    id: entry.id.clone(),
                    order: entry.order,
                    seq: entry.seq,
                    stages: entry.stages,
                    middleware: Arc::clone(&entry.middleware),
                })
                .collect(),
        };
        LoopMiddlewareRegistry {
            inner: Arc::new(Mutex::new(derived)),
        }
    }
}

#[derive(Clone, Default)]
pub struct LoopChain {
    pub before_turn: Vec<Arc<dyn LoopMiddleware>>,
    pub before_llm_request: Vec<Arc<dyn LoopMiddleware>>,
    pub after_llm_response: Vec<Arc<dyn LoopMiddleware>>,
    pub before_tool: Vec<Arc<dyn LoopMiddleware>>,
    pub after_tool: Vec<Arc<dyn LoopMiddleware>>,
    pub decide_continue: Vec<Arc<dyn LoopMiddleware>>,
    pub after_turn: Vec<Arc<dyn LoopMiddleware>>,
    pub scope: LoopScope,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoopScope {
    #[default]
    Root,
    Session,
}

impl LoopScope {
    pub fn as_str(self) -> &'static str {
        match self {
            LoopScope::Root => "root",
            LoopScope::Session => "session",
        }
    }
}

impl LoopChain {
    fn bucket_mut(&mut self, stage: LoopStage) -> &mut Vec<Arc<dyn LoopMiddleware>> {
        match stage {
            LoopStage::BeforeTurn => &mut self.before_turn,
            LoopStage::BeforeLlmRequest => &mut self.before_llm_request,
            LoopStage::AfterLlmResponse => &mut self.after_llm_response,
            LoopStage::BeforeTool => &mut self.before_tool,
            LoopStage::AfterTool => &mut self.after_tool,
            LoopStage::DecideContinue => &mut self.decide_continue,
            LoopStage::AfterTurn => &mut self.after_turn,
        }
    }

    pub fn stage(&self, stage: LoopStage) -> &[Arc<dyn LoopMiddleware>] {
        match stage {
            LoopStage::BeforeTurn => &self.before_turn,
            LoopStage::BeforeLlmRequest => &self.before_llm_request,
            LoopStage::AfterLlmResponse => &self.after_llm_response,
            LoopStage::BeforeTool => &self.before_tool,
            LoopStage::AfterTool => &self.after_tool,
            LoopStage::DecideContinue => &self.decide_continue,
            LoopStage::AfterTurn => &self.after_turn,
        }
    }

    pub fn is_empty(&self) -> bool {
        LoopStage::ALL
            .iter()
            .all(|stage| self.stage(*stage).is_empty())
    }
}

#[derive(Debug)]
pub struct LoopMiddlewareHandle {
    inner: Weak<Mutex<RegistryInner>>,
    id: String,
}

impl LoopMiddlewareHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn unregister(self) -> bool {
        let Some(inner) = self.inner.upgrade() else {
            return false;
        };
        let mut guard = inner.lock().unwrap_or_else(|e| e.into_inner());
        let before = guard.entries.len();
        guard.entries.retain(|entry| entry.id != self.id);
        guard.entries.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Probe {
        id: &'static str,
        order: i64,
        stages: &'static [LoopStage],
    }

    #[async_trait]
    impl LoopMiddleware for Probe {
        fn id(&self) -> &str {
            self.id
        }

        fn order(&self) -> i64 {
            self.order
        }

        fn stages(&self) -> &'static [LoopStage] {
            self.stages
        }
    }

    fn probe(
        id: &'static str,
        order: i64,
        stages: &'static [LoopStage],
    ) -> Arc<dyn LoopMiddleware> {
        Arc::new(Probe { id, order, stages })
    }

    #[test]
    fn snapshot_orders_by_order_then_registration() {
        let reg = LoopMiddlewareRegistry::new();
        let b = reg
            .register(probe("b", 10, &[LoopStage::BeforeLlmRequest]))
            .expect("b");
        let a = reg
            .register(probe("a", 5, &[LoopStage::BeforeLlmRequest]))
            .expect("a");
        let c = reg
            .register(probe("c", 10, &[LoopStage::BeforeLlmRequest]))
            .expect("c");
        let chain = reg.snapshot();
        let ids: Vec<String> = chain
            .before_llm_request
            .iter()
            .map(|mw| mw.id().to_string())
            .collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
        drop((a, b, c));
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let reg = LoopMiddlewareRegistry::new();
        let first = reg
            .register(probe("dup", 0, &[LoopStage::AfterTurn]))
            .expect("first");
        let err = match reg.register(probe("dup", 1, &[LoopStage::AfterTurn])) {
            Ok(_) => panic!("duplicate id must be rejected"),
            Err(err) => err,
        };
        assert!(matches!(err, XyError::Config(_)));
        drop(first);
    }

    #[test]
    fn handle_unregister_removes_entry() {
        let reg = LoopMiddlewareRegistry::new();
        let handle = reg
            .register(probe("temp", 0, &[LoopStage::BeforeTool]))
            .expect("register");
        assert_eq!(handle.id(), "temp");
        assert!(!reg.is_empty());
        assert!(handle.unregister());
        assert!(reg.is_empty());
        assert!(reg.snapshot().before_tool.is_empty());
    }

    #[test]
    fn explicit_unregister_removes_entry() {
        let reg = LoopMiddlewareRegistry::new();
        let handle = reg
            .register(probe("explicit", 0, &[LoopStage::AfterTool]))
            .expect("register");
        assert!(reg.unregister("explicit"));
        assert!(!reg.unregister("explicit"));
        assert!(reg.is_empty());
        drop(handle);
    }

    #[test]
    fn stages_bucket_per_declaration() {
        let reg = LoopMiddlewareRegistry::new();
        let a = reg
            .register(probe(
                "a",
                0,
                &[LoopStage::BeforeTurn, LoopStage::DecideContinue],
            ))
            .expect("a");
        let chain = reg.snapshot();
        drop(a);
        assert_eq!(chain.before_turn.len(), 1);
        assert_eq!(chain.decide_continue.len(), 1);
        assert!(chain.before_llm_request.is_empty());
        assert!(!chain.is_empty());
        assert!(LoopChain::default().is_empty());
    }

    #[tokio::test]
    async fn stage_short_circuits_on_stop() {
        struct StopAtTool;

        #[async_trait]
        impl LoopMiddleware for StopAtTool {
            fn id(&self) -> &str {
                "stop-at-tool"
            }

            fn stages(&self) -> &'static [LoopStage] {
                &[LoopStage::BeforeTool]
            }

            async fn before_tool(&self, ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
                LoopStep::stopped(ctx)
            }
        }

        let ctx = BeforeToolCtx {
            turn_index: 0,
            tool_call_id: "1".into(),
            tool_name: "bash".into(),
            args: Value::Null,
        };
        let step = StopAtTool.before_tool(ctx).await;
        assert_eq!(step.control, LoopControl::Stop);
        assert_eq!(LoopControl::Handoff.as_str(), "handoff");
        assert_eq!(LoopStage::DecideContinue.as_str(), "decide_continue");
        assert_eq!(LoopStage::ALL.len(), 7);
    }

    #[test]
    fn after_tool_carries_structured_payload() {
        let ctx = AfterToolCtx {
            turn_index: 2,
            tool_call_id: "call-9".into(),
            tool_name: "read".into(),
            parts: vec![AgentPart::Image(crate::dto::ImageContent {
                url: None,
                data: Some("AAA".into()),
                media_type: "image/png".into(),
            })],
            details: Some(serde_json::json!({ "success": true })),
            is_error: false,
        };
        assert!(matches!(ctx.parts[0], AgentPart::Image(_)));
        assert_eq!(
            ctx.details.as_ref().and_then(|d| d.get("success")),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[test]
    fn compact_request_defaults_to_none() {
        let ctx = DecideContinueCtx {
            turn_index: 0,
            settle_count: 1,
            has_new_user_messages: false,
            assistant: None,
            history_len: 3,
            compact: None,
        };
        assert!(ctx.compact.is_none());
        let filled = CompactRequest {
            instructions: Some("keep recent tool results".into()),
            reason: Some("loop_middleware:focus".into()),
        };
        assert_eq!(filled.reason.as_deref(), Some("loop_middleware:focus"));
    }

    #[tokio::test]
    async fn isolate_scopes_same_type_registries_per_session() {
        use crate::rt::{CtxHost, TypeKey};
        let root = crate::rt::Ctx::root().expect("root ctx");
        let key = TypeKey::of::<LoopMiddlewareRegistry>();
        let session_a = root.isolate(key.clone(), "session:a");
        let session_b = root.isolate(key.clone(), "session:b");

        let reg_a = Arc::new(LoopMiddlewareRegistry::new());
        let reg_b = Arc::new(LoopMiddlewareRegistry::new());
        session_a
            .provide_as::<LoopMiddlewareRegistry>(reg_a.clone())
            .expect("provide a");
        session_b
            .provide_as::<LoopMiddlewareRegistry>(reg_b.clone())
            .expect("provide b");

        reg_a
            .register(probe("only-a", 0, &[LoopStage::BeforeTool]))
            .expect("register a");
        reg_b
            .register(probe("only-b", 0, &[LoopStage::BeforeTurn]))
            .expect("register b");

        let seen_a = session_a
            .get::<LoopMiddlewareRegistry>()
            .expect("a visible");
        let seen_b = session_b
            .get::<LoopMiddlewareRegistry>()
            .expect("b visible");
        assert_eq!(seen_a.ids(), vec!["only-a".to_string()]);
        assert_eq!(seen_b.ids(), vec!["only-b".to_string()]);
        assert!(
            root.get::<LoopMiddlewareRegistry>().is_none(),
            "default scope must stay empty so sessions cannot leak into root"
        );
    }

    #[tokio::test]
    async fn isolate_same_label_shares_one_registry() {
        use crate::rt::{CtxHost, TypeKey};
        let root = crate::rt::Ctx::root().expect("root ctx");
        let key = TypeKey::of::<LoopMiddlewareRegistry>();
        let first = root.isolate(key.clone(), "session:shared");
        let registry = Arc::new(LoopMiddlewareRegistry::new());
        first
            .provide_as::<LoopMiddlewareRegistry>(registry.clone())
            .expect("provide shared");

        let second = root.isolate(key.clone(), "session:shared");
        let seen = second
            .get::<LoopMiddlewareRegistry>()
            .expect("shared visible");
        assert!(
            Arc::ptr_eq(&seen, &registry),
            "same isolate label resolves the same registry (reconnect must not fork the chain)"
        );
    }

    #[tokio::test]
    async fn isolate_unload_withdraws_binding_and_keeps_root_clean() {
        use crate::rt::{CtxHost, TypeKey};
        let root = crate::rt::Ctx::root().expect("root ctx");
        let key = TypeKey::of::<LoopMiddlewareRegistry>();
        let scope = root.isolate(key, "session:gone");
        let registry = Arc::new(LoopMiddlewareRegistry::new());
        registry
            .register(probe("ephemeral", 0, &[LoopStage::BeforeTool]))
            .expect("register");
        let disposer = scope
            .provide_as::<LoopMiddlewareRegistry>(registry.clone())
            .expect("provide");
        assert!(scope.get::<LoopMiddlewareRegistry>().is_some());
        disposer.dispose().await.expect("dispose");
        assert!(
            scope.get::<LoopMiddlewareRegistry>().is_none(),
            "withdrawing the binding must hide the scope registry"
        );
        assert!(
            root.get::<LoopMiddlewareRegistry>().is_none(),
            "the default scope must never inherit a session registry"
        );
    }

    #[test]
    fn derive_freezes_parent_snapshot() {
        let parent = LoopMiddlewareRegistry::new();
        parent
            .register(probe("a", 10, &[LoopStage::BeforeTool]))
            .expect("a");
        let child = parent.derive();
        parent
            .register(probe("late", 5, &[LoopStage::BeforeTool]))
            .expect("late");
        assert_eq!(child.ids(), vec!["a".to_string()]);
        assert_eq!(parent.ids(), vec!["late".to_string(), "a".to_string()]);
    }

    #[test]
    fn derive_child_registrations_stay_local() {
        let parent = LoopMiddlewareRegistry::new();
        parent
            .register(probe("base", 0, &[LoopStage::BeforeTool]))
            .expect("base");
        let child = parent.derive();
        child
            .register(probe("child-only", 5, &[LoopStage::BeforeTool]))
            .expect("child-only");
        assert_eq!(
            child.ids(),
            vec!["base".to_string(), "child-only".to_string()]
        );
        assert_eq!(parent.ids(), vec!["base".to_string()]);
        assert!(child.unregister("child-only"));
        assert_eq!(child.ids(), vec!["base".to_string()]);
        assert_eq!(parent.ids(), vec!["base".to_string()]);
    }

    #[test]
    fn derive_keeps_order_then_registration_sequence() {
        let parent = LoopMiddlewareRegistry::new();
        parent
            .register(probe("z-first", 10, &[LoopStage::BeforeTool]))
            .expect("z-first");
        parent
            .register(probe("a-second", 10, &[LoopStage::BeforeTool]))
            .expect("a-second");
        let child = parent.derive();
        child
            .register(probe("appended", 10, &[LoopStage::BeforeTool]))
            .expect("appended");
        assert_eq!(
            child.ids(),
            vec![
                "z-first".to_string(),
                "a-second".to_string(),
                "appended".to_string()
            ],
            "same-order entries keep parent registration order, new ones append after"
        );
    }
}
