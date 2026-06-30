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
// Currently enforced:
//   1. infra/ must not import crate::agent (no reverse dependency)
//   2. agent/ production code must not import ANY concrete crate::infra type
//      (full scan of `crate::infra::`, per la2/la9). Pre-existing violations are
//      tracked in `AGENT_INFRA_ALLOWLIST` (c275 baseline snapshot); each entry
//      cites a follow-up change (c276/c277/c278) that will remove it. Any NEW
//      violation not in the allowlist fails the build. The allowlist shrinks as
//      those follow-ups land, tightening the guard toward zero exemptions.
//   3. app/ (except the documented seams: the cross-surface seam layer
//      app/core/{composition,driver}.rs and the composition-root surfaces
//      cli/, server/, rpc.rs) must not import crate::agent or crate::infra types.
//
// Known app exceptions (composition roots or documented seams):
//   - app/cli/ = composition root (wires ports + Agent); also hosts print.rs,
//     the CLI default render sub-mode (moved from top-level app/ in c310)
//   - app/rpc.rs = builds Agent via composition
//   - app/server/ = composition root (hosts Agent + infra runtimes) + server
//     lifecycle (subcommand.rs: Run/Install/Stop, moved from cli in c310)
//   - app/core/composition.rs = shared Agent construction (HC-1 root: the only
//     module permitted to import both agent and infra)
//   - app/core/driver.rs = Driver trait + InProcessDriver (must NOT import
//     crate::infra per la11)
//   - app/tui/diff_review/ = review engine (infra config import)

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

    /// Pre-existing agent→infra PRODUCTION imports (c275 baseline snapshot).
    ///
    /// Each entry: `(file relative to src/agent, infra import path, follow-up)`.
    /// A hit matching `(file, import)` is allowed; anything else fails the guard.
    /// Entries are removed as c276/c277/c278 clear the underlying coupling, so the
    /// guard tightens toward zero exemptions over time. Adding a new agent→infra
    /// production import requires either eliminating it or registering it here
    /// with a follow-up change id (error-on-new, warn-on-existing).
    const AGENT_INFRA_ALLOWLIST: &[(&str, &str, &str)] = &[
        // All prior exemptions (c276/c277/c278/c279) have been cleared.
        // agent/ production code must not import any crate::infra::* type;
        // any new violation fails the guard immediately (no allowlist entry).
    ];

    /// Extract `crate::infra::...` import tokens from the PRODUCTION region of each
    /// `.rs` file under `dir`. Production = everything before the first
    /// `#[cfg(test)] mod <name>` test MODULE; a `#[cfg(test)]` attribute on a
    /// single item (e.g. a gated `use`) does NOT end the production region. Test
    /// code may freely assemble concrete types (HC-1 targets production coupling
    /// only). Returns `(rel_path, import, line)`.
    fn scan_prod_infra(dir: &str) -> Vec<(String, String, usize)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let mut files = Vec::new();
        walk_rs_files(&root, &mut files);
        let mut hits = Vec::new();
        for path in &files {
            let Ok(content) = std::fs::read_to_string(path) else {
                continue;
            };
            let lines: Vec<&str> = content.lines().collect();
            // Find the test-MODULE boundary: first `#[cfg(test)]` whose next
            // non-empty line starts with `mod `. A `#[cfg(test)]` on a single
            // `use`/`fn`/`const` is a gated item, not the test region.
            let mut cfg_test_line = usize::MAX;
            let mut i = 0;
            while i < lines.len() {
                if lines[i].contains("#[cfg(test)]") {
                    // peek ahead for `mod `
                    let mut j = i + 1;
                    while j < lines.len() && lines[j].trim().is_empty() {
                        j += 1;
                    }
                    if j < lines.len() && lines[j].trim().starts_with("mod ") {
                        cfg_test_line = i;
                        break;
                    }
                }
                i += 1;
            }
            for (idx, raw) in lines.iter().enumerate() {
                if idx >= cfg_test_line {
                    break;
                }
                // strip line comments
                let line = raw.split("//").next().unwrap_or("");
                let mut rest = line;
                while let Some(start) = rest.find("crate::infra::") {
                    let after = &rest[start..];
                    // capture crate::infra::<lowercase segments joined by ::>
                    let token = after
                        .chars()
                        .take_while(|&c| c.is_ascii_lowercase() || c == '_' || c == ':')
                        .collect::<String>()
                        .trim_end_matches(':')
                        .to_string();
                    if token.len() > "crate::infra::".len() {
                        let rel = path
                            .strip_prefix(&root)
                            .unwrap_or(path)
                            .display()
                            .to_string();
                        hits.push((rel, token, idx + 1));
                    }
                    rest = &rest[start + "crate::infra::".len()..];
                }
            }
        }
        hits
    }

    #[test]
    fn agent_does_not_import_infra_in_production() {
        // HC-1 (la2/la9): agent/ must not import ANY concrete crate::infra type
        // in production code — full scan, not limited to four named providers.
        // Pre-existing violations are grandfathered in AGENT_INFRA_ALLOWLIST;
        // any NEW violation not listed here fails.
        let hits = scan_prod_infra("src/agent");
        let mut new_violations = Vec::new();
        for (rel, import, line) in &hits {
            let allowed = AGENT_INFRA_ALLOWLIST
                .iter()
                .any(|&(a_rel, a_import, _)| a_rel == rel.as_str() && a_import == import.as_str());
            if !allowed {
                new_violations.push(format!("{rel}:{line}: {import}"));
            }
        }
        assert!(
            new_violations.is_empty(),
            "agent/ production code imports crate::infra (HC-1 violation, not in allowlist):\n  {}
\
             Register it in AGENT_INFRA_ALLOWLIST with a follow-up change id, or eliminate the import.",
            new_violations.join("\n  "),
        );
    }

    #[test]
    fn app_only_from_driver() {
        // app/ (except composition roots and documented seams) must not import
        // crate::agent or crate::infra types directly.
        //
        // Scan results are paths relative to src/app/.
        // Composition-root surfaces (may import agent+infra):
        //   cli/     — CLI entry point (also hosts print.rs, the CLI render mode)
        //   server/  — HTTP/WebSocket server + lifecycle (subcommand.rs)
        //   rpc.rs   — stdio RPC transport
        // Cross-surface seams under app/core/ (the privileged seam layer):
        //   core/composition.rs — shared Agent construction (agent+infra root)
        //   core/driver.rs      — Driver trait + InProcessDriver (agent-only)
        // Other documented seam:
        //   tui/diff_review/ — review engine (infra config import)
        let exempt_prefixes = ["cli/", "server/", "core/", "tui/diff_review/"];
        let exempt_files = ["rpc.rs"];

        let mut violations = Vec::new();
        for needle in ["crate::agent", "crate::infra"] {
            for hit in scan("src/app", needle) {
                let is_exempt = exempt_prefixes.iter().any(|p| hit.starts_with(p))
                    || exempt_files.iter().any(|f| hit.starts_with(f));
                if !is_exempt {
                    violations.push(hit);
                }
            }
        }
        assert!(
            violations.is_empty(),
            "app/ (except composition roots and documented seams) must not import agent or infra — found violations:\n  {}",
            violations.join("\n  "),
        );
    }

    #[test]
    fn app_driver_does_not_import_infra() {
        // app/core/driver.rs is the Driver trait definition + InProcessDriver.
        // It must depend only on `agent` (mod-level) and runtime_protocol, never
        // on concrete infra types (the RemoteDriver half is feature-gated
        // under `server` and lives in the same file only for locality).
        let hits = scan("src/app/core/driver.rs", "crate::infra");
        assert!(
            hits.is_empty(),
            "app/core/driver.rs must not import crate::infra — found violations:\n  {}",
            hits.join("\n  "),
        );
    }
}
