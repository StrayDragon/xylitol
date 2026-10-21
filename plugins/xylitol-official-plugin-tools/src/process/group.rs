pub fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }

    #[cfg(unix)]
    {
        let pid = pid as i32;
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
        kill_process_tree(0);
    }
}
