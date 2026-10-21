use std::sync::Arc;

use crate::composition::{BuildAgentOptions, official_compaction_factory};
use crate::driver::XyDriverError;
use crate::event::EventBus;
use xylitol_official_plugin_hooks::HookDispatcher;
use xylitol_plugin_sdk::{XyEventSink, XyHookBus, XyModelBuilder, XySessionStore};
use xylitol_runtime::agent::tools::ToolSet;

pub(crate) fn assemble_blocking(
    options: BuildAgentOptions,
    store: Arc<dyn XySessionStore>,
) -> Result<xylitol_runtime::agent::RuntimeBaseline, XyDriverError> {
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
            crate::provider::factory::build_provider_with_hooks(config, hooks.clone())
        })
    };

    let hook_bus: Option<Arc<dyn XyHookBus>> = if hook_dispatcher.is_empty() {
        None
    } else {
        Some(hook_dispatcher)
    };

    let permission = options
        .permission
        .unwrap_or_else(xylitol_runtime::permission::allow_all_permission);

    let mut builder = xylitol_runtime::agent::AgentBuilder::new(
        options.model_registry,
        model_builder,
        store,
        sink,
        permission,
    )
    .tools(ToolSet::from_iter(
        xylitol_official_plugin_tools::default_tools(),
    ))
    .context_files(options.context_files)
    .append_system_prompt(options.append_system_prompt)
    .skills(options.skills)
    .compaction_settings(options.compaction_settings)
    .compaction_factory(official_compaction_factory())
    .cwd(options.cwd)
    .steering_mode(options.steering_mode)
    .follow_up_mode(options.follow_up_mode)
    .hook_bus(hook_bus)
    .batch_mode(options.batch_mode);

    if let Some(sp) = options.system_prompt {
        builder = builder.system_prompt(sp);
    }

    Ok(builder.build_baseline())
}
