use std::collections::BTreeSet;
use std::sync::Arc;

use xylitol_plugin_sdk::{XyTool, XyToolDescriptor, XyToolExposure};

pub fn descriptors_from_tools(tools: &[(String, Arc<dyn XyTool>)]) -> Vec<XyToolDescriptor> {
    let policy = xylitol_plugin_sdk::codemode_exposure_policy();
    let overrides = xylitol_plugin_sdk::codemode_exposure_overrides();
    let mut tools: Vec<_> = tools
        .iter()
        .map(|(_, tool)| tool)
        .filter(|tool| {
            overrides
                .get(tool.name())
                .copied()
                .unwrap_or_else(|| tool.exposure())
                .codemode_visible()
        })
        .filter(|tool| {
            policy
                .as_ref()
                .is_none_or(|policy| policy.allows(tool.name()))
        })
        .collect();
    tools.sort_by(|left, right| left.name().cmp(right.name()));
    let mut used = BTreeSet::new();
    tools
        .into_iter()
        .map(|tool| {
            let identifier = unique_identifier(tool.name(), &mut used);
            let exposure = overrides
                .get(tool.name())
                .copied()
                .unwrap_or_else(|| tool.exposure());
            XyToolDescriptor {
                name: tool.name().to_string(),
                identifier,
                namespace: tool.namespace().map(str::to_string),
                description: tool.description().to_string(),
                parameters: tool.parameters_schema(),
                output_schema: tool.output_schema(),
                exposure,
                guidelines: tool
                    .prompt_guidelines()
                    .iter()
                    .map(|g| (*g).to_string())
                    .collect(),
            }
        })
        .collect()
}

fn unique_identifier(name: &str, used: &mut BTreeSet<String>) -> String {
    let mut out: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push('_');
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    let base = out.clone();
    let mut suffix = 2;
    while !used.insert(out.clone()) {
        out = format!("{base}_{suffix}");
        suffix += 1;
    }
    out
}

pub fn render_declarations(catalog: &[XyToolDescriptor], budget: usize) -> String {
    let mut out = String::new();
    let mut deferred = Vec::new();
    for tool in catalog {
        if tool.exposure == XyToolExposure::Deferred {
            deferred.push(tool.name.as_str());
            continue;
        }
        if !tool.exposure.codemode_visible() {
            continue;
        }
        let line = format!(
            "- {}: {} (parameters: {})\n",
            tool.identifier, tool.description, tool.parameters
        );
        if out.len() + line.len() > budget {
            deferred.push(tool.name.as_str());
            continue;
        }
        out.push_str(&line);
    }
    if !deferred.is_empty() {
        out.push_str(&format!(
            "Deferred tools: {}. Use ALL_TOOLS, searchTools(query), or describeTool(name).\n",
            deferred.join(", ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn descriptor(name: &str, exposure: XyToolExposure) -> XyToolDescriptor {
        XyToolDescriptor {
            name: name.into(),
            identifier: name.into(),
            namespace: None,
            description: "desc".into(),
            parameters: json!({"type": "object"}),
            output_schema: None,
            exposure,
            guidelines: Vec::new(),
        }
    }

    #[test]
    fn deferred_tools_are_not_inlined() {
        let catalog = vec![
            descriptor("read", XyToolExposure::Direct),
            descriptor("later", XyToolExposure::Deferred),
        ];
        let text = render_declarations(&catalog, 10_000);
        assert!(text.contains("read"));
        assert!(!text.contains("- later"));
        assert!(text.contains("Deferred tools: later"));
    }

    #[test]
    fn budget_falls_back_to_deferred() {
        let catalog = vec![descriptor("read", XyToolExposure::Direct)];
        let text = render_declarations(&catalog, 4);
        assert!(!text.contains("- read"));
        assert!(text.contains("Deferred tools: read"));
    }
}
