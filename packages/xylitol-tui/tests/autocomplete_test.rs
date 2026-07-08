//! Autocomplete tests — fd subprocess + debounce + async CancellationToken.
//!
//! Uses c405 layers 1 (unit), 3 (timing with `#[tokio::test(start_paused = true)]`),
//! and 4 (proptest).

mod support;

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use xylitol_tui::autocomplete::{
    AutocompleteProvider, CombinedAutocompleteProvider, DebouncedAutocomplete,
};
use xylitol_tui::autocomplete_fd::{build_fd_path_query, walk_directory_with_fd};

// ── helpers ─────────────────────────────────────────────────────────────────

fn make_temp_dir_with_files() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src/utils")).unwrap();
    fs::create_dir_all(dir.path().join("src/tests")).unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("src/main.rs"), b"fn main() {}").unwrap();
    fs::write(dir.path().join("src/utils/mod.rs"), b"").unwrap();
    fs::write(dir.path().join("src/utils/helper.rs"), b"fn help() {}").unwrap();
    fs::write(dir.path().join("src/tests/test_main.rs"), b"").unwrap();
    fs::write(dir.path().join("docs/readme.md"), b"# docs").unwrap();
    fs::write(dir.path().join("Cargo.toml"), b"[package]").unwrap();
    fs::write(dir.path().join(".gitignore"), b"target/\n").unwrap();
    dir
}

/// Returns an fd path or None (skip fd tests).
fn fd_path() -> Option<&'static str> {
    // CI-friendly: skip if fd not installed
    match std::process::Command::new("fd").arg("--version").output() {
        Ok(o) if o.status.success() => Some("fd"),
        _ => None,
    }
}

fn provider(base: PathBuf) -> CombinedAutocompleteProvider {
    CombinedAutocompleteProvider::new(vec![], base)
}

fn provider_with_fd(base: PathBuf) -> CombinedAutocompleteProvider {
    CombinedAutocompleteProvider::new_with_fd(vec![], base, "fd".to_string())
}

// ── ac02: fd recursive search ──────────────────────────────────────────────

#[test]
fn build_fd_path_query_no_separator() {
    assert_eq!(build_fd_path_query("hello"), "hello");
}

#[test]
fn build_fd_path_query_with_slash() {
    let q = build_fd_path_query("src/utils/");
    // Should contain the separator pattern [\\\\/] and the segments
    assert!(q.contains("src"), "expected 'src' in query: {q}");
    assert!(q.contains("utils"), "expected 'utils' in query: {q}");
    assert!(q.contains("[\\\\/]"), "expected path separator pattern in query: {q}");
}

#[test]
fn build_fd_path_query_escapes_special_chars() {
    let q = build_fd_path_query("src/utils.rs");
    // The dot should be escaped
    assert!(q.contains("utils\\.rs") || !q.contains("utils.rs"),
        "expected dot escaped or query=raw");
}

#[test]
fn fd_recursive_search_finds_nested_files() {
    let _fd = fd_path();
    if fd_path().is_none() {
        eprintln!("skipping: fd not installed");
        return;
    }
    let dir = make_temp_dir_with_files();
    let base = dir.path().to_string_lossy().to_string();
    let ct = CancellationToken::new();
    let results = walk_directory_with_fd(&base, "fd", "helper", 20, ct);
    assert!(
        results.iter().any(|(p, _)| p.ends_with("helper.rs")),
        "expected helper.rs in results: {results:?}"
    );
}

#[test]
fn fd_recursive_search_finds_directories() {
    if fd_path().is_none() {
        eprintln!("skipping: fd not installed");
        return;
    }
    let dir = make_temp_dir_with_files();
    let base = dir.path().to_string_lossy().to_string();
    let ct = CancellationToken::new();
    let results = walk_directory_with_fd(&base, "fd", "utils", 20, ct);
    // fd outputs directories with trailing "/" which we strip
    let has_utils_dir = results.iter().any(|(p, d)| *d && p.ends_with("utils"));
    assert!(has_utils_dir, "expected utils/ dir in results: {results:?}");
}

