//! Regression baseline for the `Agent` public API surface (spec as31 / c320).
//!
//! This is NOT behavioral coverage. It snapshots the *normalized textual
//! signature list* of every `pub` item exposed by `agent::session`. After the
//! c255 session refactor (extraction of collaborators + module reorg), this
//! snapshot MUST remain byte-identical — directly verifying spec as31
//! ("facade-retained": public API unchanged).
//!
//! Mechanism: at test time we read `src/agent/session.rs`, extract every line
//! beginning a `pub` item, normalize trailing args, sort, and compare against
//! the accepted insta snapshot. A signature change fails the test until the
//! snapshot is intentionally reviewed and accepted.
//!
//! Note: this deliberately reads the SINGLE source file `session.rs`. Once c255
//! reorganizes `session.rs` into a `session/` directory, this baseline test
//! must be updated to aggregate signatures across the new submodules — at which
//! point the snapshot is regenerated against the post-refactor tree and
//! re-baselined (signatures themselves must still match).

#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use insta::assert_snapshot;

/// Locate the session module sources regardless of test working directory.
/// Returns the single-file path (pre-refactor) or all `.rs` files in the
/// `session/` directory (post-c255 layout).
fn session_sources() -> Vec<PathBuf> {
    let mut single = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    single.push("src/agent/session.rs");
    if single.exists() {
        return vec![single];
    }
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("src/agent/session");
    assert!(
        dir.is_dir(),
        "cannot locate session module at {}",
        dir.display()
    );
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read session dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    files
}

/// Extract a normalized, sorted list of public-item signature lines from the
/// session source. A signature is the text from a line that (after trimming
/// leading whitespace) starts with `pub ` or `pub(` up to the first body-open
/// `{` at paren/bracket depth 0, or the next `;` for re-exports/aliases.
/// Continuation lines are folded in. Whitespace is collapsed.
fn extract_public_items(src: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        // Only TRUE public items (not pub(crate)/pub(super)/pub(in ...)).
        let is_public = trimmed.starts_with("pub ")
            || trimmed.starts_with("pub(async ")
            || trimmed.starts_with("pub async ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub type ");
        // Explicitly exclude restricted visibility.
        let is_restricted = trimmed.starts_with("pub(");
        if is_public && !is_restricted {
            // Fold this line and following lines until we hit the signature
            // terminator at bracket/paren depth 0.
            let mut acc = String::new();
            let mut depth: i32 = 0;
            let mut terminated = false;
            while i < lines.len() {
                let l = lines[i];
                acc.push_str(l.trim());
                acc.push(' ');
                for ch in l.chars() {
                    match ch {
                        '(' | '[' => depth += 1,
                        ')' | ']' => depth -= 1,
                        '{' if depth == 0 => {
                            terminated = true;
                        }
                        ';' if depth == 0 => {
                            terminated = true;
                        }
                        _ => {}
                    }
                    if terminated {
                        break;
                    }
                }
                if terminated {
                    break;
                }
                i += 1;
            }
            push_normalized(&acc, &mut items);
        }
        i += 1;
    }
    items
}

/// Normalize internal whitespace; strip a trailing body-open `{` or terminator `;`.
fn push_normalized(acc: &str, items: &mut Vec<String>) {
    let collapsed: String = acc.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = collapsed
        .trim_end_matches('{')
        .trim_end_matches(';')
        .trim_end()
        .to_string();
    if !cleaned.is_empty() {
        items.push(cleaned);
    }
}

#[test]
fn agent_session_public_api_surface_baseline() {
    // Aggregate signatures across the whole session module (single file or
    // the post-c255 `session/` directory). Spec as31: the public API surface
    // MUST be stable across the refactor.
    let mut all_items: Vec<String> = Vec::new();
    for file in session_sources() {
        let src =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {}: {}", file.display(), e));
        all_items.extend(extract_public_items(&src));
    }
    all_items.sort();
    all_items.dedup();
    assert_snapshot!(all_items.join("\n"));
}
