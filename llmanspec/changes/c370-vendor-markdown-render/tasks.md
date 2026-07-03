# c370-vendor-markdown-render — Tasks

> 关键决策见 design.md：保留 MarkdownStyle 作适配层；高亮经 RenderHooks 注入；syntect 固定 CatppuccinMocha；纯 Rust fancy regex；StyleSegment 用全局 byte offset。

## 阶段 0：spike 验证

- [ ] 0.1 复制 ratatui-markdown 核心 4 文件（markdown/mod+render+parser、theme）到临时位置，改 `use ratatui::`→`ratatui_core::`，加 syntect+two-face 依赖，写一个 test 跑通「```rs 代码块 → 带高亮 Vec<Line>」。spike 通过才继续；失败回退（保留手写 + 仅追加表格/独立高亮）。

## 阶段 1：vendor ratatui-markdown 核心

- [ ] 1.1 建 `src/app/tui/vendor/ratatui_markdown/` 目录，从 `../ratatui-markdown/src/` 复制 17 核心文件（markdown/{mod,render,parser,inline,text,types,hooks,image}、highlight/{mod,config,hooks,segment,pest_bridge,treesitter}、theme、constants/{mod,list_prefix}）。排除 mermaid/scroll/tree/preview/viewer/text_input/box_chars。
- [ ] 1.2 机械改 14 处 `use ratatui::` → `use ratatui_core::`；`crate::` 路径适配（`crate::theme`→`super::theme` 等，或统一前缀）；constants/mod.rs 删 box_chars。
- [ ] 1.3 `vendor/mod.rs` + `tui/mod.rs` 注册 `pub(crate) mod ratatui_markdown;`。`cargo build --features tui` 通过。
- [ ] 1.4 放 `LICENSE`（SySL-1.0 原文）+ `NOTICE.md`（来源 + Copyright langyo + MODEL DISCLOSURE）。

## 阶段 2：syntect 高亮后端

- [ ] 2.1 `Cargo.toml`：tui feature 加 `syntect`（default-fancy）+ `two-face`（syntect-default-fancy），都 optional。
- [ ] 2.2 新增 `highlight/syntect_bridge.rs`（~130 行）：`SyntectHighlighter` impl `CodeHighlighter`，固定 CatppuccinMocha，含 `find_syntax`/`convert_style`/`convert_syntect_color`/`ansi_palette_color`/`MAX_HIGHLIGHT_BYTES/LINES`。参考 codex highlight.rs 最小子集。
- [ ] 2.3 `highlight/mod.rs` 注册 `pub use syntect_bridge::SyntectHighlighter;`。`cargo build --features tui` 通过。

## 阶段 3：替换手写 renderer

- [ ] 3.1 `components/markdown.rs`：`MarkdownStyle::for_user/for_assistant/for_thinking` 内部改构造 vendored `ThemeConfig` → `Box<dyn RichTextTheme>`。
- [ ] 3.2 `render_markdown` 内部改调 `MarkdownRenderer::new(width).with_render_hooks(HighlightHooks::new(SyntectHighlighter::new()))` + `Parser::new(width).parse(text)` + `.render(&blocks, &theme)`。
- [ ] 3.3 删除手写 Renderer/Block/emit_line/quote_depth/push_code_text 状态机（~350 行）。
- [ ] 3.4 `Cargo.toml` 删 `pulldown-cmark` 依赖（vendored 有自己的 parser）。
- [ ] 3.5 处理引用前缀：spike 观察 vendored blockquote 渲染；若无 `▎` 前缀，挂自定义 RenderHooks 补上（保留 c366 约定）。

## 阶段 4：测试 + 回归

- [ ] 4.1 新增：代码块高亮 TestBackend（```rs → Span 非 default style）；未识别语言 fallback（```unknown → 纯文本）；表格渲染（`| a | b |` → 对齐列）；超限 fallback（>512KB → 纯文本）。
- [ ] 4.2 回归 c355/c366：markdown 元素 9 测 + thinking/引用前缀 3 测 + transcript_line + commit_harness CJK。行为级断言应仍绿；红的分辨「vendored 差异」（修断言）vs「真退化」（修 vendor）。

## 阶段 5：校验 + 归档

- [ ] 5.1 `just qa`（all-features）：lib + bdd + clippy/fmt 零 warning + doc 无 error。
- [ ] 5.2 `arch_guard` 4 通过（vendor 目录属 app::tui，无跨层违规）。
- [ ] 5.3 `llman sdd validate c370 --strict` 通过。
- [ ] 5.4 归档 c370（tui70/tui71 合并；tui61 修订）。

## 反降级护栏自检

- [ ] vendored ratatui-markdown 实际被 `render_markdown` 调用（非仅复制文件）。
- [ ] 代码块实际经 syntect 高亮（```rs 产出带颜色 Span，spec tui71）。
- [ ] c355 tui60（user/assistant 共用 renderer）行为仍满足。
- [ ] c366 tui65/tui66（thinking 经 markdown + 引用前缀）行为仍满足。
- [ ] pulldown-cmark 依赖已移除（vendored 自带 parser）。
- [ ] LICENSE + NOTICE 在 vendor 目录（SySL 合规）。
