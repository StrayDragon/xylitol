---
depends_on: []
title: "Agent preset resume diff"
---

## Why

agent preset 是 boot 级参数，不是 session 级；`SessionContext` 只存 messages / thinking_level / model，不存 preset。resume 会用 host 当前 preset。冻结 preset 不可靠：下一次 resume 时同名 preset 可能不存在或已变化。选择 Option B：resume 用当前 preset，按 diff 注入必要上下文。

## What Changes

- session header 存 preset name + fingerprint + minimal metadata（tool set fingerprint / system prompt hash）。
- resume 时加载当前 preset，比较 fingerprint：
  - 相同：继续，不注入。
  - 不同：应用当前 preset；diff system prompt / tool set；reconcile MCP claims；注入简洁 delta（新增/移除工具、指令变化摘要），不做全量重注入。
  - preset 不存在：回退默认 preset 并提示；不得静默用错。
- per-session preset binding：agent assembly 按 session 的 preset 构建，不用 host boot preset 替代。
- diff 低成本：fingerprint 先短路；只对变化项生成 delta；缓存 diff 结果。
- TUI 通知：resume 时显示 preset changed / fallback；审计。
- 推荐 mid-session 不改 preset；变更只发生在 resume。

## Capabilities

- `agent-runtime`：per-session preset binding、resume diff、tool set reconcile。
- `protocol-app`：session header preset metadata。
- `app-tui-fixed-zone`：preset changed 通知。

## Impact

runtime agent assembly、session store、MCP claims、TUI。

## Further Notes

- 用户选择 Option B；Option A（冻结）因 preset 可能不存在而不可靠；Option C（历史快照）不直观。
- 与 `c3060-add-mcp-tool-catalog` 的 tool set 过滤 / reconcile 协同。
- 依赖：无；可与 `c3050` 的 session header 协同。

## Open Questions

- diff 粒度：只 diff tool set / system prompt，还是也 diff capability overlay。
- delta 注入形式：system 消息 / 用户可见通知 / 两者。
- preset 不存在时的 fallback 策略与用户提示。
- mid-session preset 变更是否允许：推荐不允许。
