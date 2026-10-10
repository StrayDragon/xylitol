---
depends_on: []
---

# Markdown 围栏内 mermaid 终端 ASCII 图

## Why

pi `grok-mermaid` 与 opencode merman 在 TTY 画流程图。xylitol 把 `lang=mermaid` 当普通代码高亮。D11 裁掉 Kitty Image——本票只做 Unicode 盒线，不碰协议位图。

**不依赖** ratatui-markdown：输出是 `Vec<Line>`（撞 D12），许可证 **SySL-1.0**（AI 披露 copyleft，不能进 MIT 仓）。可对照其「BFS 分层 + 盒线网格 + 失败回源」思路，**禁止**依赖或整文件搬 pest grammar。

图内平移（merman 可水平拖）需要 c2858 `HitKind::Pan`，**本票不做 pan**。渲染与鼠标管线解耦；半截围栏高度与 c2865 相关——默认 `final` 可解耦。

## What Changes

- `Markdown`：`lang == mermaid` 先于 `highlight_code` 短路；能画则 `Vec<String>` ANSI 盒线，失败留源。
- 超宽拒画、留源（pi 同策略），守 width invariant。
- 设置意向：`off | final | streaming`，默认 **`final`**（流式当代码，收束再画）。thinking 默认不画。
- `MarkdownTheme` 可加 `render_fence(lang) -> Option<Vec<String>>` 测注入。
- 拷贝默认粘贴艺术字；不承诺「拷贝仍是 mermaid 源」。

## 非目标

- 不复活 D11 / mermaid.js / WebView。
- **不** `ratatui-markdown` 依赖或源码搬迁。
- 不保证全语法；flowchart TD/LR + sequence 子集，其余失败回源。
- **不做** 图内 pan/zoom（将来另票，`depends_on: [c2858]`）。

## Capabilities

- `package-tui-markdown`

## Impact

- 与 c2865：`streaming` 会叠加半图抖动；默认 `final` 则本票可并行。
- `agent_demo` 可放一块 fence，不是产品 SSOT。

## Open Questions

1. 默认 `final`（推荐）vs pi `streaming`？
2. 子集实现：自研网格 vs 重写 grok-mermaid 思路（均不依赖 OpenTUI / ratatui-markdown）？

## Further Notes

- [research/mermaid-peers.md](./research/mermaid-peers.md)
