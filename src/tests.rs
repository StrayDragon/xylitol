// Unit-test-only module wiring.
//
// Shared test infrastructure lives under `tests/support/` so we can treat it like a lightweight
// "test-support crate" in this single-crate repository.

#[path = "../tests/support/mod.rs"]
pub mod support;

// ── Architecture guards ───────────────────────────────────────────
//
// These tests grep source files to enforce layering invariants (HC-1).
// They codify wins from the c260 refactor so regressions fail the build.
//
// Currently enforced (all green):
//   1. infra/ must not import crate::agent (no reverse dependency)
//   2. agent/ must not import concrete provider implementations
//      (infra::provider::{openai,anthropic,fake,mock}) — agent holds
//      providers only as `Arc<dyn XyModel>` via the factory.
//
// Known exceptions not yet guarded (TODO):
//   - interactive/{cli,rpc,resources} still import agent/infra types
//     (cli=composition root, rpc being migrated to Driver, resources=read-only)
//   - interactive/print.rs is clean (only facade::AgentEvent)

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

    /// Scan `dir` for lines matching `needle`, returning "relpath:line: trim" strings.
    fn scan(dir: &str, needle: &str) -> Vec<String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let mut files = Vec::new();
        walk_rs_files(&root, &mut files);
        let mut hits = Vec::new();
        for path in &files {
            if let Ok(content) = std::fs::read_to_string(path) {
                for (i, line) in content.lines().enumerate() {
                    if line.contains(needle) {
                        hits.push(format!(
                            "{}:{}: {}",
                            path.strip_prefix(&root).unwrap_or(path).display(),
                            i + 1,
                            line.trim(),
                        ));
                    }
                }
            }
        }
        hits
    }

    #[test]
    fn infra_does_not_import_agent() {
        let violations = scan("src/infra", "crate::agent");
        assert!(
            violations.is_empty(),
            "infra/ must not depend on agent/ — found violations:\n  {}",
            violations.join("\n  "),
        );
    }

    #[test]
    fn agent_does_not_import_concrete_providers() {
        // agent must hold providers as Arc<dyn XyModel> only; naming a concrete
        // provider impl (openai/anthropic/fake/mock) is an HC-1 violation.
        let mut violations = Vec::new();
        for concrete in [
            "infra::provider::openai",
            "infra::provider::anthropic",
            "infra::provider::fake",
            "infra::provider::mock",
        ] {
            violations.extend(scan("src/agent", concrete));
        }
        assert!(
            violations.is_empty(),
            "agent/ must not import concrete provider implementations — found violations:\n  {}",
            violations.join("\n  "),
        );
    }
}
