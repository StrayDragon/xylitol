---
depends_on:
  - c2020-add-package-tui-mouse-input
  - c2070-add-package-tui-dual-interaction-modes
blocks:
  - c2050-update-activity-fold-mouse-leader
apply_band: P9-deferred
summary: "点折叠三角 + L1 per-block 覆盖；延后至双交互架构 c2070 之后"
---

# 鼠标点击折叠三角 + 折叠标记字形

> **⚠️ deferred（2026-08-11）**：位于 `…/c2070-…/cascade/`（双架构顶层目录下）。与多级折叠族一并延后；**总前置** [`c2070`](../../proposal.md)。架构调研见 c2070 `research/`；本目录仅保留字形/hit 切片 [`research/fold-glyph-and-hittest.md`](./research/fold-glyph-and-hittest.md)。
>
> **c2020 提示**：点击折叠仍依赖已归档 **c2020** 的 mouse 管道（`InputEvent::Mouse` / `enable_mouse_capture`）。该管道**保留**；`XYLITOL_TUI_MOUSE` 仅为 lab/e2e。产品 inline **不开** capture——本 change 须在 **Mode B（c2070）** 落地后才有正式点折叠 UX。

> **一句话**：点折叠头行标记（倒三角/三角）toggle **单块**；键盘 `Alt+E` 仍为 **全局** tools；本 change **自带** per-block 覆盖表（不再依赖已废弃的 c2030 leader）。

## Why

今日 `Alt+E` 全局翻转所有 tool/diff（并连带 compaction）。定点折叠的最低学习成本是 **点 ▶/▼**（对齐 Web 同源，见 `docs/roadmaps/Web与TUI同源.md`）。

**产品分工（2026-08-11 拍板，弃 c2030 leader）**：

| 路径 | 作用 |
|---|---|
| **键盘** `Alt+E`（及既有 Ctrl+T / Ctrl+O） | **全局** 展开/折叠 |
| **鼠标** 点头行标记 | **单块** per-block 覆盖 toggle |

弃键盘 leader+数字的理由：β′ 屏外块一改行高，贴底 content-end 重绘会「跳回底部」；终端历史上滚无法廉价保留。鼠标点的是 **live 视口内可见头**——用户已在贴底画面上，重绘不额外制造「从历史上滚跳回」的断裂感（行高变化仍会重排上方内容，但交互锚点仍是眼前的标记）。

引擎侧：`c2020` 已提供 opt-in Mouse；产品尚无 hit-test / 覆盖表。

字形：今日 `GlyphSet::fold/unfold` = `▶`/`▼`（Ascii `>`/`v`）。可在不破坏宽度/旁注前提下微调。

## What Changes

1. **依赖 `c2020`**：产品在需要点击折叠时开 mouse capture；点击落在折叠标记（或整行头，propose 钉）→ toggle 对应 entry。
2. **Per-block 覆盖表**（本 change 落地，原拟 c2030）：`tools_expanded` 为默认；`overrides[target]` 优先；全局 `Alt+E` **改 default 并清空 overrides**（避免「按了全局没反应」）。Target：Tool/Ask 用稳定 `id`；Diff 用内容指纹或合成键。
3. **Hit-test 表**：render 维护 `fold_hit_regions`（行/列 → target）；优先标记列，避免点正文误触。复用/扩展 paint 时记录的头行命中（若有）。
4. **Paint**：单块 toggle 进 fingerprint，自该 entry truncate（ath25）；**禁止**全历史 MD 重解析。
5. **字形**：更新产品 `GlyphSet` 折叠标记；宽度 MUST 单列可视宽；Ascii 回退可读。
6. **非目标**：键盘 leader/数字编号；拖拽选区；滚轮改 app scroll；L2/L3 段点击（`c2050`）；产品自管 scroll / 锚点视口（高成本另案）。

## Capabilities（意向）

- `app-tui-transcript` — 覆盖表 + hit-test + toggle
- `app-tui-host` — Mouse 路由到 scrollback（Editor 未抢时）
- 产品 `GlyphSet` — 折叠标记

## Impact

| 层 | 影响 |
|---|---|
| 渲染 | hit 表与 paint-cache 同代；toggle 局部 miss |
| 输入 | Mouse Down/Up 去抖；忽略 move；Shift+click 跟 `c2020` |
| 键位 | **不改** `Alt+E` 全局语义（除非另钉拆 compaction） |
| 性能 | hit O(可见折叠头) |

## 依赖与排序

```text
c2020 ──depends→ [本 change c2040] ──blocks→ c2050
```

- **硬依赖**仅 `c2020`（Mouse）。
- **`blocks`**：`c2050`（段级点击语义）。

## Out of scope

- Fold-leader / 数字定点（**已废弃**原 `c2030`）
- Activity L2/L3 摘要文案（`c1760`）
- 改差分引擎算法；产品自管 transcript scroll

## Open Questions

1. 点击命中：仅标记单元格 vs 整条摘要/头行？
2. 折叠字形最终选：`▾`/`▸`、`▼`/`▶`、`▽`/`▷`，或其他？
3. mouse 默认开还是「首次需要点击折叠时再 Enable」？
4. 全局 `Alt+E` 是否仍连带 compaction，或本波顺手拆出独立键？（与弃 leader 正交，propose 可钉）

## 验证（自动化 + 人类）

| 层 | 自动化 | 人类 |
|---|---|---|
| Harness | 合成 `Mouse Down` 在标记列 → 单块 toggle；点正文 → 态不变；全局 Alt+E 清覆盖 | Kitty/foot：点标记收起/展开；误点 Markdown 不折 |
| Paint | 单块 toggle miss 上界（ath25） | 点眼前块时无明显「从历史上滚跳回」断裂（已在 live 底） |
| 字形 | `visible_width==1`；Ascii 回退 | `XYLITOL_TUI_GLYPH_SET=ascii` |

**人类最短路径**：开 mouse → 点可见折叠标记收起 → 再点展开 → `Alt+E` 全局翻转并清覆盖。

## Ethics

- risk_level: low–medium
- prohibited_actions: 点正文大面积误 toggle；无 `c2020` 透传策略就默认常开 capture；复活默认 Alt+digit leader
- required_evidence: harness Mouse→单块 toggle；错点正文不变；字形宽度；覆盖与全局 Alt+E 清表
- escalation_policy: 默认 EnableMouse 改变选区习惯须确认

## Further Notes

- 字形/hit 切片：[`research/fold-glyph-and-hittest.md`](./research/fold-glyph-and-hittest.md)
- **架构 / 选区 oneof / starline 对等**（深度调研）：[`../../research/`](../../research/)
- 差分适切性见 archive `c2020` `diff-engine-mouse-fit.md`
- `c1760` 已拍标记 `▶/▼`、不做 `(+)/(-)`——本草案可**微调**同一族三角
- **2026-08-11**：废弃 `c2030` fold-leader；定点改由本 change 鼠标路径独占；同日整组延后并挂 `c2070`
