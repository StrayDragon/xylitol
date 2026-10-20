//! Composition-root wrappers: official Host plugins as rutis fibers.
//!
//! Wrappers live here so `xylitol-rutis` does not list the seven plugins, and
//! the plugins themselves do not `use rutis`. Empty MCP / hooks skip `apply`.

use std::sync::Arc;

use crate::wiring::composition::{BuildAgentOptions, official_compaction_factory};
use crate::wiring::driver::XyDriverError;
use crate::wiring::infra::event::EventBus;
use crate::wiring::infra::permission;
use xylitol_official_plugin_hooks::HookDispatcher;
use xylitol_plugin_sdk::{
    XyCompactionFactory, XyEventSink, XyHookBus, XyModelBuilder, XySessionStore, XyTool,
};
use xylitol_runtime::agent::tools::ToolSet;
use xylitol_rutis::{BoxFuture, CordisError, Ctx, Effect, Plugin, TypeKey, apply, process_root};

struct SessionFiber {
    store: Arc<dyn XySessionStore>,
}

impl Plugin for SessionFiber {
    fn name(&self) -> &str {
        "xylitol-official-plugin-session"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide_as::<dyn XySessionStore>(
                TypeKey::of::<dyn XySessionStore>(),
                Arc::clone(&self.store),
            )?;
            Ok(Effect::Done)
        })
    }
}

struct CompactionFiber {
    factory: XyCompactionFactory,
}

impl Plugin for CompactionFiber {
    fn name(&self) -> &str {
        "xylitol-official-plugin-compaction"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(ProvidedCompaction(Arc::clone(&self.factory)))?;
            Ok(Effect::Done)
        })
    }
}

struct ToolsFiber;

impl Plugin for ToolsFiber {
    fn name(&self) -> &str {
        "xylitol-official-plugin-tools"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(ProvidedTools(xylitol_official_plugin_tools::default_tools()))?;
            Ok(Effect::Done)
        })
    }
}

struct OpenaiFiber {
    builder: XyModelBuilder,
}

impl Plugin for OpenaiFiber {
    fn name(&self) -> &str {
        "xylitol-official-plugin-provider-openai"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(ProvidedModelBuilder(Arc::clone(&self.builder)))?;
            Ok(Effect::Done)
        })
    }
}

struct AnthropicFiber;

impl Plugin for AnthropicFiber {
    fn name(&self) -> &str {
        "xylitol-official-plugin-provider-anthropic"
    }

    fn apply<'a>(&'a self, _ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move { Ok(Effect::Done) })
    }
}

struct HooksFiber {
    bus: Arc<dyn XyHookBus>,
}

impl Plugin for HooksFiber {
    fn name(&self) -> &str {
        "xylitol-official-plugin-hooks"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide_as::<dyn XyHookBus>(TypeKey::of::<dyn XyHookBus>(), Arc::clone(&self.bus))?;
            Ok(Effect::Done)
        })
    }
}

#[derive(Clone)]
struct ProvidedCompaction(XyCompactionFactory);
#[derive(Clone)]
struct ProvidedTools(Vec<Arc<dyn XyTool>>);
#[derive(Clone)]
struct ProvidedModelBuilder(XyModelBuilder);

fn apply_err(err: Arc<CordisError>) -> XyDriverError {
    format!("assembly apply: {err}").into()
}

async fn assemble_official(
    options: BuildAgentOptions,
    store: Arc<dyn XySessionStore>,
) -> Result<xylitol_runtime::agent::RuntimeBaseline, XyDriverError> {
    let ctx = process_root().map_err(|e| format!("assembly root: {e}"))?;

    let sink: Arc<dyn XyEventSink> = options
        .event_sink
        .unwrap_or_else(|| Arc::new(EventBus::new()));

    let hook_dispatcher = Arc::new(HookDispatcher::new(&options.hooks_config));
    let hooks_for_provider = if hook_dispatcher.is_empty() {
        None
    } else {
        Some(hook_dispatcher.clone())
    };
    let model_builder: XyModelBuilder = {
        let hooks = hooks_for_provider;
        Arc::new(move |config| {
            crate::wiring::infra::provider::factory::build_provider_with_hooks(
                config,
                hooks.clone(),
            )
        })
    };

    apply(&ctx, SessionFiber { store })
        .await
        .map_err(apply_err)?;
    apply(
        &ctx,
        CompactionFiber {
            factory: official_compaction_factory(),
        },
    )
    .await
    .map_err(apply_err)?;
    apply(&ctx, ToolsFiber).await.map_err(apply_err)?;
    apply(
        &ctx,
        OpenaiFiber {
            builder: model_builder,
        },
    )
    .await
    .map_err(apply_err)?;
    apply(&ctx, AnthropicFiber).await.map_err(apply_err)?;
    if !hook_dispatcher.is_empty() {
        apply(
            &ctx,
            HooksFiber {
                bus: hook_dispatcher.clone(),
            },
        )
        .await
        .map_err(apply_err)?;
    }

    let store = ctx
        .get_as::<dyn XySessionStore>(TypeKey::of::<dyn XySessionStore>())
        .ok_or_else(|| "assembly missing session store".to_string())?;
    let compaction = ctx
        .get::<ProvidedCompaction>()
        .ok_or_else(|| "assembly missing compaction".to_string())?;
    let compaction = compaction.0.clone();
    let tools = ctx
        .get::<ProvidedTools>()
        .ok_or_else(|| "assembly missing tools".to_string())?;
    let tools = tools.0.clone();
    let model_builder = ctx
        .get::<ProvidedModelBuilder>()
        .ok_or_else(|| "assembly missing model builder".to_string())?;
    let model_builder = model_builder.0.clone();
    let hook_bus = ctx.get_as::<dyn XyHookBus>(TypeKey::of::<dyn XyHookBus>());

    let permission = options
        .permission
        .unwrap_or_else(permission::allow_all_permission);

    let mut builder = xylitol_runtime::agent::AgentBuilder::new(
        options.model_registry,
        model_builder,
        store,
        sink,
        permission,
    )
    .tools(ToolSet::from_iter(tools))
    .context_files(options.context_files)
    .append_system_prompt(options.append_system_prompt)
    .skills(options.skills)
    .compaction_settings(options.compaction_settings)
    .compaction_factory(compaction)
    .cwd(options.cwd)
    .steering_mode(options.steering_mode)
    .follow_up_mode(options.follow_up_mode)
    .hook_bus(hook_bus)
    .batch_mode(options.batch_mode);

    if let Some(sp) = options.system_prompt {
        builder = builder.system_prompt(sp);
    }

    let baseline = builder.build_baseline();
    ctx.shutdown()
        .await
        .map_err(|e| format!("assembly shutdown: {e}"))?;
    Ok(baseline)
}

/// Run official-plugin `apply` on a dedicated current-thread runtime so
/// `build_baseline` stays callable from sync tests and from `#[tokio::test]`.
pub(crate) fn assemble_blocking(
    options: BuildAgentOptions,
    store: Arc<dyn XySessionStore>,
) -> Result<xylitol_runtime::agent::RuntimeBaseline, XyDriverError> {
    std::thread::Builder::new()
        .name("xylitol-assemble".into())
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("assembly runtime: {e}"))?
                .block_on(assemble_official(options, store))
        })
        .map_err(|e| format!("assembly thread: {e}"))?
        .join()
        .unwrap_or_else(|_| Err("assembly thread panicked".into()))
}
