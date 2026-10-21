use std::sync::Arc;

use xylitol_plugin_sdk::rt::{
    BoxFuture, Call, CordisError, Ctx, Effect, EventKey, Listener, Plugin, PluginFactory,
};
use xylitol_plugin_sdk::{
    AgentPreStep, HttpBeforeRequest, ToolPostExecute, ToolPreExecute, XyHookBus, XyHookOutcome,
    hook_bus_key,
};

use crate::config::HooksConfig;
use crate::dispatcher::HookDispatcher;

pub struct HooksFactory;

impl PluginFactory<HooksConfig> for HooksFactory {
    fn name(&self) -> &str {
        "hooks"
    }

    fn build(&self, config: &HooksConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(HooksPlugin {
            config: config.clone(),
        }))
    }
}

struct HooksPlugin {
    config: HooksConfig,
}

impl Plugin for HooksPlugin {
    fn name(&self) -> &str {
        "hooks"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            let dispatcher = Arc::new(HookDispatcher::new(&self.config));
            if dispatcher.is_empty() {
                return Ok(Effect::Done);
            }
            let bus: Arc<dyn XyHookBus> = dispatcher.clone();
            ctx.provide_as(hook_bus_key(), bus)?;
            ctx.events().on(
                ctx,
                &EventKey::<ToolPreExecute>::of(),
                PreTool {
                    dispatcher: dispatcher.clone(),
                },
            )?;
            ctx.events().on(
                ctx,
                &EventKey::<ToolPostExecute>::of(),
                PostTool {
                    dispatcher: dispatcher.clone(),
                },
            )?;
            ctx.events().on(
                ctx,
                &EventKey::<AgentPreStep>::of(),
                PreStep {
                    dispatcher: dispatcher.clone(),
                },
            )?;
            ctx.events().on(
                ctx,
                &EventKey::<HttpBeforeRequest>::of(),
                BeforeHttp { dispatcher },
            )?;
            Ok(Effect::Done)
        })
    }
}

struct PreTool {
    dispatcher: Arc<HookDispatcher>,
}

impl Listener<ToolPreExecute> for PreTool {
    fn call<'a>(
        &'a self,
        _: &'a Ctx,
        event: &'a ToolPreExecute,
        call: Call<'a, Option<String>>,
    ) -> BoxFuture<'a, Result<Option<Option<String>>, CordisError>> {
        Box::pin(async move {
            match XyHookBus::dispatch(
                self.dispatcher.as_ref(),
                "tool_call",
                "pre",
                event.args.clone(),
            )
            .await
            {
                XyHookOutcome::Blocked { reason } => Ok(Some(Some(reason))),
                _ => call.next().await,
            }
        })
    }
}

struct PostTool {
    dispatcher: Arc<HookDispatcher>,
}

impl Listener<ToolPostExecute> for PostTool {
    fn call<'a>(
        &'a self,
        _: &'a Ctx,
        event: &'a ToolPostExecute,
        call: Call<'a, String>,
    ) -> BoxFuture<'a, Result<Option<String>, CordisError>> {
        Box::pin(async move {
            let ctx = serde_json::json!({
                "tool": event.name,
                "ok": event.ok,
                "output": event.output,
            });
            let _ = XyHookBus::dispatch(self.dispatcher.as_ref(), "tool_call", "post", ctx).await;
            call.next().await
        })
    }
}

struct PreStep {
    dispatcher: Arc<HookDispatcher>,
}

impl Listener<AgentPreStep> for PreStep {
    fn call<'a>(
        &'a self,
        _: &'a Ctx,
        event: &'a AgentPreStep,
        call: Call<'a, Result<(), String>>,
    ) -> BoxFuture<'a, Result<Option<Result<(), String>>, CordisError>> {
        Box::pin(async move {
            let ctx = serde_json::json!({ "step": event.step });
            match XyHookBus::dispatch(self.dispatcher.as_ref(), "agent_start", "pre", ctx).await {
                XyHookOutcome::Blocked { reason } => Ok(Some(Err(reason))),
                _ => call.next().await,
            }
        })
    }
}

struct BeforeHttp {
    dispatcher: Arc<HookDispatcher>,
}

impl Listener<HttpBeforeRequest> for BeforeHttp {
    fn call<'a>(
        &'a self,
        _: &'a Ctx,
        event: &'a HttpBeforeRequest,
        call: Call<'a, serde_json::Value>,
    ) -> BoxFuture<'a, Result<Option<serde_json::Value>, CordisError>> {
        Box::pin(async move {
            let ctx = serde_json::json!({
                "model": event.model,
                "body": event.body,
            });
            match XyHookBus::dispatch(
                self.dispatcher.as_ref(),
                "before_provider_request",
                "pre",
                ctx,
            )
            .await
            {
                XyHookOutcome::Modified { args } => Ok(Some(args)),
                XyHookOutcome::Blocked { reason } => Err(CordisError::Validation {
                    issues: vec![reason],
                }),
                XyHookOutcome::Allowed => call.next().await,
            }
        })
    }
}
