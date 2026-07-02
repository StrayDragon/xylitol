# c370 Design — vendor ratatui-markdown + syntect 高亮的接口设计

> 调研已穷尽（三仓库源码逐行确认），本文只记关键决策与 seam 设计。

## 核心张力

c355/c366 留下的 `MarkdownStyle`（`for_user`/`for_assistant`/`for_thinking`）是 `RenderedLine::to_lines` 消费的 seam——它返回 `Vec<ratatui_core::text::Line>`。vendored 的 ratatui-markdown 用 `RichTextTheme` trait + `ThemeConfig` 注入样式。两者要衔接，且不能让 `to_lines` 直接依赖 vendored 内部类型（保持渲染层 seam 清晰）。

## 设计决策

### 决策 1：保留 `MarkdownStyle` 作为适配层（而非暴露 vendored theme）

`components/markdown.rs` 的 `pub fn render_markdown(text, width, &MarkdownStyle) -> Vec<Line>` 签名**不变**（c355/c366 的调用点 `RenderedLine::to_lines` 不动）。内部实现从「手写 pulldown-cmark 状态机」换成「构造 vendored `ThemeConfig` → `MarkdownRenderer::render`」。

**选项 A（采纳）**：`MarkdownStyle` 内部存一个 `Box<dyn RichTextTheme>`，`for_user/for_assistant/for_thinking` 各 build 对应 `ThemeConfig`。
**选项 B（否决）**：把 `MarkdownStyle` 替换成直接暴露 `ThemeConfig`。否决：`RenderedLine::to_lines` 在 `render.rs`，让它 import vendored 的 `ThemeConfig` 会把 vendor 实现泄漏到 seam 外，且 user/assistant/thinking 三套样式的构造逻辑散落到调用方。

### 决策 2：高亮经 RenderHooks 注入（vendored 原生机制）

ratatui-markdown 的 `MarkdownRenderer::with_render_hooks(Box<dyn RenderHooks>)` 是高亮的接入点。`HighlightHooks`（`highlight/hooks.rs`）已实现 `RenderHooks`，其 `render_code_block` 调 `highlight_to_lines(&SyntectHighlighter, lang, code)`。

**xylitol 落地**：`render_markdown` 构造 renderer 时挂上 `HighlightHooks::new(SyntectHighlighter::new())`。代码块自动走高亮，无需在 `MarkdownStyle` 加高亮相关字段。

### 决策 3：引用前缀的兼容

c366 加了 `▎ ` 引用前缀。vendored renderer 的 blockquote 渲染（render.rs 的 `MarkdownBlock::Blockquote` 分支）自带样式但不一定带 `▎` 前缀。两种处理：

- 若 vendored blockquote 渲染已有可识别视觉结构（缩进/竖线），则 c366 的 `▎` 测试改为断言「有视觉结构」（弱化断言）。
- 若 vendored 无前缀，则在 `MarkdownStyle` 适配层挂一个自定义 `RenderHooks`，其 `blockquote` 方法补 `▎` 前缀。

**决策**：spike 阶段先观察 vendored blockquote 实际渲染，再定。优先保留 `▎` 前缀（c366 的视觉约定）。

### 决策 4：syntect 主题固定 CatppuccinMocha

codex 有 80+ 行主题自适应/自定义逻辑。xylitol MVP 固定深色主题 `CatppuccinMocha`（`two_face::theme::extra().get(EmbeddedThemeName::CatppuccinMocha)`）。主题切换留后续。

### 决策 5：依赖配置（纯 Rust fancy regex）

```toml
syntect = { version = "5", default-features = false, features = ["default-fancy"], optional = true }
two-face = { version = "0.5", default-features = false, features = ["syntect-default-fancy"], optional = true }
```
不用 oniguruma（codex 的 `syntect-default-onig` 需 C 编译），纯 Rust fancy regex 跨平台 CI 友好。代价：少数复杂正则语法支持不全，对代码高亮无感。

## StyleSegment 适配（syntect → ratatui-markdown）

codex 的 `highlight_to_line_spans_with_theme` 返回 `Vec<Vec<Span>>`（按行）。ratatui-markdown 的 `CodeHighlighter::highlight` 要返回 `Vec<StyleSegment>`（全局 byte offset，`segment.rs::segments_to_lines` 消费）。

适配关键（参考 `highlight/treesitter.rs` 的同构实现）：
- 不 trim 换行符（codex trim 了，但 segment.rs 用 `raw_line.len()` + 1 算偏移，需保留 `\n` 让 offset 连续）
- 空串/超限/未识别语言返回空 Vec（消费侧 fallback 到 default style，等价 codex 的 None fallback）

## 调用点影响

| 调用点 | 改动 |
|---|---|
| `RenderedLine::to_lines`（render.rs） | 不变（仍调 `render_markdown(text, width, &style)`） |
| `MarkdownStyle::for_user/for_assistant/for_thinking` | 内部改构造 `ThemeConfig`，签名不变 |
| `render_markdown` | 内部改调 vendored `MarkdownRenderer`，签名不变 |
| `TranscriptLine::rows` | 不变 |
| 手写 Renderer/Block/emit_line/quote_depth | 删除（~350 行） |

## LICENSE 归属（SySL-1.0）

ratatui-markdown 用 SySL-1.0（强制披露 AI 生成 + 保留 LICENSE 全文 + 保留署名）。vendor 落地：
- `vendor/ratatui_markdown/LICENSE`：原样 SySL-1.0 全文
- `vendor/ratatui_markdown/NOTICE.md`：来源（仓库 + Copyright langyo）+ MODEL DISCLOSURE（vendor 用 AI 模型）
- vendored 文件头保留来源注释

## 不在本变更范围

- mermaid/image（排除）
- 主题切换/自定义（固定 CatppuccinMocha）
- 流式 markdown（仍 finalize 后渲染）
