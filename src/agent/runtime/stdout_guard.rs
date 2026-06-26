//! OutputGuard — stdout takeover for print mode.
//!
//! In print mode, agent/tool output must be suppressed from stdout
//! so the final result is clean.
//!
//! Since Rust does not allow replacing the global stdout writer at runtime,
//! this module uses an atomic flag as a semantic takeover marker:
//! - `take_over_stdout()` sets the flag and returns a guard that restores on drop
//! - `restore_stdout()` clears the flag
//! - Callers check `is_stdout_taken_over()` before writing to stdout
//! - `write_raw_stdout()` always writes to stdout regardless of the flag

#[allow(dead_code)]
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global takeover flag.
static TAKEN_OVER: AtomicBool = AtomicBool::new(false);

/// Take over stdout — set the takeover flag.
///
/// In print mode, all output that would normally go to stdout should be
/// suppressed (or redirected to stderr). This sets the flag so callers
/// can check `is_stdout_taken_over()`.
///
/// Returns a guard that automatically restores stdout when dropped.
pub(crate) fn take_over_stdout() -> OutputGuard {
    TAKEN_OVER.store(true, Ordering::SeqCst);
    OutputGuard { restored: false }
}

/// Restore stdout — clear the takeover flag.
pub(crate) fn restore_stdout() {
    TAKEN_OVER.store(false, Ordering::SeqCst);
}

/// Check whether stdout is currently taken over.
pub(crate) fn is_stdout_taken_over() -> bool {
    TAKEN_OVER.load(Ordering::SeqCst)
}

/// Write raw text directly to stdout, bypassing the takeover flag.
///
/// This is used in print mode to emit the final agent response.
/// Always writes to `std::io::stdout()` regardless of takeover state.
pub(crate) fn write_raw_stdout(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

/// Write a line directly to stdout (like `println!` but bypassing takeover).
pub(crate) fn writeln_raw_stdout(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

/// Print to stdout only if not taken over. Otherwise write to stderr.
///
/// This is the safe alternative to `println!` in agent code.
pub(crate) fn safe_println(text: &str) {
    if is_stdout_taken_over() {
        eprintln!("{text}");
    } else {
        println!("{text}");
    }
}

/// An RAII guard that restores stdout when dropped.
///
/// Created by `take_over_stdout()`.
#[must_use = "if unused the stdout takeover will be immediately undone"]
pub(crate) struct OutputGuard {
    restored: bool,
}

impl OutputGuard {
    /// Explicitly restore stdout without waiting for drop.
    pub(crate) fn restore(mut self) {
        self.restored = true;
        restore_stdout();
    }
}

impl Drop for OutputGuard {
    fn drop(&mut self) {
        if !self.restored {
            restore_stdout();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn test_takeover_and_restore() {
        assert!(!is_stdout_taken_over());

        let guard = take_over_stdout();
        assert!(is_stdout_taken_over());

        guard.restore();
        assert!(!is_stdout_taken_over());
    }

    #[test]
    #[serial_test::serial]
    fn test_guard_drop_restores() {
        assert!(!is_stdout_taken_over());

        {
            let _guard = take_over_stdout();
            assert!(is_stdout_taken_over());
        }

        assert!(!is_stdout_taken_over());
    }

    #[test]
    #[serial_test::serial]
    fn test_double_takeover_noop() {
        assert!(!is_stdout_taken_over());

        let guard1 = take_over_stdout();
        assert!(is_stdout_taken_over());

        let guard2 = take_over_stdout();
        assert!(is_stdout_taken_over());

        guard2.restore();
        assert!(!is_stdout_taken_over()); // restored after first restore

        // guard1 is already "restored" from drop's perspective
        drop(guard1);
        assert!(!is_stdout_taken_over());
    }

    #[test]
    #[serial_test::serial]
    fn test_write_raw_stdout() {
        assert!(write_raw_stdout("").is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn test_writeln_raw_stdout() {
        assert!(writeln_raw_stdout("test").is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn test_safe_println_when_taken_over() {
        let _guard = take_over_stdout();
        safe_println("should go to stderr");
        assert!(is_stdout_taken_over());
    }

    #[test]
    #[serial_test::serial]
    fn test_safe_println_when_not_taken_over() {
        assert!(!is_stdout_taken_over());
        safe_println("should go to stdout");
    }
}
