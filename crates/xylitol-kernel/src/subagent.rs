use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value, json};
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::{XyEvent, XyTool, XyToolCtx};

use crate::boot::BootOptions;
use crate::bootstrap::{BootstrapInput, bootstrap_with};
use crate::driver::XyDriver;
use xylitol_runtime::agent::tools::ToolSet;

type EnvFn = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub struct SubagentFactory {
    opts: BootOptions,
    tools: Vec<String>,
    get_env: EnvFn,
    cwd: Option<std::path::PathBuf>,
}

pub struct SubagentTool {
    factory: Arc<SubagentFactory>,
}

impl SubagentTool {
    pub fn new(opts: BootOptions, tools: Vec<String>) -> Self {
        Self::with_env(
            opts,
            tools,
            Arc::new(crate::bootstrap::process_get_env),
            None,
        )
    }

    pub fn with_env(
        opts: BootOptions,
        tools: Vec<String>,
        get_env: EnvFn,
        cwd: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            factory: Arc::new(SubagentFactory {
                opts,
                tools,
                get_env,
                cwd,
            }),
        }
    }
}

#[async_trait]
impl XyTool for SubagentTool {
    fn name(&self) -> &str {
        "subagent_readonly"
    }

    fn description(&self) -> &str {
        "Delegate a read-only exploration task to an isolated sub-agent. The sub-agent can read/search the workspace but cannot write, edit, or run shell commands."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Task or question for the read-only sub-agent."
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let prompt = args
            .get("prompt")
            .and_then(Value::as_str)
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| XyToolError::InvalidArgs("`prompt` is required".into()))?;
        let mut input = BootstrapInput::from(&self.factory.opts);
        input.session = None;
        input.ephemeral_sessions = true;
        input.caller = "subagent";
        input.static_tools = Vec::new();
        let mut runtime =
            bootstrap_with(input, &*self.factory.get_env, self.factory.cwd.as_deref())
                .map_err(|error| XyToolError::ExecutionFailed(anyhow::anyhow!(error.to_string())))?
                .into_runtime();
        let read_only =
            read_only_tools(&runtime.driver.agent.tools_snapshot(), &self.factory.tools);
        runtime.driver.agent.set_tools(read_only);
        let mut stream = runtime.driver.run(prompt).await;
        let mut text = String::new();
        loop {
            tokio::select! {
                _ = ctx.cancel.cancelled() => {
                    runtime.driver.abort();
                    return Err(XyToolError::Aborted);
                }
                event = stream.next() => {
                    match event {
                        Some(XyEvent::TextDelta(delta)) => text.push_str(&delta),
                        Some(XyEvent::AgentEnd { messages }) => {
                            if let Some(last) = messages
                                .iter()
                                .rev()
                                .find(|message| message.role_name() == "assistant")
                            {
                                let final_text = last.text();
                                if !final_text.trim().is_empty() {
                                    text = final_text;
                                }
                            }
                            break;
                        }
                        Some(_) => {}
                        None => break,
                    }
                }
            }
        }
        if text.trim().is_empty() {
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                "sub-agent returned no text"
            )));
        }
        Ok(text)
    }
}

fn read_only_tools(tools: &ToolSet, allowed: &[String]) -> ToolSet {
    ToolSet::from_iter(
        tools
            .iter()
            .filter(|tool| allowed.iter().any(|name| name.as_str() == tool.name()))
            .cloned(),
    )
}

pub fn register_subagent_tool(ctx: &xylitol_plugin_sdk::rt::Ctx, opts: &BootOptions) {
    let Some(config) = ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>() else {
        return;
    };
    let Some(subagent) = config.app.subagent.as_ref() else {
        return;
    };
    let Some(registries) = ctx.get::<xylitol_plugin_sdk::HostRegistries>() else {
        return;
    };
    registries.tools.insert(
        "subagent_readonly",
        Arc::new(SubagentTool::new(opts.clone(), subagent.tools.clone())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use xylitol_plugin_sdk::rt::CtxHost;

    use async_trait::async_trait;
    use serde_json::Value;

    struct Named(&'static str);

    #[async_trait]
    impl XyTool for Named {
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "named"
        }
        fn parameters_schema(&self) -> Value {
            json!({})
        }
        async fn execute(&self, _: &XyToolCtx, _: Value) -> Result<String, XyToolError> {
            Ok("ok".into())
        }
    }

    #[test]
    fn read_only_tools_keep_only_allowed_names() {
        let tools = ToolSet::from_iter(vec![
            Arc::new(Named("read")) as Arc<dyn XyTool>,
            Arc::new(Named("write")),
            Arc::new(Named("bash")),
            Arc::new(Named("grep")),
        ]);
        let filtered = read_only_tools(&tools, &["read".into(), "grep".into()]);
        let names: Vec<&str> = filtered.iter().map(|tool| tool.name()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"read"));
        assert!(names.contains(&"grep"));
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(fake_provider)]
    async fn subagent_readonly_runs_fake_model() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: fake\n  models:\n    fake:\n      provider: fake\n      model: fake-1\nsubagent:\n  tools: [read]\n",
        )
        .expect("config");
        crate::provider::factory::reset_fake_state();
        crate::provider::factory::set_fake_text("subagent says hi");
        let opts = BootOptions {
            profile: crate::boot::Profile::Print,
            config_path: Some(config_path),
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "test",
            ephemeral_sessions: true,
        };
        let (ctx, _loader) = crate::boot::boot_ctx(&opts).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(registries.tools.get("subagent_readonly").is_some());
        let isolated = dir.path().to_path_buf();
        let isolated_config = isolated.join(".config").join("xylitol");
        std::fs::create_dir_all(&isolated_config).expect("isolated config dir");
        let env_dir = isolated.clone();
        let config_dir = isolated_config.clone();
        let get_env = Arc::new(move |key: &str| match key {
            "HOME" => Some(env_dir.to_string_lossy().into_owned()),
            "XYLITOL_CONFIG_DIR" => Some(config_dir.to_string_lossy().into_owned()),
            _ => None,
        });
        let tool =
            SubagentTool::with_env(opts.clone(), vec!["read".into()], get_env, Some(isolated));
        let tool_ctx = XyToolCtx::new("test").with_workspace(dir.path());
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tool.execute(&tool_ctx, json!({"prompt": "inspect the workspace"})),
        )
        .await
        .expect("subagent timeout")
        .expect("subagent output");
        assert_eq!(output, "subagent says hi");
        crate::provider::factory::reset_fake_state();
        let _ = ctx.shutdown().await;
    }

    #[test]
    fn tool_requires_prompt() {
        let tool = SubagentTool::new(
            BootOptions {
                profile: crate::boot::Profile::Print,
                config_path: None,
                session: None,
                model: None,
                trust_override: None,
                interactive: false,
                caller: "test",
                ephemeral_sessions: true,
            },
            vec!["read".into()],
        );
        let ctx = XyToolCtx::new("test");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let error = runtime
            .block_on(tool.execute(&ctx, json!({})))
            .expect_err("missing prompt");
        assert!(matches!(error, XyToolError::InvalidArgs(_)));
    }
}
