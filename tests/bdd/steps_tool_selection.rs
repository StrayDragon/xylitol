use std::sync::Arc;

use async_trait::async_trait;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use serde_json::Value;
use xylitol_plugin_sdk::XyToolCtx;
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_runtime::agent::tools::{ToolSet, resolve_tool_selection};

pub struct ToolSelectionBdd {
    pub tools: std::cell::RefCell<Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>>,
    pub entries: std::cell::RefCell<Vec<String>>,
    pub set: std::cell::RefCell<Option<ToolSet>>,
    pub warnings: std::cell::RefCell<Vec<String>>,
}

impl ToolSelectionBdd {
    fn new() -> Self {
        Self {
            tools: std::cell::RefCell::new(Vec::new()),
            entries: std::cell::RefCell::new(Vec::new()),
            set: std::cell::RefCell::new(None),
            warnings: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn names(&self) -> Vec<String> {
        self.set
            .borrow()
            .as_ref()
            .expect("tool set")
            .iter()
            .map(|tool| tool.name().to_string())
            .collect()
    }
}

struct StubTool {
    name: String,
    replaces: &'static [&'static str],
}

#[async_trait]
impl xylitol_plugin_sdk::XyTool for StubTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn replaces(&self) -> &'static [&'static str] {
        self.replaces
    }

    fn description(&self) -> &str {
        "stub"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({ "type": "object" })
    }

    async fn execute(&self, _: &XyToolCtx, _: Value) -> Result<String, XyToolError> {
        Ok(String::new())
    }
}

fn stub(name: &str, replaces: &'static [&'static str]) -> Arc<dyn xylitol_plugin_sdk::XyTool> {
    Arc::new(StubTool {
        name: name.to_string(),
        replaces,
    })
}

fn install(
    tool_selection_bdd: &ToolSelectionBdd,
    tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>,
    entries: &[&str],
) {
    tool_selection_bdd.tools.borrow_mut().extend(tools);
    tool_selection_bdd
        .entries
        .borrow_mut()
        .extend(entries.iter().map(|entry| entry.to_string()));
}

#[fixture]
pub fn tool_selection_bdd() -> ToolSelectionBdd {
    ToolSelectionBdd::new()
}

#[given("tools 列表含 -grep 与 +ffgrep")]
fn g_switch_to_fff(tool_selection_bdd: &ToolSelectionBdd) {
    install(
        tool_selection_bdd,
        vec![stub("grep", &[]), stub("ffgrep", &["grep"])],
        &["-grep", "+ffgrep"],
    );
}

#[given("tools 列表含 +grep 且 ffgrep 已启用")]
fn g_conflict_builtin(tool_selection_bdd: &ToolSelectionBdd) {
    install(
        tool_selection_bdd,
        vec![stub("grep", &[]), stub("ffgrep", &["grep"])],
        &["+grep"],
    );
}

#[given("tools 列表含 -ffgrep 与 -fffind")]
fn g_disable_fff(tool_selection_bdd: &ToolSelectionBdd) {
    install(
        tool_selection_bdd,
        vec![
            stub("grep", &[]),
            stub("find", &[]),
            stub("ffgrep", &["grep"]),
            stub("fffind", &["find"]),
        ],
        &["-ffgrep", "-fffind"],
    );
}

#[given("tools 列表含 +no-such-tool")]
fn g_unknown_tool(tool_selection_bdd: &ToolSelectionBdd) {
    install(
        tool_selection_bdd,
        vec![stub("grep", &[])],
        &["+no-such-tool"],
    );
}

#[given("tools-fff 插件已装配")]
fn g_fff_installed(tool_selection_bdd: &ToolSelectionBdd) {
    install(
        tool_selection_bdd,
        vec![
            stub("grep", &[]),
            stub("find", &[]),
            stub("ffgrep", &["grep"]),
            stub("fffind", &["find"]),
        ],
        &[],
    );
}

#[given("tools-fff 插件未装配")]
fn g_fff_missing(tool_selection_bdd: &ToolSelectionBdd) {
    install(
        tool_selection_bdd,
        vec![stub("grep", &[]), stub("find", &[])],
        &[],
    );
}

#[when("装配工具集")]
fn w_resolve(tool_selection_bdd: &ToolSelectionBdd) {
    let (set, warnings) = resolve_tool_selection(
        tool_selection_bdd.tools.borrow().clone(),
        &tool_selection_bdd.entries.borrow(),
    );
    tool_selection_bdd.set.replace(Some(set));
    tool_selection_bdd.warnings.replace(warnings);
}

#[then("含 ffgrep 且不含 grep")]
fn t_fff_replaces_grep(tool_selection_bdd: &ToolSelectionBdd) {
    let names = tool_selection_bdd.names();
    assert!(names.contains(&"ffgrep".to_string()), "{names:?}");
    assert!(!names.contains(&"grep".to_string()), "{names:?}");
}

#[then("产生同类冲突告警且含 ffgrep 不含 grep")]
fn t_conflict_warns(tool_selection_bdd: &ToolSelectionBdd) {
    let names = tool_selection_bdd.names();
    assert!(names.contains(&"ffgrep".to_string()), "{names:?}");
    assert!(!names.contains(&"grep".to_string()), "{names:?}");
    let warnings = tool_selection_bdd.warnings.borrow();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("both") && w.contains("ffgrep")),
        "{warnings:?}"
    );
}

#[then("含 grep 与 find 且不含 ffgrep 与 fffind")]
fn t_builtin_restored(tool_selection_bdd: &ToolSelectionBdd) {
    let names = tool_selection_bdd.names();
    assert!(names.contains(&"grep".to_string()), "{names:?}");
    assert!(names.contains(&"find".to_string()), "{names:?}");
    assert!(!names.contains(&"ffgrep".to_string()), "{names:?}");
    assert!(!names.contains(&"fffind".to_string()), "{names:?}");
}

#[then("产生未知工具告警且其余工具正常装配")]
fn t_unknown_warns(tool_selection_bdd: &ToolSelectionBdd) {
    let names = tool_selection_bdd.names();
    assert!(names.contains(&"grep".to_string()), "{names:?}");
    let warnings = tool_selection_bdd.warnings.borrow();
    assert!(
        warnings.iter().any(|w| w.contains("unknown tool")),
        "{warnings:?}"
    );
}

#[then("含 ffgrep 与 fffind")]
fn t_fff_tools_present(tool_selection_bdd: &ToolSelectionBdd) {
    let names = tool_selection_bdd.names();
    assert!(names.contains(&"ffgrep".to_string()), "{names:?}");
    assert!(names.contains(&"fffind".to_string()), "{names:?}");
}

#[then("含 grep 与 find 且不含 ffgrep")]
fn t_builtin_fallback(tool_selection_bdd: &ToolSelectionBdd) {
    let names = tool_selection_bdd.names();
    assert!(names.contains(&"grep".to_string()), "{names:?}");
    assert!(names.contains(&"find".to_string()), "{names:?}");
    assert!(!names.contains(&"ffgrep".to_string()), "{names:?}");
}
