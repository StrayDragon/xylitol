---
depends_on: []
title: "Agent preset resume diff"
---

## Why

agent preset 是 boot 级参数，不是 session 级；`SessionContext` 只存 messages / thinking_level / model，不存 preset。resume 会用 host 当前 preset。冻结 preset 不可靠：下一次 resume 时同名 preset 可能不存在或已变化。选择 Option B：resume 用当前 preset。

## What Changes

- session header 存 preset name + fingerprint。
- resume 时加载当前 preset，比较 fingerprint：
  - 相同：继续，不重注入。
  - 不同：应用当前 preset；**全量重新注入上下文**（粗粒度，不做 diff / delta）。
  - preset 不存在：回退默认 preset 并提示；不得静默用错。
- per-session preset binding：agent assembly 按 session 的 preset 构建，不用 host boot preset 替代。
- TUI 通知：resume 时显示 preset changed / fallback；审计。
- 推荐 mid-session 不改 preset；变更只发生在 resume。

## Capabilities

- `agent-runtime`：per-session preset binding、resume full re-injection。
- `protocol-app`：session header preset metadata。
- `app-tui-fixed-zone`：preset changed 通知。

## Impact

runtime agent assembly、session store、MCP claims、TUI。

## Further Notes

- 用户选择 Option B；粗粒度处理：preset 任意变动就全量重注入。
- 与 `c3060-add-mcp-tool-catalog` 的 tool set 过滤 / reconcile 协同。
- 依赖：无；可与 `c3050` 的 session header 协同。

## Open Questions

- preset 不存在时的 fallback 策略与用户提示。
- mid-session preset 变更是否允许：推荐不允许。
- 全量重注入的成本与上下文窗口处理（compaction 是否需要先跑）。
