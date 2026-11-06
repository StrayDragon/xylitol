use std::path::Path;

use crate::cli::args::ConfigAction;
use xylitol_kernel::config::loader::load_app_config_detailed;

pub fn run(action: ConfigAction) -> Result<(), String> {
    match action {
        ConfigAction::Schema { domain, ts: _ } => {
            let schema = xylitol_kernel::config::validate::render_app_config_schema();
            if let Some(domain) = domain {
                let mut value: serde_json::Value =
                    serde_json::from_str(&schema).map_err(|e| e.to_string())?;
                let properties = value
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .cloned()
                    .unwrap_or_default();
                let filtered: serde_json::Map<String, serde_json::Value> = properties
                    .into_iter()
                    .filter(|(key, _)| key == &domain)
                    .collect();
                value["properties"] = serde_json::Value::Object(filtered);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
                );
            } else {
                print!("{schema}");
            }
            Ok(())
        }
        ConfigAction::Check { path, warn } => {
            let loaded = load_app_config_detailed(path.as_deref().map(Path::new))
                .map_err(|e| e.to_string())?;
            let unknown = loaded.config.unknown_keys.clone();
            let codemode: xylitol_plugin_sdk::CodemodeConfig = loaded
                .config
                .section_or_default("codemode")
                .unwrap_or_default();
            let tools: Vec<String> = loaded
                .config
                .section_or_default("tools")
                .unwrap_or_default();
            let mut warnings = Vec::new();
            if !codemode.is_active() && xylitol_plugin_sdk::run_code_selected(&tools) {
                warnings.push("codemode disabled; run_code selection ignored".to_string());
            }
            for warning in &warnings {
                eprintln!("{warning}");
            }
            if unknown.is_empty() {
                if warnings.is_empty() {
                    println!("config ok");
                } else {
                    println!("config ok (warnings)");
                }
                return Ok(());
            }
            let message = format!("unknown config keys: {}", unknown.join(", "));
            if warn {
                eprintln!("{message}");
                println!("config ok (warnings)");
                Ok(())
            } else {
                Err(message)
            }
        }
        ConfigAction::Show { resolved: _ } => {
            let loaded = load_app_config_detailed(None).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&loaded.config).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        ConfigAction::Init { preset: _, force } => {
            let path = xylitol_kernel::config::paths::ConfigPaths::discover().global_config_path();
            if path.exists() && !force {
                return Err(format!(
                    "{} already exists; pass --force to overwrite",
                    path.display()
                ));
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(
                &path,
                "xylitol:\n  schema_version: 1\n  unknown_keys: error\n",
            )
            .map_err(|e| e.to_string())?;
            println!("wrote {}", path.display());
            Ok(())
        }
    }
}
