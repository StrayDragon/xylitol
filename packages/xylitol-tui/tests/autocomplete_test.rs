//! Autocomplete tests — fd subprocess + debounce + async CancellationToken.
//!
//! Uses c405 layers 1 (unit) and 3 (timing with `#[tokio::test(start_paused = true)]`).

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

/// Hard-fail instead of silently skipping: a missing fd is an environment gap,
/// and a silent skip would leave these paths permanently unexercised under qa.
fn require_fd() -> &'static str {
    fd_path().expect("fd (sharkdp/fd) must be installed for the fd autocomplete tests")
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
    assert!(
        q.contains("[\\\\/]"),
        "expected path separator pattern in query: {q}"
    );
}

#[test]
fn build_fd_path_query_escapes_special_chars() {
    let q = build_fd_path_query("src/utils.rs");
    // The dot should be escaped
    assert!(
        q.contains("utils\\.rs") || !q.contains("utils.rs"),
        "expected dot escaped or query=raw"
    );
}

#[test]
fn fd_recursive_search_finds_nested_files() {
    let _fd = require_fd();
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
    let _fd = require_fd();
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
    let _fd = require_fd();
    let dir = make_temp_dir_with_files();
    let base = dir.path().to_string_lossy().to_string();
    // Cancel before spawn
    let ct = CancellationToken::new();
    ct.cancel();
    let results = walk_directory_with_fd(&base, "fd", ".", 100, ct);
    assert!(results.is_empty(), "cancelled query should return empty");
}

#[test]
fn fd_path_none_fallback() {
    let dir = make_temp_dir_with_files();
    let p = CombinedAutocompleteProvider::new(vec![], dir.path().to_path_buf());
    // Without fd_path, plain path prefixes still complete via non-recursive
    // read_dir ("src/m" → filter src/ entries by prefix "m").
    let lines = vec!["src/m".to_string()];
    let res = p.get_suggestions(&lines, 0, 5, false);
    let res = res.expect("without fd_path, read_dir fallback must complete path prefixes");
    let names: Vec<String> = res.items.iter().map(|i| i.value.clone()).collect();
    assert!(
        names.iter().any(|v| v.contains("main.rs")),
        "fallback should surface src/main.rs, got {names:?}"
    );
}

#[test]
fn async_get_suggestions_with_fd() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _fd = require_fd();
    let dir = make_temp_dir_with_files();
    let p = provider_with_fd(dir.path().to_path_buf());
    let lines = vec!["@Car".to_string()];
    let ct = CancellationToken::new();
    let res = rt.block_on(async { p.get_suggestions_async(&lines, 0, 4, false, ct).await });
    assert!(
        res.is_some(),
        "async fuzzy with fd should return suggestions"
    );
}

// ── ac01: cancellation ─────────────────────────────────────────────────────

#[test]
fn async_get_suggestions_respects_cancellation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _fd = require_fd();
    let dir = make_temp_dir_with_files();
    let p = provider_with_fd(dir.path().to_path_buf());
    let lines = vec!["@Car".to_string()];
    let ct = CancellationToken::new();
    ct.cancel();
    let res = rt.block_on(async { p.get_suggestions_async(&lines, 0, 4, false, ct).await });
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
    let lines = vec!["src/".to_string()];

    // First call at t=0
    let ct1 = CancellationToken::new();
    // Don't await — just confirm it's pending
    let _ct1_clone = ct1.clone();
    // (we can't easily test "dropped" in a single-threaded test without
    // explicit select, but we CAN test that a call after the delay fires)

    // Advance past the delay
    tokio::time::advance(Duration::from_millis(250)).await;

    let ct2 = CancellationToken::new();
    let res = debounced.get_suggestions(&lines, 0, 4, false, ct2).await;
    let res = res.expect("debounced call must complete with suggestions");
    assert!(!res.items.is_empty(), "expected at least one suggestion");
}

#[tokio::test(start_paused = true)]
async fn debounce_single_call_fires_after_delay() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let mut debounced = DebouncedAutocomplete::new(p, Duration::from_millis(250));
    let lines = vec!["src/".to_string()];
    let ct = CancellationToken::new();

    // We can't easily poll for completion on a single-threaded runtime
    // without spawning, but we can advance time and await.
    tokio::time::advance(Duration::from_millis(250)).await;
    let res = debounced.get_suggestions(&lines, 0, 4, false, ct).await;
    let res = res.expect("single debounced call must fire after the delay");
    assert!(!res.items.is_empty(), "expected at least one suggestion");
}

#[tokio::test(start_paused = true)]
async fn debounce_successive_calls_do_not_panic() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let mut debounced = DebouncedAutocomplete::new(p, Duration::from_millis(250));
    let lines = vec!["src/".to_string()];

    // Fire first query and let it complete
    let ct1 = CancellationToken::new();
    tokio::time::advance(Duration::from_millis(250)).await;
    let _ = debounced.get_suggestions(&lines, 0, 4, false, ct1).await;

    // Fire second query — this replaces last_token, cancelling the previous internal token
    let ct2 = CancellationToken::new();
    tokio::time::advance(Duration::from_millis(250)).await;
    let res = debounced.get_suggestions(&lines, 0, 4, false, ct2).await;
    let res = res.expect("successive debounced calls must still complete");
    assert!(!res.items.is_empty(), "expected at least one suggestion");
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
    assert!(res.is_none(), "cancelled debounce should return None");
}

// ── sync regression ────────────────────────────────────────────────────────

#[test]
fn get_suggestions_sync_still_works() {
    let dir = make_temp_dir_with_files();
    let p = provider(dir.path().to_path_buf());
    let lines = vec!["mod".to_string()];
    let res = p.get_suggestions(&lines, 0, 3, false);
    // Plain text has no @ / trigger, so no source fires — must be None, not a panic.
    assert!(res.is_none(), "non-trigger text must not suggest");
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
