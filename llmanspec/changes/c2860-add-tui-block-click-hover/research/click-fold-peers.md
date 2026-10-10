# 可点折叠：xylitol / pi / opencode / crush

调研时点：2026-10-10。只服务本草案，不是 live spec。

## xylitol 已有

- `ExpandableOutput`：画视口 + footer 行号；`handle_input` 空。产品 transcript **不用**该组件，只用 `render_expandable_output`。
- 产品：`FoldHitTable` + host `install_fold_triangle_hit_priority` → `TUI::set_transcript_hit_priority`。Left Down 命中则 swallow、不启拖选。
- 合约：`app-tui-transcript` r1352 三角列-only；`package-tui-interaction-modes` r1632 无态 Moved 不得整帧重绘。
- 悬停：`designing/tui-lab/modules/hover-highlight/`；roadmap M4。

## 对照

| 产品 | 命中 | 与拖选 |
|---|---|---|
| pi 1.1.0 | 活组件树 `MouseRegion`；tool result / thinking **整块** click（release 且未移动才合成 click）。bash **无**鼠标折叠 | press 未 handled 才走选区 |
| opencode | OpenTUI box `onMouseUp`；`BlockTool` 整块、多数 tool 整行；hover raise bg | `getSelectedText()` 则 skip toggle |
| crush | 整项 left click；thinking 限 box 高 | DelayedClick 400ms；有选区不 fold |
| Codex TUI | 基本无 mouse-fold | — |

## ratatui

widget **不收鼠标**。折叠要点击 = app 对上一帧 Rect 命中（custom-widget 示例甚至用硬编码列桶，反面教材）。xylitol 应对齐「paint 登记几何、引擎分类手势」，几何扩宽是产品 `FoldHitTable` → `HitRegionList`，不是引入 `Buffer`。

产品 fan-in 无条件丢 `Moved`（`src/app/tui/mod.rs`），悬停必须先 c2858 探针 + 本票停丢。

## 不推荐

把产品 scrollback 改成 pi 式每块 Component：推翻 AO `ScrollView` 行缓存与 ath25「单块不重 parse Markdown」。不抄 ratatui immediate-mode 每事件整帧 Buffer。
