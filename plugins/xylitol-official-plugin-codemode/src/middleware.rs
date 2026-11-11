use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use async_trait::async_trait;
use xylitol_plugin_sdk::{
    BeforeLlmRequestCtx, LoopMiddleware, LoopStage, LoopStep, XyToolDescriptor,
};

use crate::declarations::render_declarations;

pub struct CodemodeMiddleware {
    budget: usize,
    cache: Mutex<Option<(u64, String)>>,
}

impl CodemodeMiddleware {
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            cache: Mutex::new(None),
        }
    }

    fn declarations(&self, catalog: &[XyToolDescriptor]) -> String {
        if catalog.is_empty() {
            return String::new();
        }
        let key = fingerprint(catalog);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((cached_key, text)) = cache.as_ref()
            && *cached_key == key
        {
            return text.clone();
        }
        let text = render_declarations(catalog, self.budget);
        *cache = Some((key, text.clone()));
        text
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
        let declarations = self.declarations(&ctx.tool_catalog);
        if !declarations.is_empty() {
            let prompt = ctx.options.system_prompt.get_or_insert_with(String::new);
            prompt.push_str("\n\n## Codemode tools\n");
            prompt.push_str(&declarations);
        }
        LoopStep::continued(ctx)
    }
}

fn fingerprint(catalog: &[XyToolDescriptor]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for descriptor in catalog {
        serde_json::to_string(descriptor)
            .unwrap_or_default()
            .hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use xylitol_plugin_sdk::{BeforeLlmRequestCtx, XyGenerateOptions, XyToolExposure};

    fn descriptor(name: &str) -> XyToolDescriptor {
        XyToolDescriptor {
            name: name.into(),
            identifier: name.into(),
            namespace: Some("core".into()),
            description: "desc".into(),
            parameters: json!({"type": "object"}),
            output_schema: None,
            exposure: XyToolExposure::Direct,
            guidelines: Vec::new(),
        }
    }

    fn ctx(catalog: Vec<XyToolDescriptor>) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            step_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            tool_catalog: Arc::from(catalog),
            options: XyGenerateOptions::default(),
            todo: None,
            session_id: None,
        }
    }

    #[tokio::test]
    async fn appends_declarations_from_ctx_catalog() {
        let middleware = CodemodeMiddleware::new(10_000);
        let ctx = ctx(vec![descriptor("read")]);
        let step = middleware.before_llm_request(ctx).await;
        let prompt = step.ctx.options.system_prompt.expect("prompt");
        assert!(prompt.contains("## Codemode tools"));
        assert!(prompt.contains("- read: desc"));
    }

    #[tokio::test]
    async fn empty_catalog_appends_nothing() {
        let middleware = CodemodeMiddleware::new(10_000);
        let step = middleware.before_llm_request(ctx(Vec::new())).await;
        assert!(step.ctx.options.system_prompt.is_none());
    }

    #[tokio::test]
    async fn cache_recomputes_when_catalog_changes() {
        let middleware = CodemodeMiddleware::new(10_000);
        let first = middleware
            .before_llm_request(ctx(vec![descriptor("read")]))
            .await
            .ctx
            .options
            .system_prompt
            .expect("first");
        assert!(first.contains("- read"));
        let second = middleware
            .before_llm_request(ctx(vec![descriptor("write")]))
            .await
            .ctx
            .options
            .system_prompt
            .expect("second");
        assert!(second.contains("- write"));
        assert!(!second.contains("- read"));
    }
}
