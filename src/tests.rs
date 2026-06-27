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
//   3. interactive/ (except cli/ and driver.rs) must not import
//      crate::agent or crate::infra types.
//
// Known exceptions (all documented with c270 rationale, tracked for cleanup):
//   - interactive/cli/ = composition root (wires ports + Agent)
//   - interactive/driver.rs = Driver trait + InProcessDriver
//   - interactive/rpc.rs = builds Agent via with_ports (T2 c270)
//   - interactive/print.rs = imports AgentEvent for stream matching
//   - interactive/resources.rs = read-only resource listing
//   - interactive/diff_review/ = review engine (infra config import)

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

    #[test]
    fn interactive_only_from_driver() {
        // interactive/ (except cli/ and driver.rs) must not import
        // crate::agent or crate::infra types directly.
        //
        // Known exceptions documented above (cli=composition root,
        // driver.rs=Driver trait def). The intent is to prevent new
        // interactive modules from leaking agent/infra internals.
        // Scan results are paths relative to src/interactive/.
        // Allowed: cli/ (composition root) and driver.rs (Driver trait).
        // Known exceptions (not yet refactored, tracked by c270):
        //   resources.rs — read-only resource listing
        //   rpc.rs — builds Agent via with_ports (T2 c270)
        //   print.rs — imports AgentEvent for stream matching
        let exempt_prefixes = ["cli/", "diff_review/"];
        let exempt_files = ["driver.rs", "resources.rs", "rpc.rs", "print.rs"];

        let mut violations = Vec::new();
        for needle in ["crate::agent", "crate::infra"] {
            for hit in scan("src/interactive", needle) {
                let is_exempt = exempt_prefixes.iter().any(|p| hit.starts_with(p))
                    || exempt_files.iter().any(|f| hit.starts_with(f));
                if !is_exempt {
                    violations.push(hit);
                }
            }
        }
        assert!(
            violations.is_empty(),
            "interactive/ (except cli/ and driver.rs) must not import agent or infra — found violations:\n  {}",
            violations.join("\n  "),
        );
    }
}
