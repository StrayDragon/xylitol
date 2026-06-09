# Design: 工具进程取消

## 问题

当前 grep/find 使用 `tokio::task::spawn` 运行子进程，cancel 触发时无法访问子进程来 kill：

```rust
// 错误模式：spawn 丢失了 Child handle
let output_fut = tokio::task::spawn(async move {
    Command::new("rg").output().await
});
tokio::select! {
    _ = cancel.cancelled() => return Err(XyToolError::Aborted), // 子进程继续运行！
    result = output_fut => { /* ... */ }
}
```

## 修复

使用 bash 已有的模式：spawn 子进程、记录 pid，cancel 时 kill 进程组：

```rust
let mut child = Command::new("rg").args(&args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
let pid = child.id().unwrap_or(0);

tokio::select! {
    _ = cancel.cancelled() => {
        kill_tree(pid).await;
        return Err(XyToolError::Aborted);
    }
    status = child.wait() => { /* 读 stdout/stderr */ }
    _ = sleep(TIMEOUT) => {
        kill_tree(pid).await;
        return Err(XyToolError::Timeout(TIMEOUT));
    }
}
```

## kill_tree 共享

bash 中已有 `kill_tree(pid)` 实现。将其移至 `src/agent/tools/process.rs`（新文件），用一个模块级的 `pub(crate) async fn kill_tree(pid: u32)` 供 grep/find/bash 共用。

## 风险

| 风险 | 缓解 |
|------|------|
| pid 回收竞态 | 非问题——kill_tree 尝试 kill，已结束则忽略错误 |
| stdout/stderr 管道缓冲区满 | 子进程在输出大量数据时可能阻塞于管道写入。`child.wait()` 仅等待进程结束而不读取输出。需要改用 `wait_with_output()` |
