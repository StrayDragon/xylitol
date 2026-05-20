//! Terminal keyboard enhancement setup/teardown.
//!
//! We enable crossterm's kitty keyboard protocol support so modified keys like
//! Shift+Enter can be distinguished from Enter (Codex-style composer behavior).

use std::fmt;
use std::io::stdout;

use crossterm::Command;
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;

const DISABLE_KEYBOARD_ENHANCEMENT_ENV_VAR: &str = "XYLITOL_TUI_DISABLE_KEYBOARD_ENHANCEMENT";

pub(crate) fn enable_keyboard_enhancement() {
    if keyboard_enhancement_disabled() {
        return;
    }

    let _ = execute!(
        stdout(),
        DisableModifyOtherKeys,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    );

    if tmux_should_enable_modify_other_keys() {
        let _ = execute!(stdout(), EnableModifyOtherKeys);
    }
}

pub(crate) fn restore_keyboard_enhancement_stack() {
    let _ = execute!(
        stdout(),
        PopKeyboardEnhancementFlags,
        DisableModifyOtherKeys
    );
}

pub(crate) fn reset_keyboard_reporting_after_exit() {
    let _ = execute!(
        stdout(),
        PopKeyboardEnhancementFlags,
        ResetKeyboardEnhancementFlags,
        DisableModifyOtherKeys
    );
}

pub(crate) fn keyboard_enhancement_disabled() -> bool {
    std::env::var(DISABLE_KEYBOARD_ENHANCEMENT_ENV_VAR)
        .ok()
        .and_then(parse_bool_env)
        .unwrap_or(false)
}

fn parse_bool_env(value: String) -> Option<bool> {
    match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        v if v.eq_ignore_ascii_case("true") => Some(true),
        v if v.eq_ignore_ascii_case("false") => Some(false),
        v if v.eq_ignore_ascii_case("yes") => Some(true),
        v if v.eq_ignore_ascii_case("no") => Some(false),
        _ => None,
    }
}

fn tmux_should_enable_modify_other_keys() -> bool {
    running_in_tmux_session()
        && matches!(
            read_tmux_extended_keys_format().as_deref(),
            Some("csi-u") | None
        )
}

fn running_in_tmux_session() -> bool {
    std::env::var("TMUX").is_ok() || std::env::var("TMUX_PANE").is_ok()
}

fn read_tmux_extended_keys_format() -> Option<String> {
    for args in [
        ["display-message", "-p", "#{extended-keys-format}"],
        ["show-options", "-gqv", "extended-keys-format"],
    ] {
        let output = std::process::Command::new("tmux")
            .args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            continue;
        }

        let value = String::from_utf8(output.stdout).ok()?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        return Some(trimmed.to_string());
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResetKeyboardEnhancementFlags;

impl Command for ResetKeyboardEnhancementFlags {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[<u")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "keyboard enhancement reset is not implemented for the legacy Windows API",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnableModifyOtherKeys;

impl Command for EnableModifyOtherKeys {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[>4;2m")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "modifyOtherKeys enable is not implemented for the legacy Windows API",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DisableModifyOtherKeys;

impl Command for DisableModifyOtherKeys {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[>4;0m")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "modifyOtherKeys reset is not implemented for the legacy Windows API",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}
