//! Cross-platform process group termination.
//!
//! Extends the basic `tools/process::kill_tree` with Windows support
//! and a unified kill-tree API for bash / child process cleanup.

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
        // 直接 syscall 杀进程组：不依赖 /bin/kill 的 PATH 可用性（CI runner
        // 环境 PATH 可能不含系统 bin，外部命令缺失会静默失败——CI 偶发根因）。
        let pid = pid as i32;
        // SIGKILL 整组（负 pgid）；若目标非组首领，补杀单 pid 兜底。
        unsafe { libc::kill(-pid, libc::SIGKILL) };
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
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
