---
depends_on: [c3060-add-mcp-tool-catalog, c3206-refactor-session-graph]
title: "Agent profile resume diff"
branch: main
base_branch: origin/main
base_sha: 78d077258664b34c5b9147336d76994cb6f448b7
---

## Why

agent profile 是 boot 级参数，不是 session 级；`SessionContext` 只存 messages / thinking_level / model，不存 profile。resume 会用 host 当前 profile。冻结 profile 不可靠：下一次 resume 时同名 profile 可能不存在或已变化。选择 Option B：resume 用当前 profile。
c3206 已落地：`SessionHeader` 仍只有 cwd / parent / fork_at；manifest formatVersion 仍 7；`SessionContext` 仍只有 messages / thinking / model；profile metadata 需新增可选字段，不升 v8。

## What Changes

- session header / manifest 存 profile name + fingerprint（c3206 未提供；新增可选字段，formatVersion 保持 7）。
- resume 从 c3206 graph head / build_context 读取上下文，加载当前 profile，比较 fingerprint：
  - 相同：继续，不重注入。
  - 不同：检测提示；用户选择后处理。
  - profile 不存在：回退默认 profile 并提示；不得静默用错。
- 检测提示复用 ask 的 TUI 形态（Choice / 表单），提示 profile 变化与风险：
  1. Compact 后继续（推荐）：先 compaction，再全量重新注入上下文。
  2. 不 compaction 继续：全量重新注入上下文；风险（上下文溢出 / 质量下降）由用户承担。
  3. 取消 resume：保持 session 不变。
- 全量重新注入是粗粒度策略；不做 diff / delta。
- per-session profile binding：agent assembly 按 session 的 profile 构建，不用 host boot profile 替代。
- TUI 通知：resume 时显示 profile changed / fallback；审计。
- 推荐 mid-session 不改 profile；变更只发生在 resume。

## Capabilities

- `agent-runtime`：per-session profile binding、resume full re-injection。
- `protocol-app`：session header profile metadata。
- `app-tui-fixed-zone`：profile changed 提示与 Choice。

## Impact

runtime agent assembly、session store、MCP claims、TUI。

## Further Notes

- 用户选择 Option B；粗粒度处理：profile 任意变动就全量重注入。
- 检测提示复用 ask TUI 形态，不用 MCP elicitation slot。
- 依赖 `c3060-add-mcp-tool-catalog`：resume 按当前 profile 重建 tool set / reconcile MCP claims；本 change 负责 session header fingerprint、提示与全量重注入。
- c3206 已归档：SessionHeader 无 profile；manifest formatVersion 7；profile metadata 为新增可选字段，不升 v8。
- resume 全量重注入以 graph head / build_context 为准，不回退线性 load。
- 不与 `c3055-add-mcp-scope-pool` 依赖；profile 语义与 MCP scope pool 无交集。

## Open Questions

- profile 不存在时的 fallback 策略与用户提示。
- 全量重注入的成本与上下文窗口处理；compaction 触发条件。
- mid-session profile 变更是否允许：推荐不允许。
