---
depends_on: []
title: 恢复 c2520 归档时被整体倒退的其余 11 个 spec reviewed-through 头
---

## Why

c2520（老编号晚归档）把 15 个 spec 的 `# reviewed-through` 统一覆盖写为
c2520，旧值全部更大（c2838×1 / c2865×10 / c2890×4），15 个头全部数值倒退。
c2916 只修复了其中 3 个（agent-todo / app-tui-host / layer-architecture，
后两者叠加了后续 change 触碰）；经 a11e6c48 前值逐文件核对，其余 11 个
仍停在倒退值 c2520。该头按 855a740e 语义是「已复核到的最大 change id」，
只增不减；倒退使 staleness 把已复核工作重新报 STALE。

## What Changes

- 按倒退前值恢复 11 个头（这批文件在 a11e6c48 之后无 change 触碰，
  前值即单调正确值）：
  - → c2865：app-tui-ask、app-tui-bridge、app-tui-commands、app-tui-input、
    app-tui-session-tree、app-tui-transcript、app-tui-trust、infra-mcp
  - → c2890：app-tui、app-tui-design-playground、test-bdd
- 纯元数据修正，不改任何规则正文、场景与 verified-by 锚点。

## Impact

仅 `llmanspec/specs/**` 11 个文件的 `# reviewed-through` 头；产品代码零改动。
