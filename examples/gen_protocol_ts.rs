//! Write (or print) specta TypeScript bindings for the product envelope.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() {
    let bindings = xylitol::protocol::export_typescript_bindings().unwrap_or_else(|e| {
        eprintln!("error: specta export failed: {e}");
        std::process::exit(1);
    });

    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--stdout") => {
            let mut out = io::stdout().lock();
            out.write_all(bindings.as_bytes()).unwrap();
        }
        Some(path) => write_file(PathBuf::from(path), &bindings),
        None => {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(xylitol::protocol::BINDINGS_RELATIVE_PATH);
            write_file(path, &bindings);
        }
    }
}

fn write_file(path: PathBuf, bindings: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("error: create {}: {e}", parent.display());
            std::process::exit(1);
        });
    }
    fs::write(&path, bindings).unwrap_or_else(|e| {
        eprintln!("error: write {}: {e}", path.display());
        std::process::exit(1);
    });
    eprintln!("wrote {}", path.display());
}
