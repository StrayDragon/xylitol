use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use xylitol_plugin_sdk::XyTool;

use super::toolset::ToolSet;

pub fn resolve_tool_selection(
    tools: Vec<Arc<dyn XyTool>>,
    entries: &[String],
) -> (ToolSet, Vec<String>) {
    let mut ordered: Vec<Arc<dyn XyTool>> = Vec::new();
    for tool in tools {
        if let Some(pos) = ordered.iter().position(|t| t.name() == tool.name()) {
            ordered[pos] = tool;
        } else {
            ordered.push(tool);
        }
    }

    let names: BTreeSet<String> = ordered.iter().map(|t| t.name().to_string()).collect();
    let mut enabled: HashSet<String> = names.iter().cloned().collect();
    let mut explicit_plus: HashSet<String> = HashSet::new();
    let mut warnings = Vec::new();

    for raw in entries {
        let entry = raw.trim();
        if entry.is_empty() {
            continue;
        }
        let (disable, name) = match entry.strip_prefix('-') {
            Some(rest) => (true, rest.trim()),
            None => (false, entry.strip_prefix('+').unwrap_or(entry).trim()),
        };
        if name.is_empty() {
            continue;
        }
        if !names.contains(name) {
            warnings.push(format!("tools list references unknown tool `{name}`"));
            continue;
        }
        if disable {
            enabled.remove(name);
            explicit_plus.remove(name);
        } else {
            enabled.insert(name.to_string());
            explicit_plus.insert(name.to_string());
        }
    }

    let mut replacers: Vec<&Arc<dyn XyTool>> = ordered
        .iter()
        .filter(|tool| enabled.contains(tool.name()) && !tool.replaces().is_empty())
        .collect();
    replacers.sort_by(|a, b| a.name().cmp(b.name()));

    for tool in replacers {
        if !enabled.contains(tool.name()) {
            continue;
        }
        for target in tool.replaces() {
            if *target == tool.name() || !enabled.contains(*target) {
                continue;
            }
            enabled.remove(*target);
            if explicit_plus.contains(*target) {
                warnings.push(format!(
                    "tools list enables both `{}` and `{}`; keeping `{}`",
                    tool.name(),
                    target,
                    tool.name()
                ));
            }
        }
    }

    let set = ToolSet::from_iter(
        ordered
            .into_iter()
            .filter(|tool| enabled.contains(tool.name())),
    );
    (set, warnings)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::Value;

    use super::*;
    use xylitol_plugin_sdk::XyToolCtx;
    use xylitol_plugin_sdk::error::XyToolError;

    struct StubTool {
        name: &'static str,
        replaces: &'static [&'static str],
    }

    #[async_trait]
    impl XyTool for StubTool {
        fn name(&self) -> &str {
            self.name
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

    fn tool(name: &'static str, replaces: &'static [&'static str]) -> Arc<dyn XyTool> {
        Arc::new(StubTool { name, replaces })
    }

    fn names(set: &ToolSet) -> Vec<String> {
        set.iter().map(|t| t.name().to_string()).collect()
    }

    #[test]
    fn later_entry_wins() {
        let tools = vec![tool("grep", &[]), tool("ls", &[])];
        let (set, warnings) = resolve_tool_selection(tools, &["-grep".into(), "+grep".into()]);
        assert!(names(&set).contains(&"grep".to_string()));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn replacement_hides_builtin_by_default() {
        let tools = vec![tool("grep", &[]), tool("ffgrep", &["grep"])];
        let (set, warnings) = resolve_tool_selection(tools, &[]);
        let names = names(&set);
        assert!(names.contains(&"ffgrep".to_string()));
        assert!(!names.contains(&"grep".to_string()));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn explicit_builtin_conflict_warns_and_keeps_replacer() {
        let tools = vec![tool("grep", &[]), tool("ffgrep", &["grep"])];
        let (set, warnings) = resolve_tool_selection(tools, &["+grep".into()]);
        let names = names(&set);
        assert!(names.contains(&"ffgrep".to_string()));
        assert!(!names.contains(&"grep".to_string()));
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("both") && w.contains("ffgrep")),
            "{warnings:?}"
        );
    }

    #[test]
    fn disabling_replacer_restores_builtin() {
        let tools = vec![tool("grep", &[]), tool("ffgrep", &["grep"])];
        let (set, warnings) = resolve_tool_selection(tools, &["-ffgrep".into()]);
        let names = names(&set);
        assert!(names.contains(&"grep".to_string()));
        assert!(!names.contains(&"ffgrep".to_string()));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn unknown_tool_warns_without_failing() {
        let tools = vec![tool("grep", &[])];
        let (set, warnings) = resolve_tool_selection(tools, &["+no-such-tool".into()]);
        assert!(names(&set).contains(&"grep".to_string()));
        assert!(
            warnings.iter().any(|w| w.contains("unknown tool")),
            "{warnings:?}"
        );
    }

    #[test]
    fn replacement_cycle_keeps_one() {
        let tools = vec![tool("aaa", &["bbb"]), tool("bbb", &["aaa"])];
        let (set, _) = resolve_tool_selection(tools, &[]);
        assert_eq!(names(&set), vec!["aaa".to_string()]);
    }

    #[test]
    fn duplicate_names_last_wins() {
        let tools = vec![tool("grep", &[]), tool("grep", &[])];
        let (set, _) = resolve_tool_selection(tools, &[]);
        assert_eq!(names(&set), vec!["grep".to_string()]);
    }
}
