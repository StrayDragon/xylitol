use async_trait::async_trait;
use xylitol_plugin_sdk::{BeforeLlmRequestCtx, LoopMiddleware, LoopStage, LoopStep};

pub struct CodemodeMiddleware {
    declarations: String,
}

impl CodemodeMiddleware {
    pub fn new(declarations: String) -> Self {
        Self { declarations }
    }
}

#[async_trait]
impl LoopMiddleware for CodemodeMiddleware {
    fn id(&self) -> &str {
        "codemode-declarations"
    }

    fn order(&self) -> i64 {
        60
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::BeforeLlmRequest]
    }

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
        if !self.declarations.is_empty() {
            let prompt = ctx.options.system_prompt.get_or_insert_with(String::new);
            prompt.push_str("\n\n## Codemode tools\n");
            prompt.push_str(&self.declarations);
        }
        LoopStep::continued(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::{BeforeLlmRequestCtx, XyGenerateOptions};

    #[tokio::test]
    async fn appends_declarations_to_system_prompt() {
        let middleware = CodemodeMiddleware::new("- read: read files\n".into());
        let ctx = BeforeLlmRequestCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: XyGenerateOptions::default(),
            todo: None,
            session_id: None,
        };
        let step = middleware.before_llm_request(ctx).await;
        let prompt = step.ctx.options.system_prompt.expect("prompt");
        assert!(prompt.contains("## Codemode tools"));
        assert!(prompt.contains("- read: read files"));
    }
}
