//! Interactive Diff Review system.
//!
//! Provides shared data structures ([`types`]), a CLI terminal review backend
//! ([`cli`]), and a Web browser review backend ([`web`]).
//!
//!
//! ## Architecture
//!
//! ```text
//! StepComplete → ReviewEngine::review()
//!                 ├── collect diffs
//!                 ├── create ReviewSession
//!                 ├── dispatch HookEvent::ReviewStart
//!                 ├── render (CLI or Web)
//!                 ├── user verdict
//!                 ├── dispatch HookEvent::ReviewEnd
//!                 └── return ReviewVerdict
//! ```

pub mod cli;
pub mod types;

use types::{DiffHunk, DiffLine, DiffLineKind, ReviewSession, ReviewVerdict};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ReviewEngine
// ---------------------------------------------------------------------------

/// The review engine — orchestrates diff collection, session creation, and
/// user interaction.
pub struct ReviewEngine {
    /// Configuration reference.
    config: ReviewEngineConfig,
}

/// Configuration for the review engine.
#[derive(Debug, Clone)]
pub struct ReviewEngineConfig {
    /// Which backend to use for rendering.
    pub backend: ReviewBackend,
    /// When to trigger review.
    pub mode: ReviewMode,
}

/// Supported review rendering backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewBackend {
    /// Terminal-based review via ratatui.
    Cli,
}

/// When to trigger a review session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    /// Every step completion.
    OnStep,
    /// Only on error/failure.
    OnError,
    /// Manually triggered only.
    Manual,
}

impl ReviewEngine {
    /// Create a new review engine from configuration.
    pub fn new(config: ReviewEngineConfig) -> Self {
        Self { config }
    }

    /// Create a review engine from the app config's review section.
    pub fn from_config(cfg: &crate::infra::config::types::ReviewConfig) -> Self {
        let _backend = cfg.backend.as_str();
        let backend = ReviewBackend::Cli;
        let mode = match cfg.mode.as_str() {
            "on-error" => ReviewMode::OnError,
            "manual" => ReviewMode::Manual,
            _ => ReviewMode::OnStep,
        };
        Self::new(ReviewEngineConfig { backend, mode })
    }

    /// Return the configured review mode.
    pub fn mode(&self) -> ReviewMode {
        self.config.mode
    }

    /// Create a [`ReviewSession`] from a set of changed files.
    ///
    /// `files` — a list of `(file_path, old_content, new_content)` tuples.
    /// Returns a session with generated diff hunks.
    pub fn create_session(&self, files: &[(String, String, String)]) -> ReviewSession {
        let review_id = Uuid::new_v4().to_string();
        let mut hunks = Vec::new();

        for (file, old_content, new_content) in files {
            let file_hunks = generate_file_hunks(file, old_content, new_content);
            hunks.extend(file_hunks);
        }

        ReviewSession::new(review_id, hunks)
    }

    /// Run the review session and return the user's verdict.
    ///
    /// Dispatches `ReviewStart` and `ReviewEnd` hook events if a hook
    /// dispatcher is provided.
    pub async fn run_review(
        &self,
        session: &mut ReviewSession,
        hook_dispatcher: Option<&crate::infra::hooks::HookDispatcher>,
    ) -> ReviewVerdict {
        // Dispatch ReviewStart hook.
        if let Some(hooks) = hook_dispatcher {
            let diffs: Vec<String> = session
                .hunks
                .iter()
                .map(|h| {
                    let mut buf = String::new();
                    buf.push_str(&format!(
                        "@@ -{},{} +{},{} @@ {}\n",
                        h.old_start, h.old_count, h.new_start, h.new_count, h.file
                    ));
                    for line in &h.lines {
                        buf.push_str(line.kind.prefix());
                        buf.push_str(&line.content);
                        if !line.content.ends_with('\n') {
                            buf.push('\n');
                        }
                    }
                    buf
                })
                .collect();
            hooks
                .dispatch(
                    &crate::infra::hooks::HookEvent::ReviewStart { diffs },
                    crate::infra::hooks::HookPhase::Pre,
                )
                .await;
        }

        // Run the selected backend.
        let verdict = match self.config.backend {
            ReviewBackend::Cli => self.run_cli_review(session).await,
        };

        // Dispatch ReviewEnd hook.
        if let Some(hooks) = hook_dispatcher {
            let approved = matches!(verdict, ReviewVerdict::AcceptAll(_));
            hooks
                .dispatch(
                    &crate::infra::hooks::HookEvent::ReviewEnd { approved },
                    crate::infra::hooks::HookPhase::Post,
                )
                .await;
        }

        verdict
    }

