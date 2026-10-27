//! Regenerate `configs/config.schema.json` from `schema_for!(AppConfig)`.
//!
//! Maintenance entry (`just gen-config-schema`); the drift gate lives in
//! `validate` unit tests, not here.

use std::path::PathBuf;

use xylitol_official_plugin_config::validate::render_app_config_schema;

fn main() {
    let target = repo_root().join("configs").join("config.schema.json");
    let rendered = render_app_config_schema();
    if let Err(e) = std::fs::write(&target, rendered.as_bytes()) {
        eprintln!("error: write {}: {e}", target.display());
        std::process::exit(1);
    }
    println!("wrote {}", target.display());
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("plugin lives at <repo>/plugins/<name>")
        .to_path_buf()
}
