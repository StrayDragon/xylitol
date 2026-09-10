//! Shell discovery and environment construction.
//!
//! Provides cross-platform bash finding and shell environment building.

use std::path::PathBuf;
use std::process::Command;

/// Shell configuration result.
#[derive(Debug, Clone)]
pub struct ShellConfig {
    /// Path to the shell binary.
    pub shell: PathBuf,
    /// Arguments to pass (e.g. `["-c"]` for direct command execution).
    pub args: Vec<String>,
}

/// Find bash on the current system.
///
/// Resolution order:
/// 1. User-specified custom shell path (optional parameter)
/// 2. Platform-specific discovery
///    - Windows: Git Bash in ProgramFiles → bash on PATH
///    - Unix: /bin/bash → which bash → sh fallback
pub fn find_bash(custom_shell: Option<&std::path::Path>) -> ShellConfig {
    if let Some(path) = custom_shell
        && path.exists()
    {
        return bash_config(path);
    }

    if cfg!(target_os = "windows") {
        find_windows_bash()
    } else {
        find_unix_bash()
    }
}

// ── Platform-specific ──────────────────────────────────────────────────

fn bash_config(path: &std::path::Path) -> ShellConfig {
    // Detect legacy WSL bash path (Windows\System32\bash.exe) which needs stdin transport
    let is_legacy_wsl = cfg!(target_os = "windows")
        && path
            .to_string_lossy()
            .to_lowercase()
            .contains("windows\\system32");

    let args = if is_legacy_wsl {
        vec!["-s".to_string()] // read from stdin
    } else {
        vec!["-c".to_string()] // command as argv
    };

    ShellConfig {
        shell: path.to_path_buf(),
        args,
    }
}

#[cfg(target_os = "windows")]
fn find_windows_bash() -> ShellConfig {
    // Try Git Bash in known locations
    let candidates = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];

    for candidate in &candidates {
        let path = std::path::Path::new(candidate);
        if path.exists() {
            return bash_config(path);
        }
    }

    // Try bash on PATH
    if let Some(path) = which("bash.exe") {
        return bash_config(&path);
    }

    // Last resort
    bash_config(std::path::Path::new("bash.exe"))
}

#[cfg(not(target_os = "windows"))]
fn find_windows_bash() -> ShellConfig {
    unreachable!("not Windows")
}

#[cfg(not(target_os = "windows"))]
fn find_unix_bash() -> ShellConfig {
    // Try /bin/bash
    let bash_path = std::path::Path::new("/bin/bash");
    if bash_path.exists() {
        return bash_config(bash_path);
    }

    // Try bash on PATH
    if let Some(path) = which("bash") {
        return bash_config(&path);
    }

    // Fallback to sh
    bash_config(std::path::Path::new("sh"))
}

#[cfg(target_os = "windows")]
fn find_unix_bash() -> ShellConfig {
    unreachable!("not Unix")
}

/// Find a binary on PATH using `which`.
fn which(name: &str) -> Option<PathBuf> {
    let output = Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_bash_unix_finds_bash_or_sh() {
        if cfg!(not(target_os = "windows")) {
            let config = find_bash(None);
            let name = config.shell.file_name().unwrap().to_string_lossy();
            assert!(
                name == "bash" || name == "sh",
                "expected bash or sh, got {name}"
            );
            assert_eq!(config.args, vec!["-c"]);
        }
    }
}
