# Design: c2040 鼠标点击折叠三角

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| L1 三角列可点：Tool / Diff / Ask / Thinking | Bash / Compaction / Ctrl+O viewport → [`c2045`](../c2045-add-tui-fold-target-remaining/) |
| Thinking **per-id**（default + overrides） | L2/L3 段点击（c2050，可被 c2045 吸收） |
| tools 族 per-block overrides；`Alt+E`/`Ctrl+T` 清对应族 overrides | 拆 Alt+E×compaction；keyboard fold-leader |
| 字形 Unicode `▸`/`▾`；Ascii `>`/`v` | 整行可点；pi 式 release-无拖 click |
| 接 `set_transcript_hit_priority`；拖选 latch 忽略 fold | 改库选区状态机；产品自管 scroll |

## 为何不照搬 pi / zellij

- **pi TUI**：无点折控件；只借「引擎 swallow + 全局键盘批量」。
- **zellij**：无 fold；借 **控件 hit ≻ 选区起笔** + **拖选 latch**。
- **本仓**：`HitPriorityFn` 已为 Down 吞按预留 → 手势钉 **A+latch**。

一手：`research/pi-fold-hit-and-mouse.md`、`zellij-mouse-hit.md`、`synth-pi-zellij-xylitol-fold-hit.md`。

## 状态模型

```text
tools 族（Tool / Diff / Ask）
  default: tools_expanded: bool
  overrides: Map<ToolsTargetId, bool>   // 优先于 default

thinking 族
  default: thinking_expanded: bool
  overrides: Map<ThinkingId, bool>      // 优先于 default
```

| 动作 | 效果 |
|---|---|
| 点三角（该族某 id） | `overrides[id] = !effective(id)`（或删回 default——实现选更简者，语义=单块翻转） |
| `Alt+E` | flip `tools_expanded` default；**清空 tools overrides** |
| `Ctrl+T` | flip `thinking_expanded` default；**清空 thinking overrides** |
| 拖选中 | **不**跑 fold hit（latch） |

`effective(id) = overrides.get(id).copied().unwrap_or(default)`。

### ThinkingId

今日 `UiEntry::Thinking { text }` **无 id**。本 change MUST 为 Thinking 引入稳定 id（会话/轮次可复现；rebuild 与 live 同 id）。具体字段名属实现；合约只要求 **per-block** 可点且全局 `Ctrl+T` 清 thinking overrides。

## 命中与坐标

```text
Left Down (screen col,row)
  if selection_dragging → 只更新/结束选区
  else if fold_hit_regions 含 (col,row) 且为三角列
       → hit_priority true → toggle → 清 transcript 选区 → rerender
  else → 既有选区 / dock / Editor 路径
```

- hit 表：render 时只登记**可见**折叠头三角列（1 cell）；绑 paint generation。
- 产品经 `TUI::set_transcript_hit_priority` 接入；**不必**改引擎选区机。

## Paint / 性能

- 单块 toggle → fingerprint / cache 自该 entry truncate（ath25）；**禁止**全历史 MD 重解析。
- 字形 `visible_width == 1`；`XYLITOL_TUI_GLYPH_SET=ascii` 回退。

## 测试 seam（已与人确认）

1. 产品 harness / ScriptedDriver：合成 Mouse Down 三角列 → 单块 toggle；点正文不变
2. 同 harness：`Alt+E` / `Ctrl+T` 清对应族 overrides
3. ath25：单块 toggle miss 上界
4. 字形单测 width
5. 库 `application_owned_transcript_hit_priority_swallows_press` 不重做；产品接线测覆盖 latch

可执行 GWT：优先 **feature:false + harness**（对齐 ath29/ath30）；apply 阶段补绑定若升 `.feature`。

## 依赖顺序

```text
c2020 ✓ + c2070 ✓ + c2071 ✓（产品 AO）
  → c2040 Specs → 覆盖表+ThinkingId → hit 表接线 → 字形 → harness
  → c2045（剩余 FoldTarget）∥ c1760 → c2050（或被 c2045 吸收）
```
