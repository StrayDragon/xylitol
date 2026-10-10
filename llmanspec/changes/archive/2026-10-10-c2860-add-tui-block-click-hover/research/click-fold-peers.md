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

## 长内容回收（相对「只放宽 header」）

header 全宽只降低 **展开** 成本。展开后块头滚出视口，L1 再折要滚回去——这是 accordion 通病。

xylitol 已把「输出很长」交给 Ctrl+O 视口：`expandable_output` 展开后在 **块尾** 画 `ctrl+o to fold`（peo3 / att30），读完自然落在折回控件上。Activity 另有从输入侧 `collapseNearest`。这两条都不该被「整块可点」替代。

整块 click（opencode/pi/crush）用空选守卫换回收便利，和 DESIGN「复制友好优先」抢划词。钉住块头要 Viewport 钉行，偏固定区而不是对话条目，留给以后（滚动槽一类）而不是本票。

L1 首轮设想曾对齐 att30「头上展开、尾上折回」。锁定后改为空选点 painted 正文也可折（见 proposal Open Questions 1）；att30 hint 仍只折视口。

## 不推荐

把产品 scrollback 改成 pi 式每块 Component：推翻 AO `ScrollView` 行缓存与 ath25「单块不重 parse Markdown」。不抄 ratatui immediate-mode 每事件整帧 Buffer。
