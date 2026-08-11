---
depends_on:
  - c2020-add-package-tui-mouse-input
  - c2030-add-tui-fold-leader-digit-toggle
blocks:
  - c2050-update-activity-fold-mouse-leader
---

# 鼠标点击折叠三角 + 折叠标记字形

> **一句话**：点折叠头行标记（倒三角/三角）toggle 单块；Unicode 用更清晰的折叠字形，Ascii 保留 `>`/`v`。

## Why

键盘 leader（`c2030`）解决定点折叠的无鼠标路径；有鼠标时，点击标记是最低学习成本（对齐 Web 同源「点 ▶/▼」心智，见 `docs/roadmaps/Web与TUI同源.md`）。`c1760` 已预留「segment/entry ↔ 行距」缝，但引擎无 Mouse、产品无 hit-test。

字形：今日 `GlyphSet::fold/unfold` = `▶`/`▼`（Ascii `>`/`v`）。用户希望「更好看的倒三角/三角」代表可折叠——在**不破坏宽度/旁注**前提下微调（候选见调研；propose 目视拍板）。

## What Changes

1. **依赖 `c2020`**：产品开 mouse capture（至少在需要点击折叠时）；点击落在折叠标记（或整行头，策略钉）→ toggle 对应 entry/segment。
2. **Hit-test 表**：render 时维护 `fold_hit_regions: [(y_range, x_range, target_id)]`（或行号→id）；仅头行标记列优先，避免点正文误触。
3. **与 `c2030` 共用块级覆盖表**：点击与数字键同一语义动作（跨面动作 id 意向：`fold.toggle(target)`）。
4. **字形**：更新 `GlyphSet`（产品）折叠标记；宽度 MUST 仍为单列可视宽；Ascii 回退不变或同步美化。
5. **非目标**：拖拽选区、滚轮改 scroll（另案）；L2/L3 段点击细节由 `c2050` 收口。

## Capabilities（意向）

- `app-tui-transcript` — hit-test + toggle
- `app-tui-host` — Mouse 路由到 scrollback（Editor 未抢时）
- 产品 `GlyphSet` — 折叠标记

## Impact

| 层 | 影响 |
|---|---|
| 渲染 | 每帧可附带 hit 表；须与 paint-cache 同代指纹，避免错点 |
| 输入 | Mouse Down/Up 去抖；忽略 move；Shift+click 策略跟 `c2020` |
| 性能 | hit 表 O(可见折叠头)；toggle 只失效目标 entry 及之后行高（对齐 ath25） |

## 依赖与排序

```text
c2020 ──┐
        ├──depends→ [本 change c2040] ──blocks→ c2050
c2030 ──┘
```

- **硬依赖** `c2020`（Mouse 事件）+ `c2030`（同一 per-block 覆盖表 / `fold.toggle`；禁止本 change 另起第二套状态）。
- **`blocks`**：`c2050`（段级点击语义）。

## Out of scope

- Leader 编号模式（`c2030`）
- Activity L2/L3 摘要文案（`c1760`）
- 改差分引擎算法

## Open Questions

1. 点击命中：仅标记单元格 vs 整条摘要/头行？
2. 折叠字形最终选：`▾`/`▸`、`▼`/`▶`、`▽`/`▷`，或其他？
3. mouse 默认开还是「首次需要点击折叠时再 Enable」？

## Ethics

- risk_level: low–medium
- prohibited_actions: 点正文大面积误 toggle；无 `c2020` 透传策略就默认常开 capture
- required_evidence: harness 合成 Mouse→单块 toggle；错点正文不变；字形宽度单测
- escalation_policy: 默认 EnableMouse 改变选区习惯须确认

## Further Notes

- 调研：[`research/fold-glyph-and-hittest.md`](./research/fold-glyph-and-hittest.md)
- `c1760` 已拍标记 `▶/▼`、不做 `(+)/(-)`——本草案可**微调**同一族三角，不引入加减号
