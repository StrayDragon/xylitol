//! fd-based recursive filesystem search for autocomplete.
//!
//! Ported from pi's `walkDirectoryWithFd` in `autocomplete.ts`. Spawns the
//! `fd(1)` subprocess and parses its output. When `fd_path` is `None`, the
//! caller must fall back to the non-recursive `read_dir` path in
//! [`crate::autocomplete`].

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tokio_util::sync::CancellationToken;

/// Build a query string for `fd`'s `--full-path` mode.
///
/// Normalizes backslashes to forward slashes and constructs a regex pattern
/// matching path segments joined by separators. Aligns with pi
/// `buildFdPathQuery()`.
pub fn build_fd_path_query(query: &str) -> String {
    let normalized = query.replace('\\', "/");
    if !normalized.contains('/') {
        return normalized;
    }

    let has_trailing_separator = normalized.ends_with('/');
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        return normalized;
    }

    let separator_pattern = "[\\\\/]";
    let segments: Vec<String> = trimmed
        .split('/')
        .filter(|s| !s.is_empty())
        .map(regex_escape)
        .collect();
    if segments.is_empty() {
        return normalized;
    }

    let mut pattern = segments.join(separator_pattern);
    if has_trailing_separator {
        pattern.push_str(separator_pattern);
    }
    pattern
}

fn regex_escape(value: &str) -> String {
    let special: &[char] = &[
        '.', '*', '+', '?', '^', '$', '{', '}', '(', ')', '|', '[', ']', '\\',
    ];
    let mut out = String::with_capacity(value.len() * 2);
    for ch in value.chars() {
        if special.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Walk `base_dir` with `fd`, returning matching file/directory paths.
///
/// Returns `Vec<(path: String, is_directory: bool)>`. Directories have a
/// trailing `/` in the path from fd output; we strip it for the string but
/// return `true` in the flag.
///
/// When `ct.is_cancelled()`, the subprocess is killed and an empty vec is
/// returned.
pub fn walk_directory_with_fd(
    base_dir: &str,
    fd_path: &str,
    query: &str,
    max_results: usize,
    ct: CancellationToken,
) -> Vec<(String, bool)> {
    if ct.is_cancelled() {
        return vec![];
    }

    let display_query = query.replace('\\', "/");
    let has_path_separator = display_query.contains('/');

    let max_str = max_results.to_string();
    let mut args = vec![
        "--base-directory",
        base_dir,
        "--max-results",
        &max_str,
        "--type",
        "f",
        "--type",
        "d",
        "--follow",
        "--hidden",
        "--exclude",
        ".git",
        "--exclude",
        ".git/*",
        "--exclude",
        ".git/**",
    ];

    if has_path_separator {
        args.push("--full-path");
    }

    let fd_query = if query.is_empty() {
        ".".to_string()
    } else if has_path_separator {
        build_fd_path_query(query)
    } else {
        query.to_string()
    };
    args.push(&fd_query);

    let mut child = match Command::new(fd_path)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    if ct.is_cancelled() {
        let _ = child.kill();
        return vec![];
    }

    let stdout = child.stdout.take();
    let reader = BufReader::new(stdout.unwrap());
    let mut results: Vec<(String, bool)> = Vec::with_capacity(max_results);

    for line in reader.lines() {
        if ct.is_cancelled() {
            let _ = child.kill();
            return results;
        }
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.is_empty() {
            continue;
        }

        let display_line = line.replace('\\', "/");
        let is_directory = display_line.ends_with('/');
        let normalized_path = if is_directory {
            display_line[..display_line.len() - 1].to_string()
        } else {
            display_line
        };

        // Filter .git entries (fd should have excluded these, but belt-and-suspenders).
        if normalized_path == ".git"
            || normalized_path.starts_with(".git/")
            || normalized_path.contains("/.git/")
        {
            continue;
        }

        results.push((normalized_path, is_directory));
    }

    // Drain remaining output before waiting for child exit.
    let _ = child.wait();
    results
}
