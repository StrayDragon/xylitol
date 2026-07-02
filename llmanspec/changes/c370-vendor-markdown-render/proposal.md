---
change_id: c370-vendor-markdown-render
title: vendor ratatui-markdown 核心 + syntect 代码语法高亮
status: proposed
priority: 370
depends_on: []
author: agent
---

# c370-vendor-markdown-render

## Why

c355/c366 用手写 pulldown-cmark 状态机做了基础 markdown 渲染（标题/粗体/inline code/代码块/列表/引用/CJK 换行），但三个缺口在 c355/c366 范围外：

1. **代码块无语法高亮**：spec tui61（c355）明确排除 syntect，代码块只有语言标签 + 背景色，`fn main()` 和字符串没颜色区分。这是当前 TUI 可读性的最大短板——LLM 回复大量含代码，无高亮严重影响阅读。
2. **表格/task list 不支持**：手写状态机只覆盖 MVP 元素，`| a | b |` 表格、`- [ ]` task list 都被平铺。
3. **手写渲染器的维护负担**：c355/c366 的 Renderer 状态机约 350 行，每加一个 markdown 元素都要手写事件处理。专业库已覆盖全部 CommonMark + GFM。

调研三个本地仓库（codex / ratatui-markdown / xylitol）确认：ratatui-markdown 的核心渲染源码（markdown + highlight + theme + constants，17 文件 ~820 行）只用 `ratatui::style` + `ratatui::text`，这俩 `ratatui_core` 完整提供——类型壁垒零阻力，机械改 14 处 `use ratatui::` → `use ratatui_core::` 即可。mermaid/image 全部 `#[cfg(feature)]` 门控，不启用即自动消失，无需 stub。

高亮采用 codex 验证过的 syntect + two-face 路线（纯 Rust fancy regex，免 oniguruma C 编译），实现 ratatui-markdown 的 `CodeHighlighter` trait，核心约 130 行。

## What Changes

### 1. vendor ratatui-markdown 核心源码

落点 `src/app/tui/vendor/ratatui_markdown/`。从 `../ratatui-markdown/src/` 复制 17 个核心文件（markdown/parser+render+inline+text+types+hooks+image+mod、highlight/mod+config+hooks+segment+pest_bridge+treesitter、theme、constants/mod+list_prefix），排除 mermaid/scroll/tree/preview/viewer/text_input/box_chars。机械改 `use ratatui::` → `use ratatui_core::`（14 处），`crate::` 路径适配。

### 2. 新增 syntect 高亮后端

`src/app/tui/vendor/ratatui_markdown/highlight/syntect_bridge.rs`（~130 行）：实现 `CodeHighlighter` trait 的 `SyntectHighlighter`，固定 CatppuccinMocha 主题，参考 codex `render/highlight.rs` 的最小子集（`find_syntax` + `convert_style` + `convert_syntect_color` + `highlight` 循环 + 512KB/1万行安全限制）。依赖 syntect 5 + two-face 0.5（default-fancy，纯 Rust）。

### 3. 替换 c355/c366 手写 renderer

`components/markdown.rs` 的 `render_markdown` 改调 vendored 的 `MarkdownRenderer` + `Parser`。保留 `MarkdownStyle::for_user/for_assistant/for_thinking` 这层 seam，内部改成构造 vendored 的 `ThemeConfig`/`RichTextTheme`。删除手写 pulldown-cmark 状态机（~350 行）+ 删除 `pulldown-cmark` 依赖。

### 4. spec 适配

tui61 的「MUST NOT syntect」修订（c370 引入 syntect，约束过时）。新增 tui70（vendored renderer）、tui71（代码高亮 fallback）。保留 tui60/tui61/tui65/tui66 行为约束（测行为不绑实现）。

## Capabilities

- `app-tui`（修改）：tui61 修订（移除 syntect 禁令）；新增 tui70/tui71（vendored renderer + 高亮 fallback）。

## Impact

- **受影响代码**：
  - 新增 `src/app/tui/vendor/ratatui_markdown/`（17 vendored + 1 syntect_bridge + LICENSE + NOTICE，约 1000 行）
  - `src/app/tui/components/markdown.rs`：删除手写 Renderer 状态机，改调 vendored（净 -250 行）
  - `Cargo.toml`：加 syntect + two-face（tui feature），删 pulldown-cmark
- **受影响规范**：`app-tui`（tui61 修订 + tui70/tui71 新增）。
- **风险**：中。vendor 源码量大但机械；syntect 编译体积（+2-5MB 语法数据）；c355/c366 行为测试可能因 vendored 渲染差异变红（需逐个分辨）。阶段 0 spike 先验证可行性。

## 不在本变更范围

- mermaid 图表 / image 渲染（vendor 时排除）
- 自定义主题 / `.tmTheme` 加载（固定 CatppuccinMocha）
- 流式增量 markdown（仍只 finalize 后渲染）
- ratatui-markdown 的 scroll/tree/preview/viewer 高级 widget

## 调研证据

- **ratatui-markdown**：核心只用 `ratatui::style`/`text`（ratatui_core 全有）；mermaid/image 全 `#[cfg]` 门控；手写 parser 零 pulldown-cmark 依赖；`CodeHighlighter` trait 抽象高亮后端。
- **codex**：syntect 5 + two-face 0.5（`syntect-default-fancy` 纯 Rust）；`highlight.rs` 的 `highlight_to_line_spans_with_theme` + `convert_style` + `find_syntax` 是最小可用集；固定主题省 80% 代码。
- **xylitol**：c355/c366 行为测试是 TestBackend 断言（测内容非实现），换 renderer 应仍绿。
