use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session::{PlanMode, PlanModeKind, latest_plan_mode};
use xylitol_plugin_sdk::{
    BeforeModelCtx, LoopMiddleware, LoopStage, LoopStep, RuntimeSessionStore, XySessionStore,
};

const PLAN_MODE_STAGES: &[LoopStage] = &[LoopStage::BeforeModel];
const PLAN_MODE_CONTRACT: &str = "Plan mode: research and read first. Do not edit files yet. \
Write the plan with plan_write, then call plan_submit for approval. \
After approval the mode switches to execute and the todo list is seeded.";

pub struct PlanModeMiddleware {
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
    mode: Mutex<PlanMode>,
}

impl PlanModeMiddleware {
    pub fn new(store: Option<Arc<dyn XySessionStore>>, ctx: Option<Ctx>) -> Self {
        Self {
            store,
            ctx,
            mode: Mutex::new(PlanMode::execute()),
        }
    }

    fn resolve_store(&self) -> Option<Arc<dyn XySessionStore>> {
        self.ctx
            .as_ref()
            .and_then(|ctx| ctx.get::<RuntimeSessionStore>())
            .map(|store| store.0.clone())
            .or_else(|| {
                self.ctx
                    .as_ref()
                    .and_then(|ctx| ctx.get::<dyn XySessionStore>())
            })
            .or_else(|| self.store.clone())
    }

    async fn load_mode(&self, session_id: Option<&str>) {
        let (Some(store), Some(session_id)) = (self.resolve_store(), session_id) else {
            return;
        };
        let Ok(branch) = store.load_leaf_branch(session_id).await else {
            return;
        };
        let mode = latest_plan_mode(&branch);
        *self.mode.lock().unwrap_or_else(|e| e.into_inner()) = mode;
    }
}

#[async_trait]
impl LoopMiddleware for PlanModeMiddleware {
    fn id(&self) -> &str {
        "plan-mode"
    }

    fn order(&self) -> i64 {
        185
    }

    fn stages(&self) -> &'static [LoopStage] {
        PLAN_MODE_STAGES
    }

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
        self.load_mode(ctx.session_id.as_deref()).await;
        let mode = self.mode.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if mode.mode == PlanModeKind::Plan {
            ctx.messages
                .push(AgentMessage::Llm(XyLlmMessage::user(PLAN_MODE_CONTRACT)));
        }
        LoopStep::continued(ctx)
    }
}

pub fn plan_mode_middleware(
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
) -> Arc<dyn LoopMiddleware> {
    Arc::new(PlanModeMiddleware::new(store, ctx))
}
