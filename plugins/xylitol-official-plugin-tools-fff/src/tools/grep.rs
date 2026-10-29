use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use fff_search::grep::{Casing, GrepMode, GrepSearchOptions, parse_grep_query};
use serde::Deserialize;
use serde_json::{Value, json};
use xylitol_plugin_sdk::XyToolCtx;
use xylitol_plugin_sdk::error::XyToolError;

use super::index::IndexCache;
use super::timeout::parse_tool_timeout;

const DEFAULT_LIMIT: u64 = 100;
const MAX_LIMIT: u64 = 200;
const MAX_CONTEXT: u32 = 20;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FffGrepMode {
    #[default]
    Smart,
    Literal,
    Regex,
    Fuzzy,
}

impl FffGrepMode {
    fn grep_mode(self) -> GrepMode {
        match self {
            Self::Smart | Self::Literal => GrepMode::PlainText,
            Self::Regex => GrepMode::Regex,
            Self::Fuzzy => GrepMode::Fuzzy,
        }
    }

    fn casing(self) -> Casing {
        match self {
            Self::Literal => Casing::Sensitive,
            Self::Smart | Self::Regex | Self::Fuzzy => Casing::Smart,
        }
    }
}

pub struct FffGrepTool {
    cache: IndexCache,
}

impl FffGrepTool {
    pub fn new(cache: IndexCache) -> Self {
        Self { cache }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FffGrepArgs {
    query: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    mode: Option<FffGrepMode>,
    #[serde(default)]
    definitions_only: bool,
    #[serde(default)]
    context: u32,
    #[serde(default = "default_limit")]
    limit: u64,
    #[serde(default)]
    timeout: Option<i64>,
}

fn default_limit() -> u64 {
    DEFAULT_LIMIT
}

fn build_query(query: &str) -> String {
    query.trim().to_string()
}

#[async_trait]
impl xylitol_plugin_sdk::XyTool for FffGrepTool {
    fn name(&self) -> &str {
        "ffgrep"
    }

    fn replaces(&self) -> &'static [&'static str] {
        &["grep"]
    }

    fn description(&self) -> &str {
        "Search file contents with fff's in-process index. Prefer this over grep for repeated searches in a large repo. Supports literal/regex/fuzzy modes, definitionsOnly, context and result limit."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query; fff constraints such as `*.rs` or `!tests/` are supported inline"
                },
                "path": {
                    "type": "string",
                    "description": "Optional directory or file scope, appended to the query as a constraint"
                },
                "glob": {
                    "type": "string",
                    "description": "Optional glob filter, appended to the query as a constraint"
                },
                "mode": {
                    "type": "string",
                    "enum": ["smart", "literal", "regex", "fuzzy"],
                    "description": "Query mode (default: smart = literal + smart case)"
                },
                "definitionsOnly": {
                    "type": "boolean",
                    "description": "Only return lines that look like declarations (default: false)"
                },
                "context": {
                    "type": "integer",
                    "description": "Context lines before/after each match, clamped to 20 (default: 0)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum matches to return, clamped to 1..200 (default: 100)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Optional timeout in seconds; default 60. Clamped to 600s. Zero/negative invalid."
                }
            },
            "required": ["query"]
        })
    }

    fn execution_mode(&self) -> xylitol_plugin_sdk::XyToolExecutionMode {
        xylitol_plugin_sdk::XyToolExecutionMode::Parallel
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let args: FffGrepArgs =
            serde_json::from_value(args).map_err(|e| XyToolError::InvalidArgs(e.to_string()))?;
        let timeout = parse_tool_timeout(args.timeout)?;
        let limit = args.limit.clamp(1, MAX_LIMIT) as usize;
        let context = args.context.min(MAX_CONTEXT) as usize;
        let mode = args.mode.unwrap_or_default();
        let raw_query = build_query(&args.query);
        if raw_query.is_empty() {
            return Err(XyToolError::InvalidArgs("query must not be empty".into()));
        }
        let cache = self.cache.clone();
        let workspace = ctx.workspace.clone();
        let abort = Arc::new(AtomicBool::new(false));
        let abort_for_search = abort.clone();
        let search = tokio::task::spawn_blocking(move || {
            search(
                &cache,
                &workspace,
                &raw_query,
                args.path.as_deref(),
                args.glob.as_deref(),
                mode,
                args.definitions_only,
                context,
                limit,
                abort_for_search,
            )
        });

        tokio::select! {
            result = search => match result {
                Ok(result) => result,
                Err(join) => Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "fff grep task failed: {join}"
                ))),
            },
            _ = ctx.cancel.cancelled() => {
                abort.store(true, Ordering::SeqCst);
                Err(XyToolError::Aborted)
            }
            _ = tokio::time::sleep(timeout) => {
                abort.store(true, Ordering::SeqCst);
                Err(XyToolError::Timeout(timeout))
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn search(
    cache: &IndexCache,
    workspace: &std::path::Path,
    raw_query: &str,
    path: Option<&str>,
    glob: Option<&str>,
    mode: FffGrepMode,
    definitions_only: bool,
    context: usize,
    limit: usize,
    abort: Arc<AtomicBool>,
) -> Result<String, XyToolError> {
    let index = cache
        .get(workspace)
        .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("fff index: {e}")))?;
    let picker = index.picker().clone();
    let guard = picker
        .read()
        .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("fff index lock: {e}")))?;
    let Some(picker) = guard.as_ref() else {
        return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
            "fff index is not initialized"
        )));
    };

    let path = path.map(str::trim).filter(|p| !p.is_empty());
    let glob = glob
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .map(glob::Pattern::new)
        .transpose()
        .map_err(|e| XyToolError::InvalidArgs(format!("invalid glob: {e}")))?;
    let query = parse_grep_query(raw_query);
    let options = GrepSearchOptions {
        page_limit: limit,
        mode: mode.grep_mode(),
        casing: Some(mode.casing()),
        before_context: context,
        after_context: context,
        classify_definitions: definitions_only,
        trim_whitespace: true,
        abort_signal: Some(abort),
        ..Default::default()
    };
    let result = picker.grep(&query, &options);

    let mut out = String::new();
    if index.partial() {
        out.push_str("[index partial]\n");
    }
    let mut emitted = 0;
    for m in &result.matches {
        if definitions_only && !m.is_definition {
            continue;
        }
        if emitted >= limit {
            break;
        }
        let file = result.files[m.file_index].relative_path(picker);
        if let Some(path) = path
            && !file.starts_with(path.trim_end_matches('/'))
        {
            continue;
        }
        if let Some(glob) = &glob {
            let name = file.rsplit('/').next().unwrap_or(&file);
            if !glob.matches(&file) && !glob.matches(name) {
                continue;
            }
        }
        for (i, line) in m.context_before.iter().enumerate() {
            let line_no = m
                .line_number
                .saturating_sub((m.context_before.len() - i) as u64);
            out.push_str(&format!("{file}:{line_no}-{line}\n"));
        }
        out.push_str(&format!(
            "{file}:{}:{}\n",
            m.line_number,
            m.line_content.trim_end()
        ));
        for (i, line) in m.context_after.iter().enumerate() {
            let line_no = m.line_number + 1 + i as u64;
            out.push_str(&format!("{file}:{line_no}-{line}\n"));
        }
        emitted += 1;
    }
    if emitted == 0 {
        out.push_str("No matches found.\n");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::index::IndexCache;
    use std::time::Duration;
    use xylitol_plugin_sdk::XyTool;

    fn write_file(root: &std::path::Path, rel: &str, content: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    async fn grep(root: &std::path::Path, query: &str) -> Result<String, XyToolError> {
        grep_with(root, json!({ "query": query })).await
    }

    async fn grep_with(root: &std::path::Path, args: Value) -> Result<String, XyToolError> {
        let cache = IndexCache::new(false, Duration::from_secs(10));
        let tool = FffGrepTool::new(cache);
        let ctx = XyToolCtx::new("test").with_workspace(root);
        tool.execute(&ctx, args).await
    }

    #[tokio::test]
    async fn finds_content_with_path_and_line() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(
            tmp.path(),
            "src/main.rs",
            "fn main() {\n    println!(\"hi\");\n}\n",
        );
        let out = grep(tmp.path(), "fn main").await.expect("grep");
        assert!(out.contains("src/main.rs:"), "{out}");
        assert!(out.contains("fn main"), "{out}");
    }

    #[tokio::test]
    async fn path_scope_filters_matches() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/main.rs", "fn main() {}\n");
        write_file(tmp.path(), "tests/other.rs", "fn main() {}\n");
        let out = grep_with(tmp.path(), json!({ "query": "fn main", "path": "src" }))
            .await
            .expect("grep");
        assert!(out.contains("src/main.rs"), "{out}");
        assert!(!out.contains("tests/other.rs"), "{out}");
    }

    #[tokio::test]
    async fn glob_scope_filters_matches() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/main.rs", "fn main() {}\n");
        write_file(tmp.path(), "docs/readme.md", "fn main() {}\n");
        let out = grep_with(tmp.path(), json!({ "query": "fn main", "glob": "*.rs" }))
            .await
            .expect("grep");
        assert!(out.contains("src/main.rs"), "{out}");
        assert!(!out.contains("docs/readme.md"), "{out}");
    }

    #[tokio::test]
    async fn zero_timeout_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = IndexCache::new(false, Duration::from_secs(10));
        let tool = FffGrepTool::new(cache);
        let ctx = XyToolCtx::new("test").with_workspace(tmp.path());
        let err = tool
            .execute(&ctx, json!({ "query": "x", "timeout": 0 }))
            .await
            .expect_err("zero timeout");
        assert!(matches!(err, XyToolError::InvalidArgs(_)), "{err}");
    }

    #[test]
    fn mode_mapping() {
        assert!(matches!(
            FffGrepMode::Smart.grep_mode(),
            GrepMode::PlainText
        ));
        assert!(matches!(FffGrepMode::Literal.casing(), Casing::Sensitive));
        assert!(matches!(FffGrepMode::Regex.grep_mode(), GrepMode::Regex));
        assert!(matches!(FffGrepMode::Fuzzy.grep_mode(), GrepMode::Fuzzy));
    }
}