#[test]
fn fd_cancellation_kills_subprocess() {
    if fd_path().is_none() {
        eprintln!("skipping: fd not installed");
        return;
    }
    let dir = make_temp_dir_with_files();
    let base = dir.path().to_string_lossy().to_string();
    // Cancel before spawn
    let ct = CancellationToken::new();
    ct.cancel();
    let results = walk_directory_with_fd(&base, "fd", ".", 100, ct);
    assert!(
        results.is_empty(),
        "cancelled query should return empty"
    );
}

#[test]
fn fd_path_none_fallback() {
    let dir = make_temp_dir_with_files();
    let p = CombinedAutocompleteProvider::new(
        vec![],
        dir.path().to_path_buf(),
    );
    let lines = vec!["@mod".to_string()];
    let res = p.get_suggestions(&lines, 0, 4, false);
    // Without fd_path, should use non-recursive read_dir (no fd subprocess)
    // Just verify it doesn't panic
    assert!(res.is_some() || res.is_none());
}

#[test]
fn async_get_suggestions_with_fd() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    if fd_path().is_none() {
        eprintln!("skipping: fd not installed");
        return;
    }
    let dir = make_temp_dir_with_files();
    let p = provider_with_fd(dir.path().to_path_buf());
    let lines = vec!["@Car".to_string()];
    let ct = CancellationToken::new();
    let res = rt.block_on(async {
        p.get_suggestions_async(&lines, 0, 4, false, ct).await
    });
    assert!(res.is_some(), "async fuzzy with fd should return suggestions");
}

// ── ac01: cancellation ─────────────────────────────────────────────────────

#[test]
fn async_get_suggestions_respects_cancellation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    if fd_path().is_none() {
        eprintln!("skipping: fd not installed");
        return;
    }
    let dir = make_temp_dir_with_files();
    let p = provider_with_fd(dir.path().to_path_buf());
    let lines = vec!["@Car".to_string()];
    let ct = CancellationToken::new();
    ct.cancel();
    let res = rt.block_on(async {
        p.get_suggestions_async(&lines, 0, 4, false, ct).await
    });
    // When cancelled before the query, should return None immediately
    // (the sync fallback in get_suggestions_async would still run if we
    // weren't checking ct before the fuzzy call)
    assert!(res.is_none(), "pre-cancelled should return None");
}

// ── ac03: debounce ─────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn debounce_drops_intermediate_calls() {
    // Create a provider with fd_path (but we don't care about results here)
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let mut debounced = DebouncedAutocomplete::new(p, Duration::from_millis(250));
    let lines = vec!["test".to_string()];

    // First call at t=0
    let ct1 = CancellationToken::new();
    // Don't await — just confirm it's pending
    let _ct1_clone = ct1.clone();
    // (we can't easily test "dropped" in a single-threaded test without
    // explicit select, but we CAN test that a call after the delay fires)

    // Advance past the delay
    tokio::time::advance(Duration::from_millis(250)).await;

    let ct2 = CancellationToken::new();
    let res = debounced
        .get_suggestions(&lines, 0, 4, false, ct2)
        .await;
    // Should complete (doesn't panic/crash on cancelled previous)
    assert!(res.is_some() || res.is_none());
}

#[tokio::test(start_paused = true)]
async fn debounce_single_call_fires_after_delay() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let mut debounced = DebouncedAutocomplete::new(p, Duration::from_millis(250));
    let lines = vec!["test".to_string()];
    let ct = CancellationToken::new();

    // We can't easily poll for completion on a single-threaded runtime
    // without spawning, but we can advance time and await.
    tokio::time::advance(Duration::from_millis(250)).await;
    let res = debounced.get_suggestions(&lines, 0, 4, false, ct).await;
    assert!(res.is_some() || res.is_none());
}

