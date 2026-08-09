//! Cross-platform process group termination.
//!
//! Extends the basic `tools/process::kill_tree` with Windows support
//! and a unified API matching pi's `shell.ts`.

use std::process::Command;

/// Kill a process and all its children.
///
/// On Unix: sends SIGKILL to the process group (negative PID).
/// On Windows: uses `taskkill /F /T`.
pub fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }

    #[cfg(unix)]
    {
        // Kill the entire process group
        let _ = Command::new("kill")
            .args(["-9", &format!("-{pid}")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_process_tree_zero_pid_is_noop() {
        // Should not panic or fail
        kill_process_tree(0);
    }
}
