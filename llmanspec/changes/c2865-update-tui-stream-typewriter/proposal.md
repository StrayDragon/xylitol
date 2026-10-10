---
depends_on: []
---

# 流式打字机稳定：未闭合围栏与尾部 wrap

## Why

产品已有 16ms 合帧、TextDelta 合并、assistant 稳定 Markdown 前缀增量 paint、引擎差分 `Vec<String>` ANSI。体感不稳主要来自 **半截闭合围栏 / 不稳定后缀整段重 parse+wrap**，不是屏幕 diff，也 **不是鼠标管线**（本票不依赖 c2858）。

pi 有 `trimPartialClosingFences`。opencode 把稳定块 commit 进终端 scrollback——与 AO 全量逻辑 transcript 不相容，但「只重画开着的最后一块」可在 D12 内用行切片近似。

ratatui / ratatui-markdown 每帧把 markdown 画进 `Line`/`Buffer`，没有「稳定前缀缓存」故事；**不要**为打字机改成 cell grid。CJK wrap 两边都靠 `unicode-width`；xylitol 已有 ANSI+ASCII 快路径（c1509），剩 grapheme 慢路径与围栏抖动。

delayed `c1535` 仍 park（CPU% ≠ 体感）。不重开 c1505 / c2525。

## What Changes

- 引擎 `Markdown`：流式修剪不完整闭合围栏（对齐 pi），避免代码块高度来回跳。
- 可选：thinking 尾只 wrap 最后视觉行；assistant 稳定前缀扩到围栏/列表边界。
- **不**引入 `StyledLine` / ratatui `Line`（D12）；**不**为 CPU% 降帧率。
- 尺子：半截 \`\`\` 到达前后可见高度不变。

## 非目标

- 不 promote c1505 / c2525。
- 不改 Codex 式终端 scrollback 历史。
- 不把 `UiModel` 流缓冲下沉进包。
- 不处理鼠标（hover 脏行是 c2858）。

## Capabilities

- `package-tui-markdown`
- `app-tui-host`（仅当 r1255 前缀边界扩展）

## Impact

- 包 Markdown 快照 + 半截围栏序列。
- 产品 `find_stable_markdown_prefix_end` 可吃引擎修剪结果。
- 与 c2890：若 mermaid 默认 `streaming`，半图高度要和本票一起测；推荐 mermaid `final` 则解耦。

## Open Questions

1. 修剪放引擎 `Markdown`（推荐）还是只放产品 `StreamingAssistantPaint`？
2. thinking 尾 wrap 是否本切片一起做？

## Further Notes

- [research/stream-typewriter-peers.md](./research/stream-typewriter-peers.md)
- park：delayed `c1535` / `c1505` / `c2525`
