---
depends_on: []
title: 修复 spec reviewed-through 单调性（老编号晚归档倒退 + 后续触碰未推进）
---

## Why

`# reviewed-through` 头按 855a740e 的语义是「已复核到的最大 change id」，只增不减。
两处破坏了单调性：

- c2520（老编号晚归档）归档时把 16 个 spec 的头统一改写为 `c2520`，其中
  agent-todo（c2865）、app-tui-host（c2838）、layer-architecture（c2890）被倒退。
- c2895 / c2900 / c2875 系列归档修改了 server-core、protocol-app、
  app-tui-fixed-zone 等 spec 内容却不推进该头，与 c2520 的写法互相矛盾。

## What Changes

- 按单调最大值修正 6 个 spec 头：
  - agent-todo → c2865（恢复倒退前值）
  - app-tui-host → c2895（max(c2838, c2520, c2895)）
  - layer-architecture → c2895（max(c2890, c2520, c2895)）
  - server-core → c2900（max(c2865, c2895, c2900)）
  - protocol-app → c2900（max(c2890, c2900)）
  - app-tui-fixed-zone → c2914（max(c2520, c2875…c2914)）
- 不改任何规则正文、场景与 verified-by 锚点；纯元数据修正。

## Impact

仅 `llmanspec/specs/**` 六个文件的 `# reviewed-through` 头；产品代码零改动。
