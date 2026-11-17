mod support;

use std::fs;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use xylitol_tui::autocomplete_fd::{build_fd_path_query, walk_directory_with_fd};

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

fn fd_path() -> Option<&'static str> {
    match std::process::Command::new("fd").arg("--version").output() {
        Ok(o) if o.status.success() => Some("fd"),
        _ => None,
    }
}

fn require_fd() -> &'static str {
    fd_path().expect("fd (sharkdp/fd) must be installed for the fd autocomplete tests")
}

#[test]
fn build_fd_path_query_no_separator() {
    assert_eq!(build_fd_path_query("hello"), "hello");
}

#[test]
fn build_fd_path_query_with_slash() {
    let q = build_fd_path_query("src/utils/");
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
    let has_utils_dir = results.iter().any(|(p, d)| *d && p.ends_with("utils"));
    assert!(has_utils_dir, "expected utils/ dir in results: {results:?}");
}

#[test]
fn fd_cancellation_kills_subprocess() {
    let _fd = require_fd();
    let dir = make_temp_dir_with_files();
    let base = dir.path().to_string_lossy().to_string();
    let ct = CancellationToken::new();
    ct.cancel();
    let results = walk_directory_with_fd(&base, "fd", ".", 100, ct);
    assert!(results.is_empty(), "cancelled query should return empty");
}
