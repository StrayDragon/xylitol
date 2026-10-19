//! Stdio trust prompt — used when bootstrap `interactive` is true (TUI path).
//!
//! Keeps terminal I/O out of the pure resolver; CLI/TUI set `interactive`
//! and bootstrap calls this when Ask policy needs a decision.

use std::io::{self, BufRead, Write};

use super::resolve::format_trust_prompt;
use super::store::TrustOption;

/// Prompt on stderr/stdin for a trust option index. Returns `None` on cancel/EOF.
pub fn prompt_trust_options_stdio(cwd: &str, options: &[TrustOption]) -> Option<usize> {
    let mut err = io::stderr().lock();
    let _ = writeln!(err, "{}", format_trust_prompt(cwd));
    let _ = writeln!(err);
    for (i, opt) in options.iter().enumerate() {
        let _ = writeln!(err, "  [{}] {}", i + 1, opt.label);
    }
    let _ = write!(err, "Choice [1-{}] (Enter = deny): ", options.len().max(1));
    let _ = err.flush();

    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return None;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let Ok(n) = trimmed.parse::<usize>() else {
        return None;
    };
    if n == 0 || n > options.len() {
        return None;
    }
    Some(n - 1)
}
