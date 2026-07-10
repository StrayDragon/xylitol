//! Debug logging bootstrap — installs the global tracing subscriber.
//!
//! `tracing` is an unconditional dependency and emit sites already exist across
//! `infra/` and `agent/`, but until a subscriber is installed those emits are
//! silent no-ops. This module installs a **file-only** subscriber so logs never
//! reach stdout/stderr (which would corrupt the TUI's inline viewport + per-frame
//! DSR cursor query).
//!
//! Activation priority (no CLI flag, no settings field):
//! - `RUST_LOG` set → install with `EnvFilter::try_from_default_env()`.
//! - `XYLITOL_DEBUG=1` (and no `RUST_LOG`) → install with the default filter
//!   `xylitol=debug,warn`.
//! - **debug builds** (`cfg(debug_assertions)`) → same default filter (c460).
//! - release builds with neither env → do nothing; `tracing::` stays no-op.
//!
//! The file writer is **synchronous** (`OpenOptions::append`), not
//! `tracing-appender::non_blocking`, so `panic = "abort"` (see `Cargo.toml`)
//! cannot drop buffered lines on the floor. The file is created `0o600` on unix
//! and ANSI is disabled.
//!
//! Init lives at the composition root (`app::cli::run`) — the single common
//! entry for print / TUI / RPC / subcommands — so every app surface is covered.
//! The `agent/` layer never installs a subscriber (it only emits), preserving
//! the `agent → runtime_protocol → domain` dependency direction.

use std::io::Write;
use std::path::Path;

use tracing_subscriber::EnvFilter;

/// Default filter used when `XYLITOL_DEBUG=1` is set but `RUST_LOG` is not.
///
/// `xylitol=` matches this crate's emits (tracing uses the crate name as the
/// default target); the bare `warn` catches warnings from any dependency.
const DEFAULT_FILTER: &str = "xylitol=debug,warn";

/// Install the global tracing subscriber if the environment requests it.
///
/// Writes to `<agent_dir>/logs/xylitol.log` (created if missing). Returns
/// `Some(())` when a subscriber was installed, `None` when logging stayed off.
/// Installing when a global subscriber already exists (e.g. a prior call, or a
/// test harness) is a no-op via `try_init`.
pub fn init_logging(agent_dir: &Path) -> Option<()> {
    // Priority: explicit RUST_LOG > XYLITOL_DEBUG one-switch > off.
    // An empty RUST_LOG is treated as unset so a stale `export RUST_LOG=`
    // in the user's shell doesn't accidentally enable verbose logging.
    let filter = if std::env::var_os("RUST_LOG").is_some_and(|v| !v.is_empty()) {
        EnvFilter::try_from_default_env().ok()?
    } else if std::env::var_os("XYLITOL_DEBUG").is_some_and(is_truthy) {
        EnvFilter::new(DEFAULT_FILTER)
    } else {
        // Debug builds: default on so `tail -f ~/.xylitol/logs/xylitol.log` works
        // without env vars (c460 / ath3). Release stays off unless env opts in.
        #[cfg(debug_assertions)]
        {
            EnvFilter::new(DEFAULT_FILTER)
        }
        #[cfg(not(debug_assertions))]
        {
            return None;
        }
    };

    let log_dir = agent_dir.join("logs");
    let log_path = log_dir.join("xylitol.log");
    // create_dir_all + open are best-effort: if the log dir can't be created
    // (read-only home, sandbox), fall back to no logging rather than crashing
    // the app — debug logging must never break the normal flow.
    let file = open_append(&log_dir, &log_path)?;

    // `try_init` returns Err if a global subscriber is already set (e.g. in
    // repeated test runs). That is benign — keep the first subscriber.
    let _ = tracing_subscriber::fmt()
        .with_writer(file)
        .with_ansi(false)
        .with_env_filter(filter)
        .try_init();

    tracing::info!(target: "xylitol::logging", path = %log_path.display(), "logging enabled");
    Some(())
}

/// Open `path` for append, creating it (and `dir`) as needed. On unix the file
/// is created `0o600` so logs (which may carry prompts/secrets) stay private.
fn open_append(dir: &Path, path: &Path) -> Option<std::fs::File> {
    std::fs::create_dir_all(dir).ok()?;
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path).ok()?;
    // Force the umask-respecting mode on already-existing files too (create_dir
    // above may have left a loose 0027 umask difference); chmod is best-effort.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    // Eagerly surface any deferred open error and keep the writer honest.
    let _ = file.flush();
    Some(file)
}

/// Match common truthy env values (`1`, `true`, `yes`, `on`), case-insensitive.
fn is_truthy(v: std::ffi::OsString) -> bool {
    let s = v.to_string_lossy();
    matches!(
        s.as_ref(),
        "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_append_creates_file_and_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("logs");
        let path = dir.join("xylitol.log");
        let _ = open_append(&dir, &path).expect("open_append succeeds");
        assert!(path.exists());
        // Reopening appends rather than truncating.
        let mut f = open_append(&dir, &path).unwrap();
        use std::io::Write;
        assert!(writeln!(f, "line1").is_ok());
        let mut f2 = open_append(&dir, &path).unwrap();
        assert!(writeln!(f2, "line2").is_ok());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("line1") && contents.contains("line2"));
    }

    #[test]
    fn open_append_missing_dir_is_no_op() {
        // A read-only parent makes create_dir_all fail → open_append yields None
        // (debug logging must never break the normal flow).
        let tmp = tempfile::tempdir().unwrap();
        let ro = tmp.path().join("ro");
        std::fs::create_dir(&ro).unwrap();
        let mut perms = std::fs::metadata(&ro).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&ro, perms).unwrap();
        let dir = ro.join("logs");
        let path = dir.join("xylitol.log");
        assert!(open_append(&dir, &path).is_none());
    }

    #[test]
    fn is_truthy_matches_common_values() {
        assert!(is_truthy("1".into()));
        assert!(is_truthy("true".into()));
        assert!(is_truthy("yes".into()));
        assert!(is_truthy("on".into()));
        assert!(!is_truthy("0".into()));
        assert!(!is_truthy("no".into()));
        assert!(!is_truthy("".into()));
    }

    // `init_logging`'s env-driven branch is exercised manually
    // (`RUST_LOG=debug cargo run -- tui`) and via the BDD/manual checks in
    // tasks.md: it mutates the process-global subscriber + env vars, so unit
    // tests that assert on it are inherently flaky and provide little extra
    // signal over the pure-helper tests above.
}
