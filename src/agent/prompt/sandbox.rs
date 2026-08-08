//! Sandboxed minijinja for default system-prompt assembly (c1220 / pt5).
//!
//! Strict undefined, whitelist context only, pre-registered templates — no
//! filesystem loader and no `env`/`secret` namespaces (unlike config YAML).

use minijinja::{AutoEscape, Environment, UndefinedBehavior, context, value::Value as MjValue};
use serde::Serialize;

const DEFAULT_TEMPLATE: &str = "default_system.j2";
const MCP_DISCOVER: &str = "MCP/custom tools are provided in this turn's tools list — call by exact name \
(see `/mcp` in the product TUI for connection status).";

#[derive(Debug, Clone, Serialize)]
struct ToolRow {
    name: String,
    snippet: String,
}

/// Build a sandboxed Environment with only embedded prompt templates registered.
fn sandbox_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    // No `set_loader` — includes resolve only against `add_template` names.
    env.add_template(
        DEFAULT_TEMPLATE,
        include_str!("templates/default_system.j2"),
    )
    .expect("embed default_system.j2");
    env
}

/// Render the default system base (intro + Available tools + MCP discover).
///
/// `tools` must already be filtered (no MCP names). Failures are programming
/// errors (bad embed / ctx shape) and panic in production paths after expect —
/// unit tests cover strict/unknown-include via [`try_render_default`].
pub(crate) fn render_default_base(tools: &[(String, String)]) -> String {
    try_render_default(tools).expect("default system template render")
}

fn try_render_default(tools: &[(String, String)]) -> Result<String, minijinja::Error> {
    let rows: Vec<ToolRow> = tools
        .iter()
        .map(|(name, snippet)| ToolRow {
            name: name.clone(),
            snippet: snippet.clone(),
        })
        .collect();
    let env = sandbox_env();
    let tmpl = env.get_template(DEFAULT_TEMPLATE)?;
    let ctx = context! {
        tools => MjValue::from_serialize(&rows),
        mcp_discover => MCP_DISCOVER,
    };
    tmpl.render(ctx)
}

/// Test-only: render an ad-hoc body against the sandbox (strict + no loader).
#[cfg(test)]
pub(crate) fn try_render_str(
    name: &str,
    source: &str,
    ctx: minijinja::value::Value,
) -> Result<String, minijinja::Error> {
    let mut env = sandbox_env();
    env.add_template(name, source)?;
    env.get_template(name)?.render(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_lists_tools_and_discover() {
        let out = render_default_base(&[
            ("read".into(), "Read file".into()),
            ("bash".into(), "Run bash".into()),
        ]);
        assert!(out.contains("You are an expert coding assistant"));
        assert!(out.contains("- read: Read file"));
        assert!(out.contains("- bash: Run bash"));
        assert!(out.contains("MCP/custom tools are provided"));
        assert!(!out.contains("mcp_"));
        assert!(!out.contains("mcp-"));
        assert!(!out.contains("mcp__"));
        assert!(!out.contains("mcp:"));
    }

    #[test]
    fn default_base_none_when_empty_tools() {
        let out = render_default_base(&[]);
        assert!(out.contains("(none)"));
    }

    #[test]
    fn strict_unknown_key_fails() {
        let err = try_render_str("t.j2", "Hello {{ nosuch }}", context! {}).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("nosuch") || msg.to_lowercase().contains("undefined"),
            "{msg}"
        );
    }

    #[test]
    fn unknown_include_fails_without_loader() {
        let err = try_render_str("t.j2", r#"{% include "missing.j2" %}"#, context! {}).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("missing") || msg.contains("not found") || msg.contains("template"),
            "{msg}"
        );
    }
}
