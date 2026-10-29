use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use fff_search::QueryParser;
use fff_search::file_picker::FuzzySearchOptions;
use fff_search::types::PaginationArgs;
use serde::Deserialize;
use serde_json::{Value, json};
use xylitol_plugin_sdk::XyToolCtx;
use xylitol_plugin_sdk::error::XyToolError;

use super::index::IndexCache;
use super::timeout::parse_tool_timeout;

const DEFAULT_LIMIT: u64 = 100;
const MAX_LIMIT: u64 = 1_000;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FffFindMode {
    #[default]
    Fuzzy,
    Glob,
    Regex,
}

pub struct FffFindTool {
    cache: IndexCache,
}

impl FffFindTool {
    pub fn new(cache: IndexCache) -> Self {
        Self { cache }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FffFindArgs {
    query: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    mode: Option<FffFindMode>,
    #[serde(default = "default_limit")]
    limit: u64,
    #[serde(default)]
    timeout: Option<i64>,
}

fn default_limit() -> u64 {
    DEFAULT_LIMIT
}

#[async_trait]
impl xylitol_plugin_sdk::XyTool for FffFindTool {
    fn name(&self) -> &str {
        "fffind"
    }

    fn replaces(&self) -> &'static [&'static str] {
        &["find"]
    }

    fn description(&self) -> &str {
        "Find files by fuzzy path fragment, glob or regex using fff's in-process index. Prefer this over find for repeated searches. Returns paths relative to the session workspace."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Path fragment (fuzzy), glob pattern, or regex"
                },
                "path": {
                    "type": "string",
                    "description": "Optional directory scope; results are filtered to this prefix"
                },
                "mode": {
                    "type": "string",
                    "enum": ["fuzzy", "glob", "regex"],
                    "description": "Match mode (default: fuzzy)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum paths to return, clamped to 1..1000 (default: 100)"
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
        let args: FffFindArgs =
            serde_json::from_value(args).map_err(|e| XyToolError::InvalidArgs(e.to_string()))?;
        let timeout = parse_tool_timeout(args.timeout)?;
        let limit = args.limit.clamp(1, MAX_LIMIT) as usize;
        let mode = args.mode.unwrap_or_default();
        let query = args.query.trim().to_string();
        if query.is_empty() {
            return Err(XyToolError::InvalidArgs("query must not be empty".into()));
        }
        let path = args
            .path
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty());
        let cache = self.cache.clone();
        let workspace = ctx.workspace.clone();
        let abort = Arc::new(AtomicBool::new(false));
        let abort_for_search = abort.clone();
        let search = tokio::task::spawn_blocking(move || {
            search(
                &cache,
                &workspace,
                &query,
                path.as_deref(),
                mode,
                limit,
                abort_for_search,
            )
        });

        tokio::select! {
            result = search => match result {
                Ok(result) => result,
                Err(join) => Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "fff find task failed: {join}"
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
    query: &str,
    path: Option<&str>,
    mode: FffFindMode,
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

    let mut out = String::new();
    if index.partial() {
        out.push_str("[index partial]\n");
    }
    let mut emitted = 0;

    match mode {
        FffFindMode::Fuzzy => {
            let parsed = QueryParser::default().parse(query);
            let result = picker.fuzzy_search(
                &parsed,
                None,
                FuzzySearchOptions {
                    pagination: PaginationArgs { offset: 0, limit },
                    ..Default::default()
                },
            );
            for item in result.items {
                if emitted >= limit {
                    break;
                }
                let rel = item.relative_path(picker);
                if let Some(path) = path
                    && !rel.starts_with(path.trim_end_matches('/'))
                {
                    continue;
                }
                out.push_str(&rel);
                out.push('\n');
                emitted += 1;
            }
        }
        FffFindMode::Glob => {
            let pattern = glob::Pattern::new(query)
                .map_err(|e| XyToolError::InvalidArgs(format!("invalid glob: {e}")))?;
            for file in picker.get_files() {
                if abort.load(Ordering::Relaxed) {
                    return Err(XyToolError::Aborted);
                }
                let rel = file.relative_path(picker);
                if !pattern.matches(&rel) {
                    continue;
                }
                if let Some(path) = path
                    && !rel.starts_with(path.trim_end_matches('/'))
                {
                    continue;
                }
                out.push_str(&rel);
                out.push('\n');
                emitted += 1;
                if emitted >= limit {
                    break;
                }
            }
        }
        FffFindMode::Regex => {
            let pattern = regex::Regex::new(query)
                .map_err(|e| XyToolError::InvalidArgs(format!("invalid regex: {e}")))?;
            for file in picker.get_files() {
                if abort.load(Ordering::Relaxed) {
                    return Err(XyToolError::Aborted);
                }
                let rel = file.relative_path(picker);
                if !pattern.is_match(&rel) {
                    continue;
                }
                if let Some(path) = path
                    && !rel.starts_with(path.trim_end_matches('/'))
                {
                    continue;
                }
                out.push_str(&rel);
                out.push('\n');
                emitted += 1;
                if emitted >= limit {
                    break;
                }
            }
        }
    }

    if emitted == 0 {
        out.push_str("No files found.\n");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use xylitol_plugin_sdk::XyTool;

    fn write_file(root: &std::path::Path, rel: &str, content: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    async fn find(root: &std::path::Path, args: Value) -> Result<String, XyToolError> {
        let cache = IndexCache::new(false, Duration::from_secs(10));
        let tool = FffFindTool::new(cache);
        let ctx = XyToolCtx::new("test").with_workspace(root);
        tool.execute(&ctx, args).await
    }

    #[tokio::test]
    async fn finds_file_by_fragment() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/lib.rs", "pub fn f() {}\n");
        write_file(tmp.path(), "src/main.rs", "fn main() {}\n");
        let out = find(tmp.path(), json!({ "query": "lib" }))
            .await
            .expect("find");
        assert!(out.contains("src/lib.rs"), "{out}");
    }

    #[tokio::test]
    async fn glob_mode_matches_paths() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/lib.rs", "");
        write_file(tmp.path(), "src/readme.md", "");
        let out = find(tmp.path(), json!({ "query": "src/*.rs", "mode": "glob" }))
            .await
            .expect("find");
        assert!(out.contains("src/lib.rs"), "{out}");
        assert!(!out.contains("readme.md"), "{out}");
    }

    #[tokio::test]
    async fn invalid_regex_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let err = find(tmp.path(), json!({ "query": "[", "mode": "regex" }))
            .await
            .expect_err("invalid regex");
        assert!(matches!(err, XyToolError::InvalidArgs(_)), "{err}");
    }
}
