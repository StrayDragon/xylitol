use std::sync::Arc;

use xylitol_plugin_sdk::rt::{CallExt, Ctx, Error, ErrorKind, Plugin, Result};
use xylitol_plugin_sdk::{
    AgentPreStep, HttpBeforeRequest, ToolPostExecute, ToolPreExecute, XyHookBus, XyHookOutcome,
};

use crate::config::HooksConfig;
use crate::dispatcher::HookDispatcher;

pub struct HooksPlugin {
    config: HooksConfig,
}

impl HooksPlugin {
    pub fn new(config: &HooksConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for HooksPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "hooks"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let dispatcher = Arc::new(HookDispatcher::new(&self.config));
        if dispatcher.is_empty() {
            return Ok(());
        }
        let bus: Arc<dyn XyHookBus> = dispatcher.clone();
        ctx.provide_as::<dyn XyHookBus>(bus)?;
        ctx.on::<ToolPreExecute>({
            let dispatcher = dispatcher.clone();
            move |event, call| {
                let dispatcher = dispatcher.clone();
                async move {
                    match XyHookBus::dispatch(
                        dispatcher.as_ref(),
                        "tool_call",
                        "pre",
                        event.args.clone(),
                    )
                    .await
                    {
                        XyHookOutcome::Blocked { reason } => Ok(Some(Some(reason))),
                        _ => call.chain().await,
                    }
                }
            }
        })?;
        ctx.on::<ToolPostExecute>({
            let dispatcher = dispatcher.clone();
            move |event, call| {
                let dispatcher = dispatcher.clone();
                async move {
                    let ctx = serde_json::json!({
                        "tool": event.name,
                        "ok": event.ok,
                        "output": event.output,
                    });
                    let _ =
                        XyHookBus::dispatch(dispatcher.as_ref(), "tool_call", "post", ctx).await;
                    call.chain().await
                }
            }
        })?;
        ctx.on::<AgentPreStep>({
            let dispatcher = dispatcher.clone();
            move |event, call| {
                let dispatcher = dispatcher.clone();
                async move {
                    let ctx = serde_json::json!({ "step": event.step });
                    match XyHookBus::dispatch(dispatcher.as_ref(), "agent_start", "pre", ctx).await
                    {
                        XyHookOutcome::Blocked { reason } => Ok(Some(Err(reason))),
                        _ => call.chain().await,
                    }
                }
            }
        })?;
        ctx.on::<HttpBeforeRequest>({
            let dispatcher = dispatcher.clone();
            move |event, call| {
                let dispatcher = dispatcher.clone();
                async move {
                    let ctx = serde_json::json!({
                        "model": event.model,
                        "body": event.body,
                    });
                    match XyHookBus::dispatch(
                        dispatcher.as_ref(),
                        "before_provider_request",
                        "pre",
                        ctx,
                    )
                    .await
                    {
                        XyHookOutcome::Modified { args } => Ok(Some(args)),
                        XyHookOutcome::Blocked { reason } => {
                            Err(Error::from(ErrorKind::Validation {
                                issues: vec![reason],
                            }))
                        }
                        XyHookOutcome::Allowed => call.chain().await,
                    }
                }
            }
        })?;
        Ok(())
    }
}
