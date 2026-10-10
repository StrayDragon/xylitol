---
depends_on: []
---

# 流式打字机稳定：未闭合围栏

## Why

产品已有 16ms 合帧、TextDelta 合并、assistant 稳定 Markdown 前缀增量 paint、引擎差分 `Vec<String>` ANSI。体感不稳主要来自 **半截闭合围栏 / 不稳定后缀整段重 parse+wrap**，不是屏幕 diff，也 **不是鼠标管线**（本票不依赖 c2858）。

pi 有 `trimPartialClosingFences`。opencode 把稳定块 commit 进终端 scrollback——与 AO 全量逻辑 transcript 不相容，但「只重画开着的最后一块」可在 D12 内用行切片近似。

ratatui / ratatui-markdown 每帧把 markdown 画进 `Line`/`Buffer`，没有「稳定前缀缓存」故事；**不要**为打字机改成 cell grid。CJK wrap 两边都靠 `unicode-width`；xylitol 已有 ANSI+ASCII 快路径（c1509），剩 grapheme 慢路径与围栏抖动。

delayed `c1535` 仍 park（CPU% ≠ 体感）。不重开 c1505 / c2525。

## What Changes

- 引擎 `Markdown::render`：parse **前**修剪未闭合围栏末行上的不完整 closer（对齐 pi `trimPartialClosingFences` / pulldown 只能动源字符串）。
- 产品流式助手继续走该组件，不另写第二套修剪。
- **不**引入 `StyledLine` / ratatui `Line`（D12）；**不**为 CPU% 降帧率。
- 尺子：半截 \`\`\` 到达前后，代码块可见行数不变。

## 非目标

- 不 promote c1505 / c2525。
- 不改 Codex 式终端 scrollback 历史。
- 不把 `UiModel` 流缓冲下沉进包。
- 不处理鼠标（hover 脏行是 c2858）。
- 不改 thinking 尾 wrap（plain wrap，另开切片）。
- 不扩 r1255 稳定前缀到列表/表边界。

## Capabilities

- `package-tui-markdown`（新 r1936：半截闭合围栏）
- `app-tui-host`（新 r1937：流式助手高度不跳；r1255 不改句）

## Impact

- 包 Markdown 快照 + 半截围栏序列。
- 产品 `find_stable_markdown_prefix_end` 可吃引擎修剪结果。
- 与 c2890：若 mermaid 默认 `streaming`，半图高度要和本票一起测；推荐 mermaid `final` 则解耦。

## Open Questions

1. ~~修剪放引擎 `Markdown` 还是只放产品 `StreamingAssistantPaint`？~~ **已定引擎**（2026-10-10）：parse 前剥源字符串。pulldown 不能改 token；产品前缀缓存救不了围栏内那一行。
2. ~~thinking 尾 wrap 是否本切片一起做？~~ **已定否**：thinking 是 plain wrap，另开切片。
3. ~~扩稳定前缀到列表/表边界？~~ **已定否**：r1255 不改句。

## Further Notes

- [research/stream-typewriter-peers.md](./research/stream-typewriter-peers.md)
- park：delayed `c1535` / `c1505` / `c2525`
