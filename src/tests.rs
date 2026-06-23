// Unit-test-only module wiring.
//
// Shared test infrastructure lives under `tests/support/` so we can treat it like a lightweight
// "test-support crate" in this single-crate repository.

#[path = "../tests/support/mod.rs"]
pub mod support;

// ── Architecture guard: infra must not depend on agent ────────────
//
// This test grep-checks that no file under `src/infra/` imports anything
// from `crate::agent` — a dependency direction violation.

#[cfg(test)]
mod arch_guard {
    /// Walk a directory recursively using std::fs (no external dep).
    fn walk_rs_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_rs_files(&path, files);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    files.push(path);
                }
            }
        }
    }

    #[test]
    fn infra_does_not_import_agent() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/infra");
        let mut files = Vec::new();
        walk_rs_files(&src_dir, &mut files);

        let mut violations: Vec<String> = Vec::new();

        for path in &files {
            if let Ok(content) = std::fs::read_to_string(path) {
                for (i, line) in content.lines().enumerate() {
                    if line.contains("crate::agent") {
                        violations.push(format!(
                            "{}:{}: {}",
                            path.strip_prefix(&src_dir).unwrap_or(path).display(),
                            i + 1,
                            line.trim(),
                        ));
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "infra/ must not depend on agent/ — found violations:\n  {}",
            violations.join("\n  "),
        );
    }
}
