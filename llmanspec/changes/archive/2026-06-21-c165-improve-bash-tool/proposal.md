---
change_id: c165-improve-bash-tool
depends_on:
  - c135-add-shell-process-mgmt
---

# c165-improve-bash-tool: 增强 Bash 工具 — Shell 钩子与进程管理

## Why

pi 的 bash 工具在 `bash.ts`（453 行）中提供了比 xylitol 更完善的执行能力：

- **BashSpawnHook**：在 spawn 前/后的回调钩子，供扩展系统使用
- **BashOperations 接口**：将 bash 操作抽象为 trait，支持注入模拟
- **详细的超时处理**：逐级降级（SIGTERM → 5s → SIGKILL）
- **增强的环境变量**：注入 PATH 中的 bin 目录
- **跨平台 shell 发现**：Windows Git Bash、Unix bash/sh 自动检测

xylitol 的 `bash.rs`（209 行）和 `bash_executor.rs`（292 行）实现了基础执行，但缺少 hooks 接口和系统化的进程管理。

## What Changes

增强 `src/agent/tools/bash.rs` 和 `src/agent/bash_executor.rs`：

1. **BashOperations trait**：将 bash 执行抽象为 trait，支持模拟/测试
2. **Spawn hooks**：在 bash 执行前后添加回调点
3. **超时逐级降级**：SIGTERM → 等待 → SIGKILL
4. **跨平台 bash 发现**：利用 c135 的 process-mgmt 模块
5. **环境变量注入**：自动添加 bin 目录到 PATH

## Capabilities

- 修改现有 capability: `bash-execution`
- 修改现有 capability: `tool-system`

## Impact

- 依赖 c135（shell process mgmt）
- 修改现有文件，无新 feature flag
- 约 150-250 行代码变更
