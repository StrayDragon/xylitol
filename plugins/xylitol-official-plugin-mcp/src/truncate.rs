use async_trait::async_trait;

use xylitol_plugin_sdk::{AfterToolCtx, LoopMiddleware, LoopStage, LoopStep};

pub const MCP_RESULT_MAX_BYTES: usize = 50 * 1024;

pub struct McpResultTruncate;

impl McpResultTruncate {
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpResultTruncate {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_text(text: &str) -> Option<String> {
    if text.len() <= MCP_RESULT_MAX_BYTES {
        return None;
    }
    let mut end = MCP_RESULT_MAX_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let kept = &text[..end];
    let dropped = text.len() - end;
    Some(format!(
        "{kept}\n\n[truncated: {} kept, {} dropped of {} bytes; limit {} bytes]",
        end,
        dropped,
        text.len(),
        MCP_RESULT_MAX_BYTES
    ))
}

#[async_trait]
impl LoopMiddleware for McpResultTruncate {
    fn id(&self) -> &str {
        "mcp-result-truncate"
    }

    fn order(&self) -> i64 {
        900
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::AfterTool]
    }

    async fn after_tool(&self, mut ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        if ctx.is_error || !xylitol_plugin_sdk::is_mcp_tool_name(&ctx.tool_name) {
            return LoopStep::continued(ctx);
        }
        let [xylitol_plugin_sdk::AgentPart::Text { text }] = &ctx.parts[..] else {
            return LoopStep::continued(ctx);
        };
        let Some(truncated) = truncate_text(text) else {
            return LoopStep::continued(ctx);
        };
        ctx.parts = vec![xylitol_plugin_sdk::AgentPart::text(truncated)];
        LoopStep::continued(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::AgentPart;

    fn ctx(name: &str, text: &str) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error: false,
        }
    }

    #[tokio::test]
    async fn small_mcp_output_is_untouched() {
        let mw = McpResultTruncate::new();
        let step = mw.after_tool(ctx("mcp__srv__tool", "ok")).await;
        assert_eq!(step.ctx.parts[0].as_text().unwrap(), "ok");
    }

    #[tokio::test]
    async fn oversized_mcp_output_is_truncated_with_notice() {
        let mw = McpResultTruncate::new();
        let big = "x".repeat(MCP_RESULT_MAX_BYTES + 100);
        let step = mw.after_tool(ctx("mcp__srv__tool", &big)).await;
        let text = step.ctx.parts[0].as_text().unwrap();
        assert!(text.len() < big.len(), "must shrink");
        assert!(text.contains("[truncated:"), "must announce the cut");
        assert!(text.starts_with(&"x".repeat(MCP_RESULT_MAX_BYTES)));
    }

    #[tokio::test]
    async fn non_mcp_tools_are_never_truncated() {
        let mw = McpResultTruncate::new();
        let big = "x".repeat(MCP_RESULT_MAX_BYTES + 100);
        let step = mw.after_tool(ctx("bash", &big)).await;
        assert_eq!(step.ctx.parts[0].as_text().unwrap().len(), big.len());
    }

    #[tokio::test]
    async fn multi_part_results_are_preserved() {
        let mw = McpResultTruncate::new();
        let mut c = ctx("mcp__srv__tool", "ignored");
        c.parts = vec![
            AgentPart::text("note"),
            AgentPart::Image(xylitol_plugin_sdk::ImageContent {
                url: None,
                data: Some("AAA".into()),
                media_type: "image/png".into(),
            }),
        ];
        let step = mw.after_tool(c).await;
        assert_eq!(step.ctx.parts.len(), 2, "structured parts must survive");
        assert!(matches!(step.ctx.parts[1], AgentPart::Image(_)));
    }

    #[tokio::test]
    async fn error_results_are_not_truncated() {
        let mw = McpResultTruncate::new();
        let mut c = ctx("mcp__srv__tool", &"x".repeat(MCP_RESULT_MAX_BYTES + 100));
        c.is_error = true;
        let step = mw.after_tool(c).await;
        assert_eq!(
            step.ctx.parts[0].as_text().unwrap().len(),
            MCP_RESULT_MAX_BYTES + 100
        );
    }
}
