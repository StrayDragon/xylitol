---
change_id: c135-add-shell-process-mgmt
depends_on: []
---

# c135-add-shell-process-mgmt: 添加 Shell 进程管理模块

## Why

pi 提供了 `shell.ts` + `child-process.ts`（共约 362 行），提供：
- 跨平台 bash 发现（Windows Git Bash、Unix /bin/bash、sh 回退）
- Shell 环境构建（PATH 注入 bin 目录）
- 进程组终止（SIGKILL 跨平台）
- 分离子进程跟踪与清理
- 子进程的可靠等待（管道滞留保护）

xylitol 的 `bash_executor.rs` 只有基础执行能力，缺少进程组管理、bash 发现和 shell 环境构建。

## What Changes

1. 新增 `src/infra/process/mod.rs` — 进程管理公共 API
2. 新增 `src/infra/process/shell.rs` — bash 发现 + shell 环境
3. 新增 `src/infra/process/group.rs` — 进程组管理（kill tree）
4. 新增 `src/infra/process/child.rs` — 子进程可靠等待（管道滞留保护）
5. 增强 `src/agent/bash_executor.rs` — 集成进程组管理

## Capabilities

- 新增 capability: `process-mgmt`
- 修改现有 capability: `bash-execution`（增强 bash_executor.rs）

## Impact

- 内置模块（无 feature gate），因为与现有 bash_executor 深度集成
- 约 300-400 行新 Rust 代码
