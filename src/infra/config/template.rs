//! MiniJinja-based template rendering for config files.
//!
//! Supports `{{ env.KEY }}`, `{{ secret.KEY }}`, and `| default("val")`.

#![allow(dead_code)]
use std::collections::HashMap;

use minijinja::UndefinedBehavior;

/// Errors that can occur during template rendering.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TemplateError {
    #[error("template error in config file: {detail}")]
    Parse { detail: String },
    #[error(
        "missing variable `{key}` in template (no default). \
         Edit the config file or set the corresponding env/secret."
    )]
    MissingVar { key: String },
}

/// Pre-process a YAML string through template rendering.
///
/// `env_vars` and `secret_vars` provide the `{{ env.* }}` and `{{ secret.* }}`
/// namespaces respectively.
pub(crate) fn render(
    source: &str,
    env_vars: &HashMap<String, String>,
    secret_vars: &HashMap<String, String>,
) -> Result<String, TemplateError> {
    let mut env = minijinja::Environment::new();

    // Strict mode: any undefined variable causes an immediate error.
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    // Add `default(alt)` filter: if the value is undefined, fall back to alt.
    env.add_filter(
        "default",
        |v: Option<minijinja::Value>, alt: minijinja::Value| v.unwrap_or(alt),
    );

    // Build namespace values from env/secrets.
    let env_ns = minijinja::Value::from_serialize(env_vars);
    let secret_ns = minijinja::Value::from_serialize(secret_vars);

    // Add the namespaces into the context.
    let mut root = HashMap::new();
    root.insert("env", env_ns);
    root.insert("secret", secret_ns);

    let tmpl = env
        .template_from_str(source)
        .map_err(|e| TemplateError::Parse {
            detail: e.to_string(),
        })?;

    let rendered = tmpl.render(root).map_err(|e| {
        let msg = e.to_string();
        // Report as MissingVar if it looks like an undefined variable.
        if msg.contains("is undefined")
            || msg.contains("is not defined")
            || msg.contains("undefined value")
        {
            let key = msg
                .lines()
                .next()
                .unwrap_or(&msg)
                .split(['`', '\''])
                .nth(1)
                .unwrap_or(&msg)
                .to_string();
            TemplateError::MissingVar { key }
        } else {
            TemplateError::Parse { detail: msg }
        }
    })?;

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_env_var() {
        let mut env = HashMap::new();
        env.insert("MODEL".into(), "gpt-4o".into());
        let secret = HashMap::new();
        let result = render("model: {{ env.MODEL }}", &env, &secret).unwrap();
        assert_eq!(result, "model: gpt-4o");
    }

    #[test]
    fn test_render_secret_var() {
        let env = HashMap::new();
        let mut secret = HashMap::new();
        secret.insert("API_KEY".into(), "sk-test".into());
        let result = render("api_key: {{ secret.API_KEY }}", &env, &secret).unwrap();
        assert_eq!(result, "api_key: sk-test");
    }

    #[test]
    fn test_render_default_filter() {
        let env = HashMap::new();
        let secret = HashMap::new();
        let result = render(
            "model: {{ env.MISSING | default(\"claude-3\") }}",
            &env,
            &secret,
        )
        .unwrap();
        assert_eq!(result, "model: claude-3");
    }

    #[test]
    fn test_render_missing_var_error() {
        let env = HashMap::new();
        let secret = HashMap::new();
        let err = render("model: {{ env.NONEXISTENT }}", &env, &secret).unwrap_err();
        match &err {
            TemplateError::MissingVar { key } => {
                assert!(!key.is_empty(), "expected non-empty key, got error: {err}");
            }
            _ => panic!("expected MissingVar, got {err:?}"),
        }
    }

    #[test]
    fn test_render_mixed_namespaces() {
        let mut env = HashMap::new();
        env.insert("MODEL".into(), "gpt-4o".into());
        let mut secret = HashMap::new();
        secret.insert("API_KEY".into(), "sk-456".into());
        let result = render(
            "model: {{ env.MODEL }}\napi_key: {{ secret.API_KEY }}",
            &env,
            &secret,
        )
        .unwrap();
        assert_eq!(result, "model: gpt-4o\napi_key: sk-456");
    }
}
