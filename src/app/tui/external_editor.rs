//! Product Ctrl+G external editor (c650) — independent of `agent_demo` / package.
//!
//! Package provides only [`xylitol_tui::TUI::with_terminal_suspended`].
//! This module owns `$VISUAL`/`$EDITOR` resolve, tempfile, and spawn.

use std::io::IsTerminal;
use std::process::Command;

/// Prefer real `$EDITOR` path (interactive TTY). Harness / tests / non-TTY → stub.
///
/// - `XYLITOL_TUI_EDITOR_STUB=1` → always stub
/// - `XYLITOL_TUI_REAL_EDITOR=1` → force real path (manual / targeted tests)
/// - `cfg(test)` → stub unless `XYLITOL_TUI_REAL_EDITOR=1`
pub fn prefer_real_external_editor() -> bool {
    if env_flag("XYLITOL_TUI_EDITOR_STUB") {
        return false;
    }
    if env_flag("XYLITOL_TUI_REAL_EDITOR") {
        return true;
    }
    if cfg!(test) {
        return false;
    }
    std::io::stdin().is_terminal()
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Strict resolve: non-empty `$VISUAL`, else non-empty `$EDITOR`. No nano/notepad default.
pub fn resolve_external_editor_command() -> Result<String, String> {
    resolve_external_editor_command_from(
        std::env::var("VISUAL").ok().as_deref(),
        std::env::var("EDITOR").ok().as_deref(),
    )
}

pub fn resolve_external_editor_command_from(
    visual: Option<&str>,
    editor: Option<&str>,
) -> Result<String, String> {
    for raw in [visual, editor].into_iter().flatten() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    Err("set $VISUAL or $EDITOR to use external editor (Ctrl+G)".into())
}

/// Write `initial` to a tempfile, spawn `editor_cmd`, return new text on exit 0.
/// Terminal must already be suspended by the caller.
pub fn run_external_editor_process_with_command(
    editor_cmd: &str,
    initial: &str,
) -> Result<Option<String>, String> {
    let path = std::env::temp_dir().join(format!(
        "xylitol-tui-editor-{}-{}.md",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    std::fs::write(&path, initial).map_err(|e| format!("write tempfile: {e}"))?;

    let mut parts = editor_cmd.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| "empty editor command".to_string())?;
    let mut args: Vec<&str> = parts.collect();
    let path_str = path.to_string_lossy();
    args.push(path_str.as_ref());

    let status = Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("spawn {program}: {e}"))?;

    let result = if status.success() {
        let new_content = std::fs::read_to_string(&path).map_err(|e| format!("read back: {e}"))?;
        Some(
            new_content
                .strip_suffix('\n')
                .unwrap_or(&new_content)
                .to_string(),
        )
    } else {
        None
    };
    let _ = std::fs::remove_file(&path);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_visual_then_editor() {
        assert_eq!(
            resolve_external_editor_command_from(Some("vim"), Some("nano")).unwrap(),
            "vim"
        );
        assert_eq!(
            resolve_external_editor_command_from(None, Some("nano")).unwrap(),
            "nano"
        );
        assert_eq!(
            resolve_external_editor_command_from(Some("  "), Some("emacs")).unwrap(),
            "emacs"
        );
    }

    #[test]
    fn resolve_missing_is_err_no_default() {
        let err = resolve_external_editor_command_from(None, None).unwrap_err();
        assert!(err.contains("$VISUAL") || err.contains("$EDITOR"), "{err}");
        let err = resolve_external_editor_command_from(Some(""), Some("   ")).unwrap_err();
        assert!(err.contains("Ctrl+G"), "{err}");
    }

    #[test]
    fn run_with_script_rewrites_file() {
        // Unique per invocation — avoid /tmp pid collisions under cargo test.
        let script =
            std::env::temp_dir().join(format!("xylitol-c650-editor-{}.sh", uuid::Uuid::new_v4()));
        std::fs::write(&script, "#!/bin/sh\nprintf 'edited' > \"$1\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }
        let cmd = script.to_string_lossy().into_owned();
        let out = run_external_editor_process_with_command(&cmd, "draft").unwrap();
        let _ = std::fs::remove_file(&script);
        assert_eq!(out.as_deref(), Some("edited"));
    }

    #[test]
    fn run_spawn_fail_is_err() {
        let err = run_external_editor_process_with_command(
            "/nonexistent/xylitol-editor-c650-test",
            "draft",
        )
        .unwrap_err();
        assert!(err.contains("spawn") || err.contains("No such"), "{err}");
    }

    #[test]
    fn run_nonzero_exit_keeps_none() {
        let out = run_external_editor_process_with_command("false", "draft").unwrap();
        assert_eq!(out, None);
    }
}
