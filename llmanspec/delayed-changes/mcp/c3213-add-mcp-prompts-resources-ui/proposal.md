---
depends_on:
  - c3070-add-mcp-prompts-resources
title: "Add MCP prompts and resources UI"
---

## Why

c3070 已把 prompts / resources 暴露为模型工具（`mcp__server__prompt__name`、`mcp__server__resource__read`），但 TUI 的 slash prompt 选择器、`@` 资源选择器、`subscriptions/listen` 与资源读取权限/缓存均未实现。用户仍无法从 TUI 主动选择 prompt 或附加 resource。

## What Changes

- prompts/list + prompts/get 的 TUI slash 命令：列出当前 scope 的 prompt，选择后注入为 user-controlled 消息。
- resources/list + resources/read + resources/templates/list 的 `@` 选择器与上下文附件。
- `subscriptions/listen`：资源更新通知进入 catalog 失效/刷新。
- 资源读取权限：走 trust 门禁；外部资源与本地文件区分。
- 缓存策略：按 connection key / URI 缓存，明确失效条件。
- per-scope 暴露：Shared / Workspace / Session 的可见性。

## Scope

- 不改变 c3070 的模型工具命名与语义。
- 不实现新的 MCP 协议方法；只接 `subscriptions/listen` 与现有 list/read。
- TUI 交互复用现有 Choice / fixed-zone 槽位。

## 延后原因

- 模型工具已满足 agent 使用；TUI slash / `@` 是新增交互，UX 与权限策略未定。
- `subscriptions/listen` 需要 rmcp 通知接入与 catalog 失效策略；主线先交付静态 catalog。
- 资源缓存与 trust 门禁的交叉需要单独设计。

## 启动前置条件

- c3070 已归档（当前已满足）。
- 确定 TUI slash 命令 SSOT 与 Choice 槽位。
- 确定 `@` 选择器交互（搜索、预览、附加、移除）。
- 确定资源读取的 permission / trust 模型。
- 验证 rmcp 3.8 的 resource subscription / notification 支持。
- 确定缓存键与失效条件。

## Related

- c3070-add-mcp-prompts-resources（已归档）
- c3075-add-mcp-diagnostics（面板显示）
- c3050-add-host-workspace-binding（workspace set）

## Open Questions

- `@` 资源缓存是否跨 session。
- prompt 是 user-controlled、resource 是 application-controlled 的边界如何呈现。
- 资源读取失败与权限拒绝的 UI。
- subscription 通知频率与 catalog 刷新节流。
