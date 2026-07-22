//! YAML config template rendering — `{{ env.KEY }}` / `{{ secret.KEY }}` /
//! `{{ vars.home }}` (r4/r5 / rc23).
//!
//! Uses minijinja with **strict** undefined behavior so missing keys fail with
//! a path-aware error (edit the YAML or secret.env).

use std::collections::HashMap;
use std::path::Path;

use minijinja::{AutoEscape, Environment, UndefinedBehavior, context, value::Value as MjValue};

use super::secret_env::SecretMap;

/// Render a config file body through minijinja.
///
/// Namespaces:
/// - `env.*` — process environment (after secret.env injection)
/// - `secret.*` — keys from collected `secret.env` files (plus same key from
///   process env when present, so shell overrides still work in templates)
/// - `vars.home` — user home directory (`dirs::home_dir`); **only** this key
///   under `vars` (rc23). Other `vars.*` keys are undefined (strict fail).
pub(crate) fn render_config_template(
    raw: &str,
    path: &Path,
    secrets: &SecretMap,
) -> Result<String, String> {
    // Fast path: no mustache — skip engine (common for plain YAML).
    if !raw.contains("{{") {
        return Ok(raw.to_string());
    }

    let env_map: HashMap<String, String> = std::env::vars().collect();
    // secret.* = secret.env map; process env wins for the same keys.
    let mut secret_ns = secrets.clone();
    for k in secrets.keys() {
        if let Ok(v) = std::env::var(k) {
            secret_ns.insert(k.clone(), v);
        }
    }

    // vars.* — only `home` when resolvable; missing home → undefined (strict
    // fails only if the template references vars.home).
    let mut vars_ns = HashMap::new();
    if let Some(home) = dirs::home_dir() {
        vars_ns.insert("home".to_string(), home.to_string_lossy().into_owned());
    }

    let mut jinja = Environment::new();
    jinja.set_undefined_behavior(UndefinedBehavior::Strict);
    // Default minijinja auto-escape treats `*.yaml`/`*.yml` as JSON → string
    // interpolations gain extra quotes (`"{{ secret.K }}"` → `""value""`), which
    // breaks YAML parse and MCP headers. Config templates are plain text.
    jinja.set_auto_escape_callback(|_| AutoEscape::None);

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("config.yaml");
    jinja
        .add_template(name, raw)
        .map_err(|e| format_template_error(path, &e))?;

    let tmpl = jinja
        .get_template(name)
        .map_err(|e| format_template_error(path, &e))?;

    let ctx = context! {
        env => MjValue::from_serialize(&env_map),
        secret => MjValue::from_serialize(&secret_ns),
        vars => MjValue::from_serialize(&vars_ns),
    };

    tmpl.render(ctx)
        .map_err(|e| format_template_error(path, &e))
}

fn format_template_error(path: &Path, err: &minijinja::Error) -> String {
    format!(
        "template error in {}: {err}\n  Hint: set the missing key in the environment, \
         or in secret.env next to this config, then reference {{{{ env.KEY }}}} / \
         {{{{ secret.KEY }}}}; path home is {{{{ vars.home }}}} only (no other vars.*).",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn renders_env_and_secret() {
        let _guard = ENV_LOCK.lock().unwrap();
        let path = PathBuf::from("config.yaml");
        unsafe {
            std::env::set_var("XYLITOL_TEST_TMPL_ENV", "from-env");
        }
        let mut secrets = SecretMap::new();
        secrets.insert("XYLITOL_TEST_TMPL_SECRET".into(), "from-secret".into());

        let out = render_config_template(
            "a: {{ env.XYLITOL_TEST_TMPL_ENV }}\nb: {{ secret.XYLITOL_TEST_TMPL_SECRET }}\n",
            &path,
            &secrets,
        )
        .unwrap();
        assert!(out.contains("from-env"), "{out}");
        assert!(out.contains("from-secret"), "{out}");
        unsafe {
            std::env::remove_var("XYLITOL_TEST_TMPL_ENV");
        }
    }

    #[test]
    fn missing_secret_is_strict_error() {
        let path = PathBuf::from("cfg.yaml");
        let err = render_config_template(
            "key: {{ secret.DOES_NOT_EXIST_XYZ }}\n",
            &path,
            &SecretMap::new(),
        )
        .unwrap_err();
        assert!(err.contains("cfg.yaml"), "{err}");
        assert!(err.contains("secret.env") || err.contains("Hint"), "{err}");
    }

    #[test]
    fn plain_yaml_skips_engine() {
        let out =
            render_config_template("models: {}\n", Path::new("x.yaml"), &SecretMap::new()).unwrap();
        assert_eq!(out, "models: {}\n");
    }

    #[test]
    fn renders_inside_yaml_double_quotes() {
        let _guard = ENV_LOCK.lock().unwrap();
        let path = PathBuf::from("config.yaml");
        let mut secrets = SecretMap::new();
        secrets.insert("CONTEXT7_API_KEY".into(), "sk-test".into());
        let out = render_config_template(
            "headers:\n  CONTEXT7_API_KEY: \"{{ secret.CONTEXT7_API_KEY }}\"\n",
            &path,
            &secrets,
        )
        .unwrap();
        assert_eq!(out, "headers:\n  CONTEXT7_API_KEY: \"sk-test\"");
    }

    #[test]
    fn renders_vars_home() {
        let home = dirs::home_dir().expect("home_dir for test");
        let home_str = home.to_string_lossy();
        let out = render_config_template(
            "command: \"{{ vars.home }}/.cargo/bin/lspz\"\n",
            Path::new("config.yaml"),
            &SecretMap::new(),
        )
        .unwrap();
        assert_eq!(out, format!("command: \"{home_str}/.cargo/bin/lspz\""));
    }

    #[test]
    fn unknown_vars_key_is_strict_error() {
        let err = render_config_template(
            "path: {{ vars.project }}\n",
            Path::new("cfg.yaml"),
            &SecretMap::new(),
        )
        .unwrap_err();
        assert!(err.contains("cfg.yaml"), "{err}");
        assert!(
            err.contains("vars.home") || err.contains("undefined") || err.contains("project"),
            "{err}"
        );
    }
}
