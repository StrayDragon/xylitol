//! Sandbox routing — extract the sandbox-relevant target from tool arguments.
//!
//! The ReAct loop calls [`sandbox_target`] before executing a tool so the
//! sandbox engine can veto dangerous paths/domains. Kept separate from the
//! loop so the loop file stays focused on the turn algorithm.

use serde_json::Value;

/// Extract the sandbox-relevant target (path or domain) from tool arguments.
pub(crate) fn sandbox_target(name: &str, args: &Value) -> String {
    match name {
        "read" | "write" | "edit" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "bash" => {
            // Extract the first URL domain from the command, if any.
            args.get("command")
                .and_then(|v| v.as_str())
                .map(extract_first_domain)
                .unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// Extract the first domain from a shell command containing a URL.
fn extract_first_domain(command: &str) -> String {
    // Look for common URL patterns: https://, http://.
    for word in command.split_whitespace() {
        let word = word.trim_matches('\'').trim_matches('"');
        if let Some(rest) = word
            .strip_prefix("https://")
            .or_else(|| word.strip_prefix("http://"))
        {
            // Extract domain (stop at first /, :, or ?).
            let domain = rest.split(['/', ':', '?']).next().unwrap_or(rest);
            return domain.to_string();
        }
    }
    String::new()
}
