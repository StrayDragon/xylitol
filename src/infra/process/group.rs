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

/// Track child PIDs so they can be killed on shutdown.
static TRACKED_PIDS: std::sync::LazyLock<std::sync::Mutex<Vec<u32>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a child PID for cleanup on shutdown.
pub fn track_child(pid: u32) {
    if let Ok(mut pids) = TRACKED_PIDS.lock() {
        pids.push(pid);
    }
}

/// Remove a child PID from the tracking set (e.g. after clean exit).
pub fn untrack_child(pid: u32) {
    if let Ok(mut pids) = TRACKED_PIDS.lock() {
        pids.retain(|&p| p != pid);
    }
}

/// Kill all tracked child processes (called on shutdown).
pub fn kill_tracked_children() {
    let pids: Vec<u32> = if let Ok(pids) = TRACKED_PIDS.lock() {
        pids.clone()
    } else {
        return;
    };
    for pid in pids {
        kill_process_tree(pid);
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

    #[test]
    fn test_track_untrack_roundtrip() {
        track_child(42);
        track_child(99);

        let tracked = TRACKED_PIDS.lock().unwrap().clone();
        assert!(tracked.contains(&42));
        assert!(tracked.contains(&99));

        untrack_child(42);
        let tracked = TRACKED_PIDS.lock().unwrap().clone();
        assert!(!tracked.contains(&42));
        assert!(tracked.contains(&99));

        // Cleanup
        TRACKED_PIDS.lock().unwrap().clear();
    }

    #[test]
    fn test_kill_tracked_children_does_not_panic() {
        track_child(99999); // Non-existent PID — should not panic
        kill_tracked_children();
        TRACKED_PIDS.lock().unwrap().clear();
    }
}
