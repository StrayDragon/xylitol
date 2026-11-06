use std::collections::{BTreeSet, HashMap};

use xylitol_plugin_sdk::{XyTool, XyToolCatalog, XyToolDescriptor};

use super::ToolSet;

const CORE_TOOLS: &[&str] = &["read", "write", "edit", "bash", "grep", "find", "ls"];

#[derive(Clone, Default)]
pub struct ToolCatalog {
    descriptors: Vec<XyToolDescriptor>,
    by_name: HashMap<String, XyToolDescriptor>,
}

impl ToolCatalog {
    pub fn from_toolset(set: &ToolSet, allowed: Option<&BTreeSet<String>>) -> Self {
        let policy = xylitol_plugin_sdk::codemode_exposure_policy();
        let overrides = xylitol_plugin_sdk::codemode_exposure_overrides();
        let mut tools: Vec<_> = set
            .iter()
            .filter(|tool| {
                effective_exposure(tool.name(), tool.exposure(), &overrides).codemode_visible()
            })
            .filter(|tool| match allowed {
                Some(names) => names.contains(tool.name()),
                None => policy
                    .as_ref()
                    .is_none_or(|policy| policy.allows(tool.name())),
            })
            .cloned()
            .collect();
        tools.sort_by(|left, right| left.name().cmp(right.name()));

        let mut used = BTreeSet::new();
        let mut descriptors = Vec::with_capacity(tools.len());
        for tool in tools {
            let identifier = unique_identifier(tool.name(), &mut used);
            let exposure = effective_exposure(tool.name(), tool.exposure(), &overrides);
            descriptors.push(descriptor_for(tool.as_ref(), identifier, exposure));
        }
        let by_name = descriptors
            .iter()
            .map(|descriptor| (descriptor.name.clone(), descriptor.clone()))
            .collect();
        Self {
            descriptors,
            by_name,
        }
    }

    pub fn descriptors(&self) -> &[XyToolDescriptor] {
        &self.descriptors
    }

    pub fn allowed_names(&self) -> BTreeSet<String> {
        self.by_name.keys().cloned().collect()
    }
}

impl XyToolCatalog for ToolCatalog {
    fn snapshot(&self) -> Vec<XyToolDescriptor> {
        self.descriptors.clone()
    }

    fn describe(&self, name: &str) -> Option<XyToolDescriptor> {
        self.by_name.get(name).cloned()
    }

    fn search(&self, query: &str, limit: usize) -> Vec<XyToolDescriptor> {
        let needle = query.to_ascii_lowercase();
        self.descriptors
            .iter()
            .filter(|descriptor| {
                descriptor.name.to_ascii_lowercase().contains(&needle)
                    || descriptor
                        .description
                        .to_ascii_lowercase()
                        .contains(&needle)
            })
            .take(limit)
            .cloned()
            .collect()
    }
}

fn effective_exposure(
    name: &str,
    exposure: xylitol_plugin_sdk::XyToolExposure,
    overrides: &std::collections::BTreeMap<String, xylitol_plugin_sdk::XyToolExposure>,
) -> xylitol_plugin_sdk::XyToolExposure {
    overrides.get(name).copied().unwrap_or(exposure)
}

fn descriptor_for(
    tool: &dyn XyTool,
    identifier: String,
    exposure: xylitol_plugin_sdk::XyToolExposure,
) -> XyToolDescriptor {
    XyToolDescriptor {
        name: tool.name().to_string(),
        identifier,
        namespace: namespace_for(tool.name(), tool.namespace()),
        description: tool.description().to_string(),
        parameters: tool.parameters_schema(),
        output_schema: tool.output_schema(),
        exposure,
        guidelines: tool
            .prompt_guidelines()
            .iter()
            .map(|guideline| (*guideline).to_string())
            .collect(),
    }
}

fn namespace_for(name: &str, explicit: Option<&str>) -> Option<String> {
    if let Some(namespace) = explicit {
        return Some(namespace.to_string());
    }
    if let Some(rest) = name.strip_prefix("mcp__") {
        let server = rest.split("__").next().unwrap_or(rest);
        return Some(format!("mcp:{server}"));
    }
    if let Some(rest) = name.strip_prefix("plugin__") {
        let plugin = rest.split("__").next().unwrap_or(rest);
        return Some(format!("plugin:{plugin}"));
    }
    if CORE_TOOLS.contains(&name) {
        return Some("core".to_string());
    }
    Some("official".to_string())
}

fn unique_identifier(name: &str, used: &mut BTreeSet<String>) -> String {
    let base = sanitize_identifier(name);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while !used.insert(candidate.clone()) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    candidate
}

fn sanitize_identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::{Value, json};

    use super::*;
    use xylitol_plugin_sdk::{XyToolCtx, XyToolExposure};

    struct StubTool {
        name: &'static str,
        exposure: XyToolExposure,
        namespace: Option<&'static str>,
    }

    #[async_trait]
    impl XyTool for StubTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            "stub"
        }
        fn parameters_schema(&self) -> Value {
            json!({"type": "object"})
        }
        fn exposure(&self) -> XyToolExposure {
            self.exposure
        }
        fn namespace(&self) -> Option<&str> {
            self.namespace
        }
        async fn execute(
            &self,
            _: &XyToolCtx,
            _: Value,
        ) -> Result<String, xylitol_plugin_sdk::XyToolError> {
            Ok("ok".into())
        }
    }

    fn tool(name: &'static str, exposure: XyToolExposure) -> Arc<dyn XyTool> {
        Arc::new(StubTool {
            name,
            exposure,
            namespace: None,
        })
    }

    #[test]
    fn exposure_splits_model_and_codemode_surfaces() {
        let set = ToolSet::from_iter(vec![
            tool("read", XyToolExposure::Direct),
            tool("ask", XyToolExposure::ModelOnly),
            tool("code_only", XyToolExposure::Codemode),
            tool("later", XyToolExposure::Deferred),
            tool("secret", XyToolExposure::Hidden),
        ]);
        let catalog = ToolCatalog::from_toolset(&set, None);
        let names: Vec<_> = catalog.snapshot().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["code_only", "later", "read"]);
        assert!(catalog.describe("ask").is_none());
        assert!(catalog.describe("secret").is_none());
    }

    #[test]
    fn identifiers_are_unique_and_namespaced() {
        let set = ToolSet::from_iter(vec![
            tool("mcp__fs__read", XyToolExposure::Direct),
            tool("mcp__fs__read", XyToolExposure::Direct),
        ]);
        let catalog = ToolCatalog::from_toolset(&set, None);
        let descriptors = catalog.snapshot();
        assert_eq!(descriptors[0].namespace.as_deref(), Some("mcp:fs"));
        assert_ne!(descriptors[0].identifier, descriptors[1].identifier);
    }

    #[test]
    fn allowed_set_filters_catalog() {
        let set = ToolSet::from_iter(vec![
            tool("read", XyToolExposure::Direct),
            tool("write", XyToolExposure::Direct),
        ]);
        let allowed = BTreeSet::from(["read".to_string()]);
        let catalog = ToolCatalog::from_toolset(&set, Some(&allowed));
        assert!(catalog.describe("read").is_some());
        assert!(catalog.describe("write").is_none());
    }
}
