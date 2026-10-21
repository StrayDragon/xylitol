use serde_json::Value;

use xylitol_plugin_sdk::{XyPermission, XyPermissionVerdict};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SandboxCapability {
    Read,
    Write,
    Network,
}

pub(crate) fn sandbox_capability(tool_name: &str) -> Option<SandboxCapability> {
    match tool_name {
        "read" => Some(SandboxCapability::Read),
        "write" | "edit" => Some(SandboxCapability::Write),
        "bash" => Some(SandboxCapability::Network),
        _ => None,
    }
}

pub(crate) fn permission_target(name: &str, args: &Value) -> String {
    match sandbox_capability(name) {
        Some(SandboxCapability::Read | SandboxCapability::Write) => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        Some(SandboxCapability::Network) => args
            .get("command")
            .and_then(|v| v.as_str())
            .map(extract_first_domain)
            .unwrap_or_default(),
        None => String::new(),
    }
}

pub(crate) fn check_tool_permission(
    engine: &dyn XyPermission,
    tool_name: &str,
    target: &str,
) -> Option<String> {
    let verdict = match sandbox_capability(tool_name)? {
        SandboxCapability::Read => engine.check_read(target),
        SandboxCapability::Write => engine.check_write(target),
        SandboxCapability::Network => engine.check_network(target),
    };
    match verdict {
        XyPermissionVerdict::Deny { reason } => Some(reason),
        _ => None,
    }
}

fn extract_first_domain(command: &str) -> String {
    for word in command.split_whitespace() {
        let word = word.trim_matches('\'').trim_matches('"');
        if let Some(rest) = word
            .strip_prefix("https://")
            .or_else(|| word.strip_prefix("http://"))
        {
            let domain = rest.split(['/', ':', '?']).next().unwrap_or(rest);
            return domain.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn capability_covers_sandbox_builtins() {
        assert_eq!(sandbox_capability("read"), Some(SandboxCapability::Read));
        assert_eq!(sandbox_capability("write"), Some(SandboxCapability::Write));
        assert_eq!(sandbox_capability("edit"), Some(SandboxCapability::Write));
        assert_eq!(sandbox_capability("bash"), Some(SandboxCapability::Network));
        assert_eq!(sandbox_capability("ask"), None);
    }

    #[test]
    fn target_uses_path_or_domain() {
        assert_eq!(
            permission_target("read", &json!({ "path": "/tmp/a" })),
            "/tmp/a"
        );
        assert_eq!(
            permission_target("bash", &json!({ "command": "curl https://example.com/x" })),
            "example.com"
        );
        assert_eq!(permission_target("ask", &json!({ "prompt": "hi" })), "");
    }
}
