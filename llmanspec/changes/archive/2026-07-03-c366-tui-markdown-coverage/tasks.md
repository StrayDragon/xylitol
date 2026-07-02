# c366-tui-markdown-coverage — Tasks

> 范围：收口 c355 的 markdown 管线——ThinkingText 接入 + 引用块视觉前缀。无新依赖，纯渲染层。

## 阶段 1：ThinkingText 接入 markdown

- [x] 1.1 `markdown.rs`：新增 `MarkdownStyle::for_thinking(&Palette)` 构造器——以 `p.thinking()`（dim italic）为 text 基色，heading/code_inline/code_block_bg/list_marker/quote 偏 dim（保持「思考是次要内容」的视觉层级，但保留 markdown 结构）。
- [x] 1.2 `render.rs`：`RenderedLine::to_lines` 的 `ThinkingText` 分支从 `vec![self.to_line()]` 改为 `render_markdown(text, width, &MarkdownStyle::for_thinking(&p))`。
- [x] 1.3 新增 TestBackend 测试：ThinkingText 含代码块 → 渲染出语言标签 + 结构（dim 样式，spec tui65 thinking-renders-markdown）。

## 阶段 2：引用块视觉前缀

- [x] 2.1 `markdown.rs`：`Renderer` 加 `quote_depth: usize` 字段，`Start(Tag::BlockQuote)` 时递增、`End(TagEnd::BlockQuote)` 时递减。
- [x] 2.2 `Renderer` 的 BlockQuote 行产出：每行首加 `▎ `.repeat(quote_depth) 前缀 span（`style.quote` 着色），含 wrap 续行。统一经 `emit_line` helper 注入前缀（flush_inline / push_code_text / Rule 三处）。
- [x] 2.3 新增 TestBackend 测试：引用 → 每行有 `▎` 前缀；嵌套引用 → 双前缀（spec tui66 blockquote-has-prefix / nested-blockquote-indents）。

## 阶段 3：回归与校验

- [x] 3.1 回归 c355 现有测试（markdown 元件 + transcript_line + commit_harness CJK）全绿。
- [x] 3.2 `just qa`（all-features）通过：lib 568 + bdd 85、clippy/fmt 零 warning、doc 无 error。
- [x] 3.3 `arch_guard` 4 通过。
- [x] 3.4 `llman sdd validate c366-tui-markdown-coverage --strict --no-interactive` 通过。

## 反降级护栏自检

- [x] ThinkingText 实际经 `render_markdown`（非 to_line，spec tui65 thinking-source-shares-renderer）。
- [x] 引用块每物理行含 `▎` 前缀（spec tui66；可视化验证：`▎ 这是一段引用` / 嵌套 `▎ ▎ 嵌套引用`）。
- [x] 流式 thinking mutable 顶行仍是纯文本（spec tui61 streaming-stays-plain 不回退；MutableLine widget 未改）。
