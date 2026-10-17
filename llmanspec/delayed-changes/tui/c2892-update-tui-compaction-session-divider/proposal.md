---
depends_on: []
---

# 压缩在对话条目里画成分割线（延后）

> **⚠️ deferred（2026-10-10）**：搁置在 `llmanspec/delayed-changes/tui/`。c2860 整块可点折叠 **不**改 Compaction 视觉；只保证迁命中表时三角列行为不 regress。落实时经 propose / ff 迁回 `changes/`。

## Why

今日 Complete 态是 `[compaction] Compacted from N tokens` + 三角 + `(Alt+E to expand)`，Pending 是 transcript 里一行 muted `[compaction] Compacting…`，进行中短词还在状态条。它既不像工具叶块，也不像压缩前后会话的边界，点选语义跟着别扭（全局 `compaction_expanded`，空选点正文会误翻）。

压缩在用户直觉里是 **压缩前会话 | 压缩后会话** 的分割，不是又一个可折叠工具块。

## What Changes（意向，未钉）

进行中（横向按当前列宽实时居中；`spinner` 走既有忙碌指示，不新开装饰）：

```
---------------[spinner] compacting --------------------
```

完成后（分割线保留；详情不要再伪装成 Alt+E 叶块）：

```
----------------------------------------------------------
```

可选后切：分割线内 **单行**流式展示模型给出的 compact 摘要（可回看），比展开一块 muted 散文更直观。

- 进行中 MUST 出现在对话条目主滚动区，MUST NOT 只靠状态条短词 `Compacting`。
- 分割线宽度 MUST 随终端列宽计算，标签居中。
- 完成后分割线仍是边界，不是带三角的工具块。点选 / 悬停规则在本票重画时一并定（预期：无可折叶块，c2860 的三角列可退役）。
- 产品 compaction 呈现以 `src/app/tui` 代码与 `DESIGN.md` 为准（今日写「进行中走 status、完成是可折块」）。

## 非目标

- 不改压缩算法、cut 点、token 估计。
- 不在 c2860 里预画假分割线。
- 不把压缩摘要伪装成工具 rail 块。

## Capabilities（意向）

- `app-tui-transcript`（r1359 三角列 / 完成块形态）
- `src/app/tui/DESIGN.md`（compaction 呈现）

## Impact

- 压缩在滚动区里一眼是「会话被切过」，不是又一个 `[compaction]` 工具。
- c2860 把 Compaction 排除在叶块空选点正文之外，等本票重画后再收点选。

## Further Notes

- 现行 paint：`paint_compaction_block`（Pending 一行 muted；Complete 三角 + Alt+E）。
- 交叉：`delayed-changes/next-todo/c1897` 是压缩 × 状态条堆积策略，不是本视觉。
