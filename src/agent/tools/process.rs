//! Shared process helpers for external-process tools (bash, grep, find).

/// Kill a process tree by process group ID.
///
/// Uses `kill -9 -<pid>` to target the entire process group,
/// ensuring all grandchildren are terminated.
pub(crate) async fn kill_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    let _ = tokio::process::Command::new("kill")
        .args(["-9", &format!("-{pid}")])
        .output()
        .await;
}
