---
depends_on:
  - c3206-refactor-session-graph
title: "Add workspace backend"
---

## Why

Agent 改代码需要分支级工作区：当前 session 只绑定 cwd，没有 mount / commit / branch / merge / rewind。c3206 只留 `WorkspaceRef { backend, commit }` 字段（manifest / branch），没有后端。

## What Changes

- Workspace 端口：mount、commit、branch、merge、rewind、diff。
- Branch 绑定 WorkspaceRef（backend + commit）；工具在 mount 内运行；commit 作为事实追加。
- 后端：local dir（兼容现状）、git worktree（先做）、LayerFS sidecar（后做）。
- git worktree：每 branch 一个 worktree；冲突 / 合并走 git 语义；target/ 不随树提交。
- LayerFS：只宜外进程 sidecar（无 Rust API、Store 只跑 Linux 本地盘、macOS 需 Docker、无 rewind、publish 覆盖语义、durability 不承诺）；不进核心运行时。

## Scope

- 不在 c3206 内实现；c3206 只留 `WorkspaceRef { backend, commit }` 字段。
- 不做中心化工作区服务；先本机。

## Related

- #7（LayerFS 作为 workspace-checkpoint 的调研留档；结论 defer，本 draft 更新为 sidecar 候选）
- #13（Cloud Agent，搁置）
- #74（nsh 原生端；端侧工作区展示）

## Open Questions

- worktree 粒度：每 branch 还是每 session。
- merge / rewind 的产品语义与冲突处理。
- LayerFS 2.0 形状与跨平台限制。

## 2026-11-09 复审

结论：仍必要。当前 session 只绑定 cwd/workspace set，fork 共享 cwd；`WorkspaceRef` 字段存在但恒为 None。建议排在 c3207/c3208 之后，先 git worktree，LayerFS 仅 sidecar。
