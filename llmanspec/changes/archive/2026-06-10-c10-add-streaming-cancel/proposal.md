---
change_id: c10-add-streaming-cancel
title: "为 Grep/Find 工具添加进程取消支持"
depends_on: []
status: active
priority: 10
---

# 变更提案：工具进程取消

## Why

bash 工具已有正确的 cancel+kill_tree 实现，但 grep 和 find 使用 `tokio::task::spawn` 包装 `Command::new().output()`，在 cancel 路径上无法访问子进程来 kill 它。

```rust
// grep — 当前（丢失子进程句柄）
let output_fut = tokio::task::spawn(async move { Command::new("rg").args(&rg_args).output().await });
let child_result = tokio::select! {
    _ = cancel.cancelled() => return Err(XyToolError::Aborted), // rg 继续运行！
    result = output_fut => result,
};

// bash — 已有（正确模式）
let child = Command::new(shell).arg("-c").arg(cmd).spawn()?;
let pid = child.id();
let output_fut = child.wait_with_output();
tokio::select! {
    _ = cancel.cancelled() => { kill_tree(pid).await; return Err(XyToolError::Aborted); }
    r = timeout_fallible(output_fut, timeout_dur) => r,
}
```

pi 使用流式输出的 AbortController 模式。在 Rust 中由于 grep/find 是调用外部二进制文件，无法流式中断，但可以通过 spawn + kill on drop 实现清理。

## What Changes

### Grep：从 `spawn` 改为 `spawn` + kill

1. `Command::new("rg").spawn()` → 获取 `Child`
2. 记录 pid
3. 使用 `tokio::select! { cancel, child_output, timeout }`
4. cancel 分支：`kill_tree(pid)` 后返回 Aborted

### Find：相同模式

`fd` 工具使用与 grep 完全相同的修复。

### kill_tree helper 提升

将 `bash.rs` 中的 `kill_tree()` 提升到共享位置（`tools/mod.rs` 或新建 `tools/process.rs`），供 grep/find/bash 共用。

## Capabilities

- **tool-system**: 工具进程取消

## Impact

- `src/agent/tools/grep.rs`: spawn+kill_tree 替代 tokio::task::spawn+drop
- `src/agent/tools/find.rs`: 同上
- `src/agent/tools/bash.rs`: kill_tree 移动到共享模块
- `src/agent/tools/mod.rs`: 导出 kill_tree
- `tests/features/{grep,find}.feature`: 已有 cancel 场景，需要更新步骤定义
