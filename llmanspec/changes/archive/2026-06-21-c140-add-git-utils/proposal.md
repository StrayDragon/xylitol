---
change_id: c140-add-git-utils
depends_on: []
---

# c140-add-git-utils: 添加 Git 工具模块

## Why

pi 提供了 `git.ts`（226 行），支持：
- Git 仓库元数据发现（commondir、HEAD、worktree）
- 当前分支检测（含 detached HEAD 处理）
- 分支变更文件系统监控（FSWatcher）
- Git URL 解析（SCP-like、https、ssh、git 协议，含 hosted-git-info 集成）

xylitol 当前没有任何 Git 相关工具。

## What Changes

在 `src/infra/git/` 下创建新模块，可选的 `infra-git` feature：

1. `mod.rs` — 公共 API
2. `repo.rs` — 仓库检测（.git 目录/文件遍历，worktree 支持）
3. `branch.rs` — 当前分支检测
4. `url.rs` — Git URL 解析（SCP、HTTPS、SSH、git 协议）

## Capabilities

- 新增 capability: `git-utils`

## Impact

- 新增可选 feature `infra-git`
- 约 250-350 行新 Rust 代码