    /// Run CLI (ratatui) review.
    async fn run_cli_review(&self, session: &mut ReviewSession) -> ReviewVerdict {
        match cli::run_cli_review(session).await {
            Ok(verdict) => verdict,
            Err(e) => {
                eprintln!("CLI review error: {e}");
                ReviewVerdict::AcceptAll(session.comments.clone())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Demo / smoke-test
// ---------------------------------------------------------------------------

/// Run the review demo — generates a realistic temp project, simulates agent
/// modifications, and enters the CLI review UI for interactive smoke-testing.
///
/// # XyUsage
///
/// ```bash
/// cargo run --example review-demo
/// ```
pub async fn run_demo() -> Result<(), String> {
    let review_cfg = Default::default();
    let engine = ReviewEngine::from_config(&review_cfg);

    // ── Step 1: Create temp project ──────────────────────────────────
    let tmp = std::env::temp_dir().join(format!("xylitol-review-demo-{}", Uuid::new_v4()));
    let old_dir = tmp.join("before");
    let new_dir = tmp.join("after");
    std::fs::create_dir_all(&old_dir).map_err(|e| format!("create old_dir: {e}"))?;
    std::fs::create_dir_all(&new_dir).map_err(|e| format!("create new_dir: {e}"))?;

    // ── Step 2: Seed initial files (before state) ────────────────────
    let files: Vec<(&str, &str, &str)> = vec![
        (
            "src/main.py",
            r#"#!/usr/bin/env python3
"""Simple calculator app."""

import sys


def add(a: int, b: int) -> int:
    return a + b


def subtract(a: int, b: int) -> int:
    return a - b


def main():
    if len(sys.argv) < 4:
        print("XyUsage: calc.py <op> <a> <b>")
        sys.exit(1)
    op = sys.argv[1]
    a = int(sys.argv[2])
    b = int(sys.argv[3])
    if op == "add":
        print(add(a, b))
    elif op == "sub":
        print(subtract(a, b))
    else:
        print(f"Unknown op: {op}")
        sys.exit(1)


if __name__ == "__main__":
    main()
"#,
            r#"#!/usr/bin/env python3
"""Simple calculator app — now with multiply, divide, and logging."""

import logging
import sys

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger(__name__)


def add(a: int, b: int) -> int:
    result = a + b
    logger.info("add(%d, %d) = %d", a, b, result)
    return result


def subtract(a: int, b: int) -> int:
    result = a - b
    logger.info("sub(%d, %d) = %d", a, b, result)
    return result


def multiply(a: int, b: int) -> int:
    result = a * b
    logger.info("mul(%d, %d) = %d", a, b, result)
    return result


def divide(a: int, b: int) -> float:
    if b == 0:
        raise ValueError("division by zero")
    result = a / b
    logger.info("div(%d, %d) = %.2f", a, b, result)
    return result


def main():
    if len(sys.argv) < 4:
        print("XyUsage: calc.py <op> <a> <b>")
        print("  op: add | sub | mul | div")
        sys.exit(1)
    op = sys.argv[1]
    a = int(sys.argv[2])
    b = int(sys.argv[3])
    ops = {
        "add": add,
        "sub": subtract,
        "mul": multiply,
        "div": divide,
    }
    if op in ops:
        result = ops[op](a, b)
        print(f"Result: {result}")
    else:
        print(f"Unknown op: {op}")
        sys.exit(1)


if __name__ == "__main__":
    main()
"#,
        ),
        (
            "src/config.py",
            r#"DEFAULT_PORT = 8080
DEBUG = False
MAX_RETRIES = 3
"#,
            r#"DEFAULT_PORT = 3000
DEBUG = True
MAX_RETRIES = 5
LOG_LEVEL = "INFO"
DATABASE_URL = "sqlite:///data.db"
"#,
        ),
        (
            "README.md",
            r#"# Calculator

A simple calculator.

## XyUsage

```
python main.py add 1 2
```
"#,
            r#"# Calculator

A simple calculator with extended operations.

## Features

- Addition, subtraction, multiplication, division
- Structured logging
- Error handling

## XyUsage

```
python main.py add 1 2
python main.py mul 3 4
python main.py div 10 2
```

## Configuration

Edit `config.py` to adjust port, debug mode, and retry settings.
"#,
        ),
        (
            "tests/test_calc.py",
            r#"from src.main import add, subtract


class TestCalc:
    def test_add(self):
        assert add(1, 2) == 3

    def test_subtract(self):
        assert subtract(5, 3) == 2
"#,
            r#"from src.main import add, subtract, multiply, divide
import pytest


class TestCalc:
    def test_add(self):
        assert add(1, 2) == 3
        assert add(-1, 1) == 0

    def test_subtract(self):
        assert subtract(5, 3) == 2
        assert subtract(0, 5) == -5

    def test_multiply(self):
        assert multiply(3, 4) == 12
        assert multiply(-2, 3) == -6

    def test_divide(self):
        assert divide(10, 2) == 5.0
        assert divide(7, 2) == 3.5

    def test_divide_by_zero(self):
        with pytest.raises(ValueError):
            divide(1, 0)
"#,
        ),
    ];

    // ── Step 3: Write files to disk ──────────────────────────────────
    let mut changes: Vec<(String, String, String)> = Vec::new();

    for (path, old_content, new_content) in &files {
        // Write old version
        let old_path = old_dir.join(path);
        if let Some(parent) = old_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create {parent:?}: {e}"))?;
        }
        std::fs::write(&old_path, old_content).map_err(|e| format!("write {old_path:?}: {e}"))?;

        // Write new version
        let new_path = new_dir.join(path);
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create {parent:?}: {e}"))?;
        }
        std::fs::write(&new_path, new_content).map_err(|e| format!("write {new_path:?}: {e}"))?;

        changes.push((
            path.to_string(),
            old_content.to_string(),
            new_content.to_string(),
        ));
    }

    // ── Step 4: Create review session ────────────────────────────────
    let mut session = engine.create_session(&changes);

    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║         xylitol — Diff Review Demo              ║");
    println!("  ╠══════════════════════════════════════════════════╣");
    println!("  ║  Temp project: {:>41} ║", tmp.display());
    println!("  ║  Files changed: {:>41} ║", session.file_count());
    println!("  ║  Diff hunks:    {:>41} ║", session.hunks.len());
    println!("  ║  Backend:       {:>41} ║", review_cfg.backend);
    println!("  ╚══════════════════════════════════════════════════╝");
    println!();

    if session.hunks.is_empty() {
        println!("  (no changes to review — demo data may be stale)");
        return Ok(());
    }

    // ── Step 5: Enter review ────────────────────────────────────────
    let verdict = engine.run_review(&mut session, None).await;

    // ── Step 6: Report verdict ──────────────────────────────────────
    println!();
    let (accepted, comments) = match &verdict {
        ReviewVerdict::AcceptAll(comments) => (true, comments),
        ReviewVerdict::RejectWithComments(comments) => (false, comments),
    };
    if accepted {
        println!("  ✓ Review demo: changes ACCEPTED.");
        println!("  → New files are at: {:?}", new_dir);
    } else {
        println!("  ✗ Review demo: changes REJECTED.");
    }
    if !comments.is_empty() {
        println!("  → {} comment(s) provided:", comments.len());
        for (i, c) in comments.iter().enumerate() {
            println!(
                "    {}. [{}] {}:{} — {}",
                i + 1,
                c.severity.label(),
                c.file,
                c.line_start,
                c.content
            );
        }
    }
    println!();

    // Cleanup temp dir if accepted.
    if accepted {
        let _ = std::fs::remove_dir_all(&tmp);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Diff generation
// ---------------------------------------------------------------------------

/// Generate structured [`DiffHunk`]s from old and new file content.
fn generate_file_hunks(file_path: &str, old_text: &str, new_text: &str) -> Vec<DiffHunk> {
    use similar::{ChangeTag, TextDiff};

    if old_text == new_text {
        return Vec::new();
    }

    let diff = TextDiff::from_lines(old_text, new_text);
    let mut hunks = Vec::new();
    let mut lines = Vec::new();
    let mut old_lineno = 0u32;
    let mut new_lineno = 0u32;
    let mut old_start = 0u32;
    let mut new_start = 0u32;
    let mut in_hunk = false;

    for change in diff.iter_all_changes() {
        let tag = change.tag();
        let value = change.value();
        let is_newline = value == "\n" || value == "\r\n";

        match tag {
            ChangeTag::Equal => {
                if !is_newline {
                    old_lineno += 1;
                    new_lineno += 1;
                    if in_hunk {
                        // Context line inside a hunk — flush the hunk here
                        // (unified diff style includes context before/after changes).
                        lines.push(DiffLine {
                            kind: DiffLineKind::Context,
                            content: value.to_string(),
                            old_line_no: old_lineno,
                            new_line_no: new_lineno,
                        });

                        // Count lines seen so far in this hunk.
                        let old_seen =
                            lines.iter().filter(|l| l.kind != DiffLineKind::Add).count() as u32;
                        let new_seen = lines
                            .iter()
                            .filter(|l| l.kind != DiffLineKind::Delete)
                            .count() as u32;
                        hunks.push(DiffHunk {
                            file: file_path.to_string(),
                            old_start,
                            old_count: old_seen,
                            new_start,
                            new_count: new_seen,
                            lines: lines.clone(),
                        });
                        in_hunk = false;
                    }
                }
            }
            ChangeTag::Delete => {
                if !in_hunk {
                    in_hunk = true;
                    old_start = old_lineno + 1;
                    new_start = new_lineno + 1;
                    lines.clear();
                }
                old_lineno += 1;
                lines.push(DiffLine {
                    kind: DiffLineKind::Delete,
                    content: value.to_string(),
                    old_line_no: old_lineno,
                    new_line_no: 0,
                });
            }
            ChangeTag::Insert => {
                if !in_hunk {
                    in_hunk = true;
                    old_start = old_lineno + 1;
                    new_start = new_lineno + 1;
                    lines.clear();
                }
                new_lineno += 1;
                lines.push(DiffLine {
                    kind: DiffLineKind::Add,
                    content: value.to_string(),
                    old_line_no: 0,
                    new_line_no: new_lineno,
                });
            }
        }
    }

    // Flush remaining hunk (no trailing context).
    if in_hunk && !lines.is_empty() {
        let old_seen = lines.iter().filter(|l| l.kind != DiffLineKind::Add).count() as u32;
        let new_seen = lines
            .iter()
            .filter(|l| l.kind != DiffLineKind::Delete)
            .count() as u32;
        hunks.push(DiffHunk {
            file: file_path.to_string(),
            old_start,
            old_count: old_seen,
            new_start,
            new_count: new_seen,
            lines,
        });
    }

    hunks
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::types::{CommentSeverity, DiffLineKind};
    use super::*;

    // ── generate_file_hunks ──────────────────────────────────────────

    #[test]
    fn test_generate_hunks_no_changes() {
        let hunks = generate_file_hunks("file.txt", "same\ncontent\n", "same\ncontent\n");
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_generate_hunks_insertion() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        let hunks = generate_file_hunks("f.rs", old, new);
        assert!(!hunks.is_empty());
        let add_lines: Vec<_> = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == DiffLineKind::Add)
            .collect();
        assert!(!add_lines.is_empty());
        assert!(add_lines.iter().any(|l| l.content.contains("line3")));
    }

    #[test]
    fn test_generate_hunks_deletion() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline3\n";
        let hunks = generate_file_hunks("f.rs", old, new);
        assert!(!hunks.is_empty());
        let del_lines: Vec<_> = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == DiffLineKind::Delete)
            .collect();
        assert!(!del_lines.is_empty());
        assert!(del_lines.iter().any(|l| l.content.contains("line2")));
    }

    #[test]
    fn test_generate_hunks_replacement() {
        let old = "keep\nremove\nkeep\n";
        let new = "keep\nreplaced\nkeep\n";
        let hunks = generate_file_hunks("f.rs", old, new);
        assert!(!hunks.is_empty());
        let kinds: Vec<_> = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .map(|l| l.kind)
            .collect();
        assert!(kinds.contains(&DiffLineKind::Delete), "expected delete");
        assert!(kinds.contains(&DiffLineKind::Add), "expected add");
    }

    #[test]
    fn test_generate_hunks_empty_new() {
        let old = "line1\nline2\n";
        let new = "";
        let hunks = generate_file_hunks("f.rs", old, new);
        assert!(!hunks.is_empty());
        assert!(
            hunks
                .iter()
                .flat_map(|h| &h.lines)
                .all(|l| l.kind == DiffLineKind::Delete)
        );
    }

    #[test]
    fn test_generate_hunks_empty_old() {
        let old = "";
        let new = "line1\nline2\n";
        let hunks = generate_file_hunks("f.rs", old, new);
        assert!(!hunks.is_empty());
        assert!(
            hunks
                .iter()
                .flat_map(|h| &h.lines)
                .all(|l| l.kind == DiffLineKind::Add)
        );
    }

    // ── ReviewSession ────────────────────────────────────────────────

    #[test]
    fn test_session_new() {
        let s = ReviewSession::new("test-1".into(), Vec::new());
        assert_eq!(s.review_id, "test-1");
        assert!(s.hunks.is_empty());
        assert!(s.comments.is_empty());
        assert!(s.verdict.is_none());
    }

    #[test]
    fn test_session_add_comment() {
        let mut s = ReviewSession::new("t1".into(), Vec::new());
        let c = types::ReviewComment {
            file: "f.rs".into(),
            line_start: 1,
            line_end: 1,
            content: "looks wrong".into(),
            severity: types::CommentSeverity::Warning,
        };
        s.add_comment(c);
        assert_eq!(s.comments.len(), 1);
    }

    #[test]
    fn test_session_total_changes() {
        let hunks = vec![DiffHunk {
            file: "f.rs".into(),
            old_start: 1,
            old_count: 3,
            new_start: 1,
            new_count: 3,
            lines: vec![
                DiffLine {
                    kind: DiffLineKind::Context,
                    content: "a\n".into(),
                    old_line_no: 1,
                    new_line_no: 1,
                },
                DiffLine {
                    kind: DiffLineKind::Delete,
                    content: "b\n".into(),
                    old_line_no: 2,
                    new_line_no: 0,
                },
                DiffLine {
                    kind: DiffLineKind::Add,
                    content: "c\n".into(),
                    old_line_no: 0,
                    new_line_no: 2,
                },
            ],
        }];
        let s = ReviewSession::new("t1".into(), hunks);
        assert_eq!(s.total_changes(), 3);
        assert_eq!(s.file_count(), 1);
    }

    // ── ReviewVerdict ────────────────────────────────────────────────

    #[test]
    fn test_verdict_accept() {
        let v = ReviewVerdict::AcceptAll(Vec::new());
        assert!(matches!(v, ReviewVerdict::AcceptAll(_)));
    }

    #[test]
    fn test_verdict_reject_with_comments() {
        let v = ReviewVerdict::RejectWithComments(Vec::new());
        match v {
            ReviewVerdict::RejectWithComments(c) => assert!(c.is_empty()),
            _ => panic!("expected RejectWithComments"),
        }
    }

    // ── CommentSeverity ──────────────────────────────────────────────

    #[test]
    fn test_severity_parse() {
        assert_eq!(CommentSeverity::parse("info"), CommentSeverity::Info);
        assert_eq!(CommentSeverity::parse("WARNING"), CommentSeverity::Warning);
        assert_eq!(CommentSeverity::parse("error"), CommentSeverity::Error);
        assert_eq!(
            CommentSeverity::parse("critical"),
            CommentSeverity::Critical
        );
        assert_eq!(CommentSeverity::parse("unknown"), CommentSeverity::Info);
    }

    #[test]
    fn test_severity_label() {
        assert_eq!(CommentSeverity::Info.label(), "info");
        assert_eq!(CommentSeverity::Warning.label(), "warning");
        assert_eq!(CommentSeverity::Error.label(), "error");
        assert_eq!(CommentSeverity::Critical.label(), "critical");
    }

    // ── DiffLineKind ─────────────────────────────────────────────────

    #[test]
    fn test_diff_line_prefix() {
        assert_eq!(DiffLineKind::Add.prefix(), "+");
        assert_eq!(DiffLineKind::Delete.prefix(), "-");
        assert_eq!(DiffLineKind::Context.prefix(), " ");
    }

    // ── ReviewEngine ─────────────────────────────────────────────────

    #[test]
    fn test_engine_from_config_defaults() {
        let cfg = crate::infra::config::types::ReviewConfig::default();
        let engine = ReviewEngine::from_config(&cfg);
        assert_eq!(engine.mode(), ReviewMode::OnStep);
    }

    #[test]
    fn test_engine_create_session_no_changes() {
        let engine = ReviewEngine::new(ReviewEngineConfig {
            backend: ReviewBackend::Cli,
            mode: ReviewMode::OnStep,
        });
        let session = engine.create_session(&[]);
        assert!(session.hunks.is_empty());
    }

    #[test]
    fn test_engine_create_session_with_changes() {
        let engine = ReviewEngine::new(ReviewEngineConfig {
            backend: ReviewBackend::Cli,
            mode: ReviewMode::OnStep,
        });
        let files = vec![("f.rs".into(), "old\n".into(), "new\n".into())];
        let session = engine.create_session(&files);
        assert!(!session.hunks.is_empty());
    }
}
