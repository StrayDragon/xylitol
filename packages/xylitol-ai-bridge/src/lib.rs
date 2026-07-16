//! xylitol-ai-bridge — client→LLM provider wiring + token accounting.
pub mod accounting;
pub mod dto;
pub mod error;
pub mod fake;

pub use fake::{AiBridgeModel, FakeProvider, FakeProviderBuilder, FakeProviderMode, ScenarioStep};
pub mod hooks;
pub mod provider;
pub mod registry;
pub mod thinking;
pub mod tokenize;
pub mod usage;

pub use dto::*;
pub use error::AiBridgeError;
pub use thinking::{
    AiBridgeGenerateOptions, AiBridgeResolvedThinking, AiBridgeThinkingAdapterKind,
    AiBridgeThinkingBudgets, apply_thinking_anthropic, apply_thinking_openai_completions,
    apply_thinking_openai_responses, resolve_from_options, resolve_thinking_for_request,
};

#[cfg(test)]
mod boundary_tests {
    use std::path::PathBuf;

    /// Compile-time guard: this package must not depend on the main `xylitol` crate.
    #[test]
    fn package_does_not_depend_on_main_crate() {
        let manifest = include_str!("../Cargo.toml");
        assert!(
            !manifest.contains("xylitol ="),
            "xylitol-ai-bridge must not depend on the main xylitol crate"
        );
    }

    fn walk_rs(dir: PathBuf, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs(path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Metering side must not import provider (HTTP/SSE) modules.
    #[test]
    fn accounting_tokenize_registry_do_not_import_provider() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let guarded = ["accounting", "tokenize", "registry"];
        let mut violations = Vec::new();
        for dir_name in guarded {
            let mut files = Vec::new();
            walk_rs(root.join(dir_name), &mut files);
            for path in files {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (i, line) in content.lines().enumerate() {
                    let t = line.trim();
                    if t.starts_with("//") {
                        continue;
                    }
                    if t.contains("crate::provider") || t.contains("super::provider") {
                        violations.push(format!(
                            "{}:{}: {}",
                            path.strip_prefix(&root).unwrap_or(&path).display(),
                            i + 1,
                            t
                        ));
                    }
                }
            }
        }
        assert!(
            violations.is_empty(),
            "accounting/tokenize/registry must not import provider:\n  {}",
            violations.join("\n  ")
        );
    }
}
