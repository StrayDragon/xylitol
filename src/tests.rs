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
//   3. interactive/ (except the composition root + driver) must not import
//      crate::agent or crate::infra types.
//
// Known interactive exceptions (composition root or documented seams):
//   - interactive/cli/ = composition root (wires ports + Agent)
//   - interactive/driver.rs = Driver trait + InProcessDriver
//   - interactive/rpc.rs = builds Agent via with_ports
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

    /// Pre-existing agent→infra PRODUCTION imports (c275 baseline snapshot).
    ///
    /// Each entry: `(file relative to src/agent, infra import path, follow-up)`.
    /// A hit matching `(file, import)` is allowed; anything else fails the guard.
    /// Entries are removed as c276/c277/c278 clear the underlying coupling, so the
    /// guard tightens toward zero exemptions over time. Adding a new agent→infra
    /// production import requires either eliminating it or registering it here
    /// with a follow-up change id (error-on-new, warn-on-existing).
    const AGENT_INFRA_ALLOWLIST: &[(&str, &str, &str)] = &[
        // c276 — hoist shared vocabulary types to core/
        (
            "compaction/cut_detector.rs",
            "crate::infra::session::types",
            "c276",
        ),
        (
            "compaction/file_ops.rs",
            "crate::infra::session::types",
            "c276",
        ),
        (
            "compaction/message_converter.rs",
            "crate::infra::session::types",
            "c276",
        ),
        ("compaction/mod.rs", "crate::infra::session::types", "c276"),
        (
            "compaction/settings.rs",
            "crate::infra::config::types",
            "c276",
        ),
        (
            "compaction/settings.rs",
            "crate::infra::settings::types",
            "c276",
        ),
        ("prompt/commands.rs", "crate::infra::source_info", "c276"),
        ("prompt/skills.rs", "crate::infra::resource", "c276"),
        (
            "prompt/skills.rs",
            "crate::infra::skills::loader::xml_escape",
            "c276",
        ),
        ("prompt/system.rs", "crate::infra::resource", "c276"),
        (
            "prompt/system.rs",
            "crate::infra::skills::loader::xml_escape",
            "c276",
        ),
        ("prompt/templates.rs", "crate::infra::source_info", "c276"),
        (
            "session/events.rs",
            "crate::infra::event::lifecycle",
            "c276",
        ),
        ("session/export.rs", "crate::infra::session", "c276"),
        ("session/mod.rs", "crate::infra::event::lifecycle", "c276"),
        ("session/mod.rs", "crate::infra::resource", "c276"),
        (
            "session/steering.rs",
            "crate::infra::event::lifecycle",
            "c276",
        ),
        ("tools/definition.rs", "crate::infra::source_info", "c276"),
        // c277 — sink assembly to composition root
        (
            "compaction/mod.rs",
            "crate::infra::session::manager",
            "c277",
        ),
        (
            "compaction/orchestrator.rs",
            "crate::infra::session::manager",
            "c277",
        ),
        ("facade.rs", "crate::infra::session", "c277"),
        (
            "model/manager.rs",
            "crate::infra::provider::factory::build_provider",
            "c277",
        ),
        ("model/registry.rs", "crate::infra::config::value", "c277"),
        (
            "runtime/bash.rs",
            "crate::infra::process::shell::find_bash",
            "c277",
        ),
        (
            "runtime/bash.rs",
            "crate::infra::tools::accumulator",
            "c277",
        ),
        (
            "runtime/bash.rs",
            "crate::infra::tools::process::kill_tree",
            "c277",
        ),
        ("runtime/bash.rs", "crate::infra::tools::truncate", "c277"),
        ("runtime/react.rs", "crate::infra::sandbox", "c277"),
        ("session/events.rs", "crate::infra::event", "c277"),
        (
            "session/export.rs",
            "crate::infra::session::manager",
            "c277",
        ),
        ("session/mod.rs", "crate::infra::event", "c277"),
        ("session/mod.rs", "crate::infra::sandbox", "c277"),
        ("session/mod.rs", "crate::infra::session::manager", "c277"),
        // c278 — merge export forwarding layer
        (
            "session/export.rs",
            "crate::infra::session::export::parse_jsonl",
            "c278",
        ),
        (
            "session/export.rs",
            "crate::infra::session::export::render_html",
            "c278",
        ),
        (
            "session/export.rs",
            "crate::infra::session::export::render_jsonl",
            "c278",
        ),
        (
            "session/export.rs",
            "crate::infra::session::export::share_guidance_message",
            "c278",
        ),
        (
            "session/export.rs",
            "crate::infra::session::export::write_to",
            "c278",
        ),
    ];

    /// Extract `crate::infra::...` import tokens from the PRODUCTION region of each
    /// `.rs` file under `dir`. Production = everything before the first
    /// `#[cfg(test)]` line; test code may freely assemble concrete types (HC-1
    /// targets production coupling only). Returns `(rel_path, import, line)`.
    fn scan_prod_infra(dir: &str) -> Vec<(String, String, usize)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let mut files = Vec::new();
        walk_rs_files(&root, &mut files);
        let mut hits = Vec::new();
        for path in &files {
            let Ok(content) = std::fs::read_to_string(path) else {
                continue;
            };
            let cfg_test_line = content
                .lines()
                .position(|l| l.contains("#[cfg(test)]"))
                .unwrap_or(usize::MAX);
            for (i, raw) in content.lines().enumerate() {
                if i >= cfg_test_line {
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
                        hits.push((rel, token, i + 1));
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
    fn interactive_only_from_driver() {
        // interactive/ (except cli/ and driver.rs) must not import
        // crate::agent or crate::infra types directly.
        //
        // Known exceptions documented above (cli=composition root,
        // driver.rs=Driver trait def). The intent is to prevent new
        // interactive modules from leaking agent/infra internals.
        // Scan results are paths relative to src/interactive/.
        // Allowed: cli/ (composition root) and driver.rs (Driver trait).
        // Documented seams (not composition root, but pinned exceptions):
        //   resources.rs — read-only resource listing
        //   rpc.rs — builds Agent via with_ports
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
