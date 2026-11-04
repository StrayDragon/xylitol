---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP tool catalog per scope and agent preset"
---

## Why

MCP 工具当前进全局 `HostRegistries.tools`；adapter 忽略 `XyToolCtx`。不同 agent preset 的工具集不同；per-scope 连接可能有不同工具；tool list changed 未刷新。

## What Changes

- catalog 按 connection key 建立：tools / prompts / resources。
- agent preset 过滤：按 `tools` 列表 / capability overlay 在 session 侧过滤；工具名保持 `mcp__server__tool`。
- connection key 含 config fingerprint；同名不同 config（args/env/url/headers）必须独立连接与 catalog。
- agent preset 在 session 生命周期内冻结；resume 可能恢复不同 preset：重建 agent tool set，reconcile MCP claims（新 server 连接、未用 release）。
- host 维护懒连接：未启动的 server 在需要时连接；不需要 per-preset 连接，除非 config fingerprint 不同。
- tool list changed：刷新 catalog；下一轮失效并重建 agent tool set。
- TUI snapshot 按 scope 展示；`/mcp` 面板显示 scope/key/tool count。
- 假设同 config 下 server tool list 稳定；若按连接变化，catalog 按 key 保留差异。

## Preset 变更处理

- 现状：agent preset 是 boot 级参数（`agent_profile`），不是 session 级；`SessionContext` 只存 messages / thinking_level / model，不存 preset。resume 会用 host 当前 preset，不能还原旧 preset。
- 选项 A（推荐）：preset 在 session 生命周期内冻结；session header 存 preset name + fingerprint。resume 指纹相同则继续；不同则要求显式“用新 preset 重建 session”（新 session / fork），重建时重新注入上下文。
- 选项 B：resume 直接用新 preset。system prompt / tool set 变化，用户容易困惑；不推荐。
- 选项 C：从历史快照还原旧 preset。可复现但陈旧；不符合用户直觉；不推荐。
- 底层不灵活：agent assembly 在 host/boot 级；需要 per-session preset binding，才能按 session 选择 preset 并 reconcile tool set。

## Capabilities

- `infra-mcp`：catalog。
- `agent-runtime`：agent preset 过滤、resume reconcile、tool set 失效。
- `app-tui-fixed-zone`：MCP 快照展示。

## Impact

MCP plugin、runtime agent tools、host registry、TUI。

## Further Notes

- 依赖 `c3055-add-mcp-scope-pool`。
- 后续：`c3070-add-mcp-prompts-resources`、`c3075-add-mcp-diagnostics`。
- 官方 2026-07-28 是 stateless；连接级 tools/list 差异主要来自 legacy/实现差异；config fingerprint 是隔离边界。
- 软过滤：preset 只过滤工具集，不新建连接；不同 config 才独立连接。

## Open Questions

- resume 后 preset 变化：是否允许；如何 reconcile claims；是否需要 session 级 preset 冻结记录。
- 同 server 名不同 config 是否包含 credential：推荐包含。
- per-session preset binding 是否需要独立 change。
