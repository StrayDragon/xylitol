use std::path::Path;
use std::time::Duration;

use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use serde_json::json;
use xylitol_official_plugin_tools_fff::{FffFindTool, FffGrepTool, IndexCache};
use xylitol_plugin_sdk::XyTool;
use xylitol_plugin_sdk::XyToolCtx;
use xylitol_plugin_sdk::error::XyToolError;

pub struct FffSearchBdd {
    pub workspace: std::cell::RefCell<Option<tempfile::TempDir>>,
    pub workspace2: std::cell::RefCell<Option<tempfile::TempDir>>,
    pub output: std::cell::RefCell<Option<String>>,
    pub error: std::cell::RefCell<Option<String>>,
    pub outputs: std::cell::RefCell<Vec<String>>,
}

impl FffSearchBdd {
    fn new() -> Self {
        Self {
            workspace: std::cell::RefCell::new(None),
            workspace2: std::cell::RefCell::new(None),
            output: std::cell::RefCell::new(None),
            error: std::cell::RefCell::new(None),
            outputs: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn root(&self) -> std::path::PathBuf {
        self.workspace
            .borrow()
            .as_ref()
            .expect("workspace")
            .path()
            .to_path_buf()
    }
}

#[fixture]
pub fn fff_search_bdd() -> FffSearchBdd {
    FffSearchBdd::new()
}

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).unwrap();
    std::fs::write(path, content).unwrap();
}

async fn run_grep(root: &Path, query: &str, timeout: Option<i64>) -> Result<String, XyToolError> {
    let cache = IndexCache::new(false, Duration::from_secs(10));
    let tool = FffGrepTool::new(cache);
    let ctx = XyToolCtx::new("bdd").with_workspace(root);
    let mut args = json!({ "query": query });
    if let Some(timeout) = timeout {
        args["timeout"] = json!(timeout);
    }
    tool.execute(&ctx, args).await
}

async fn run_find(root: &Path, query: &str) -> Result<String, XyToolError> {
    let cache = IndexCache::new(false, Duration::from_secs(10));
    let tool = FffFindTool::new(cache);
    let ctx = XyToolCtx::new("bdd").with_workspace(root);
    tool.execute(&ctx, json!({ "query": query })).await
}

#[given("会话工作区存在文件 {path:string}")]
fn g_workspace_file_empty(fff_search_bdd: &FffSearchBdd, path: String) {
    let tmp = tempfile::tempdir().unwrap();
    write_file(tmp.path(), &path, "");
    fff_search_bdd.workspace.replace(Some(tmp));
}

#[given("会话工作区存在文件 {path:string} 内容为 {content:string}")]
fn g_workspace_file(fff_search_bdd: &FffSearchBdd, path: String, content: String) {
    let tmp = tempfile::tempdir().unwrap();
    write_file(tmp.path(), &path, &content);
    fff_search_bdd.workspace.replace(Some(tmp));
}

#[given("两个会话工作区分别存在不同文件")]
fn g_two_workspaces(fff_search_bdd: &FffSearchBdd) {
    let one = tempfile::tempdir().unwrap();
    write_file(one.path(), "one.rs", "fn one() {}\n");
    let two = tempfile::tempdir().unwrap();
    write_file(two.path(), "two.rs", "fn two() {}\n");
    fff_search_bdd.workspace.replace(Some(one));
    fff_search_bdd.workspace2.replace(Some(two));
}

#[when("以 ffgrep 检索 {query:string}")]
async fn w_ffgrep(fff_search_bdd: &FffSearchBdd, query: String) {
    match run_grep(&fff_search_bdd.root(), &query, None).await {
        Ok(out) => {
            fff_search_bdd.output.replace(Some(out));
            fff_search_bdd.error.replace(None);
        }
        Err(err) => {
            fff_search_bdd.output.replace(None);
            fff_search_bdd.error.replace(Some(err.to_string()));
        }
    }
}

#[when("以 ffgrep 检索 {query:string} 且 timeout 为 {timeout:i64}")]
async fn w_ffgrep_timeout(fff_search_bdd: &FffSearchBdd, query: String, timeout: i64) {
    match run_grep(&fff_search_bdd.root(), &query, Some(timeout)).await {
        Ok(out) => {
            fff_search_bdd.output.replace(Some(out));
            fff_search_bdd.error.replace(None);
        }
        Err(err) => {
            fff_search_bdd.output.replace(None);
            fff_search_bdd.error.replace(Some(err.to_string()));
        }
    }
}

#[when("以 fffind 检索 {query:string}")]
async fn w_fffind(fff_search_bdd: &FffSearchBdd, query: String) {
    match run_find(&fff_search_bdd.root(), &query).await {
        Ok(out) => {
            fff_search_bdd.output.replace(Some(out));
            fff_search_bdd.error.replace(None);
        }
        Err(err) => {
            fff_search_bdd.output.replace(None);
            fff_search_bdd.error.replace(Some(err.to_string()));
        }
    }
}

#[when("分别以 ffgrep 检索各自文件")]
async fn w_ffgrep_two_workspaces(fff_search_bdd: &FffSearchBdd) {
    let one = fff_search_bdd
        .workspace
        .borrow()
        .as_ref()
        .expect("workspace one")
        .path()
        .to_path_buf();
    let two = fff_search_bdd
        .workspace2
        .borrow()
        .as_ref()
        .expect("workspace two")
        .path()
        .to_path_buf();
    let first = run_grep(&one, "one", None).await.expect("grep one");
    let second = run_grep(&two, "two", None).await.expect("grep two");
    fff_search_bdd.outputs.replace(vec![first, second]);
}

#[then("输出含 {a:string}")]
fn t_output_contains_one(fff_search_bdd: &FffSearchBdd, a: String) {
    let out = fff_search_bdd.output.borrow();
    let out = out.as_ref().expect("output");
    assert!(out.contains(&a), "expected {a:?} in {out:?}");
}

#[then("输出含 {a:string} 与 {b:string}")]
fn t_output_contains(fff_search_bdd: &FffSearchBdd, a: String, b: String) {
    let out = fff_search_bdd.output.borrow();
    let out = out.as_ref().expect("output");
    assert!(out.contains(&a), "expected {a:?} in {out:?}");
    assert!(out.contains(&b), "expected {b:?} in {out:?}");
}

#[then("ffgrep 以无效 timeout 错误拒绝")]
fn t_invalid_timeout(fff_search_bdd: &FffSearchBdd) {
    let err = fff_search_bdd.error.borrow();
    let err = err.as_ref().expect("expected timeout error");
    assert!(err.contains("invalid arguments"), "{err}");
}

#[then("各自输出只含本工作区路径")]
fn t_workspace_isolated(fff_search_bdd: &FffSearchBdd) {
    let outputs = fff_search_bdd.outputs.borrow();
    assert_eq!(outputs.len(), 2, "{outputs:?}");
    assert!(outputs[0].contains("one.rs"), "{outputs:?}");
    assert!(!outputs[0].contains("two.rs"), "{outputs:?}");
    assert!(outputs[1].contains("two.rs"), "{outputs:?}");
    assert!(!outputs[1].contains("one.rs"), "{outputs:?}");
}
