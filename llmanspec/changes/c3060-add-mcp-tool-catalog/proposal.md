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
- resume 时按当前 preset 重建 agent tool set，reconcile MCP claims（新 server 连接、未用 release）。
- host 维护懒连接：未启动的 server 在需要时连接；不需要 per-preset 连接，除非 config fingerprint 不同。
- tool list changed：刷新 catalog；下一轮失效并重建 agent tool set。
- TUI snapshot 按 scope 展示；`/mcp` 面板显示 scope/key/tool count。
- 假设同 config 下 server tool list 稳定；若按连接变化，catalog 按 key 保留差异。

## Preset 变更处理

- 选择 Option B：resume 使用当前 preset，不冻结旧 preset。理由：冻结的 preset 下一次 resume 可能不存在或已变化。
- 独立 change：`c3100-add-agent-preset-resume-diff` 负责 session header preset metadata 与 resume 全量重注入。
- 本 change 负责 tool set 过滤与 reconcile：按当前 preset 重建 tool set；MCP claims 增删；tool list changed 刷新。
- 粗粒度：preset 任意变动就全量重注入；不做 diff / delta。
- preset 不存在：回退默认 preset 并提示；不得静默用错。

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

- resume 后 preset 变化：由 `c3100` 处理；本 change 只负责 tool set reconcile。
- 同 server 名不同 config 是否包含 credential：推荐包含。