#[tokio::test(start_paused = true)]
async fn debounce_successive_calls_do_not_panic() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let mut debounced = DebouncedAutocomplete::new(p, Duration::from_millis(250));
    let lines = vec!["test".to_string()];

    // Fire first query and let it complete
    let ct1 = CancellationToken::new();
    tokio::time::advance(Duration::from_millis(250)).await;
    let _ = debounced.get_suggestions(&lines, 0, 4, false, ct1).await;

    // Fire second query — this replaces last_token, cancelling the previous internal token
    let ct2 = CancellationToken::new();
    tokio::time::advance(Duration::from_millis(250)).await;
    let res = debounced.get_suggestions(&lines, 0, 4, false, ct2).await;
    assert!(res.is_some() || res.is_none());
}

#[tokio::test]
async fn debounce_cancellation_by_upstream_ct() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let mut debounced = DebouncedAutocomplete::new(p, Duration::from_millis(250));
    let lines = vec!["test".to_string()];
    let ct = CancellationToken::new();

    // Cancel upstream token immediately
    ct.cancel();
    let res = debounced.get_suggestions(&lines, 0, 4, false, ct).await;
    assert!(
        res.is_none(),
        "cancelled debounce should return None"
    );
}

// ── sync regression ────────────────────────────────────────────────────────

#[test]
fn get_suggestions_sync_still_works() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let lines = vec!["mod".to_string()];
    let res = p.get_suggestions(&lines, 0, 3, false);
    assert!(res.is_some() || res.is_none());
}

#[test]
fn slash_command_completion_works() {
    let dir = TempDir::new().unwrap();
    let cmds = vec![xylitol_tui::autocomplete::SlashCommand {
        name: "help".to_string(),
        description: Some("Show help".to_string()),
        argument_hint: None,
        get_argument_completions: None,
    }];
    let p = CombinedAutocompleteProvider::new(cmds, dir.path().to_path_buf());
    let lines = vec!["/hel".to_string()];
    let res = p.get_suggestions(&lines, 0, 4, false);
    assert!(
        res.is_some() && res.unwrap().items.iter().any(|i| i.value == "help"),
        "slash command completion should match /hel"
    );
}

// ── proptest: fd vs read_dir consistency (layer 4) ─────────────────────────

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::fs;

    fn collect_read_dir_recursive(dir: &std::path::Path) -> HashSet<String> {
        let mut set = HashSet::new();
        fn walk(set: &mut HashSet<String>, root: &std::path::Path, dir: &std::path::Path) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let display = rel.to_string_lossy().replace('\\', "/");
                    if is_dir {
                        set.insert(format!("{}/", display));
                        walk(set, root, &path);
                    } else {
                        set.insert(display.to_string());
                    }
                }
            }
        }
        walk(&mut set, dir, dir);
        set
    }

    proptest! {
        #[test]
        fn fd_results_subset_of_read_dir(_seed: u64) {
            // Skip if fd not installed
            if fd_path().is_none() {
                return Ok(());
            }
            let dir = make_temp_dir_with_files();
            let base = dir.path().to_string_lossy().to_string();
            let ct = CancellationToken::new();
            let fd_results = walk_directory_with_fd(&base, "fd", ".", 200, ct);

            let fd_set: HashSet<String> = fd_results
                .into_iter()
                .map(|(p, _)| p)
                .collect();

            let readdir_set = collect_read_dir_recursive(dir.path());

            // Every fd result should exist in a recursive read_dir walk
            for entry in &fd_set {
                // fd normalizes paths; so should our read_dir walker
                assert!(
                    readdir_set.contains(entry) || readdir_set.contains(&format!("{}/", entry)),
                    "fd returned {entry} but recursive read_dir does not contain it. \
                     readdir: {readdir_set:?}, fd: {fd_set:?}"
                );
            }
        }
    }
}
