use std::path::PathBuf;

fn main() {
    let target = repo_root().join("configs").join("config.schema.json");
    let rendered = xylitol_official_plugin_config::validate::render_app_config_schema(
        &xylitol_kernel::config_sections::all(),
    );
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
        .expect("kernel lives at <repo>/crates/xylitol-kernel")
        .to_path_buf()
}
