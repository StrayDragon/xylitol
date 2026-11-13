use xylitol_plugin_sdk::{XyToolDescriptor, XyToolExposure};

pub fn render_declarations(catalog: &[XyToolDescriptor], budget: usize) -> String {
    let mut out = String::new();
    let mut deferred = Vec::new();
    if !catalog.is_empty() {
        out.push_str(
            "run_code scripts run as the body of an async function: top-level await and return work; output channels are return, text(value), console.log(...).\n",
        );
    }
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
    fn describes_async_script_body() {
        let catalog = vec![descriptor("read", XyToolExposure::Direct)];
        let text = render_declarations(&catalog, 10_000);
        assert!(text.contains("body of an async function"), "{text}");
        assert!(text.contains("top-level await"), "{text}");
        assert!(text.contains("text(value)"), "{text}");
    }

    #[test]
    fn budget_falls_back_to_deferred() {
        let catalog = vec![descriptor("read", XyToolExposure::Direct)];
        let text = render_declarations(&catalog, 4);
        assert!(!text.contains("- read"));
        assert!(text.contains("Deferred tools: read"));
    }
}
