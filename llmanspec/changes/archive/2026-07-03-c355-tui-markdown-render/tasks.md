# c355-tui-markdown-render — Tasks

> 关键设计决策见 design.md：新增 `RenderedLine::to_lines(width)`（保留 `to_line`）承载多行；MarkdownStyle 函数式 token 注入（user/assistant 各一份）；pulldown-cmark Event 栈式状态机产出 `Vec<Line>`；UserInput 的 `❯ ` 作首 Span 不进 markdown 解析；流式文字不渲染 markdown（spec tui61）。

## 阶段 1：依赖与基础组件

- [x] 1.1 `Cargo.toml`：加 `pulldown-cmark`（默认 features 即可，仅需 CommonMark；放 `tui` feature gate，参考 `unicode-width` 的 `optional = true` 模式）。`cargo build --features tui` 通过。
- [x] 1.2 新增 `src/app/tui/components/markdown.rs`：定义 `MarkdownStyle` 结构（text/heading/code_inline/code_block_bg/code_block_lang/list_marker/quote 样式 token）+ `for_user(&Palette)` / `for_assistant(&Palette)` 构造器。
- [x] 1.3 `markdown.rs` 实现 `render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>>`：pulldown-cmark `Parser::new(text)` 事件遍历，栈式状态机，MVP 元素（spec tui61）：标题/粗体+inline code/代码块（语言标签+背景，无高亮）/无序列表/引用/CJK 段落换行（复用 `wrap_to_width`）。
- [x] 1.4 `src/app/tui/components/mod.rs` 注册 `pub(crate) mod markdown;`。

## 阶段 2：RenderedLine 多行化

- [x] 2.1 `render.rs`：`impl RenderedLine` 新增 `pub fn to_lines(&self, width: u16) -> Vec<Line<'static>>`——`UserInput`/`AssistantText` 经 `render_markdown`（UserInput 首行加 `❯ ` Span）；`ThinkingText`/`ToolSummary`/`Status` 退化为 `vec![self.to_line()]`。保留 `to_line`。同时删除 `wrap_line_to_width`（c355 后无消费者，render.rs 内仅 `to_lines` 用 `wrap_to_width`）。
- [x] 2.2 `transcript_line.rs`：`TranscriptLine::rows()` 从 `wrap_line_to_width(&to_line())` 改为 `self.rendered.to_lines(self.width)`（markdown 内部已 wrap，去掉二次 `wrap_line_to_width`）；`render` 用 `Text::from(self.rows())`。
- [x] 2.3 回归 `transcript_line` tests（ASCII/CJK/long-wrap/user-prefix/tool-summary）：6/6 行为级断言全绿。

## 阶段 3：回归与校验

- [x] 3.1 回归 tui41 CJK 测试：`commit_cjk_line_double_width_visible` / `commit_cjk_long_line_wraps_by_display_width` / `commit_long_line_wraps_to_width` 全过（markdown 路径下段落仍走 CJK wrap）。
- [x] 3.2 新增 markdown 元素 TestBackend 测试（markdown.rs 内）：标题/代码块（语言标签+背景）/无序列表/引用/粗体/inline code/CJK 换行/user-assistant 共用 各一例，9 测全绿（spec tui41 行为级）。
- [x] 3.3 `cargo test --lib --features tui`：566 passed + 2 ignored（含新增 markdown 测试 + 现有 479 升至 566，tui feature 暴露更多模块测试）。
- [x] 3.4 `cargo test --test bdd --features tui`：85 通过（无 TUI BDD 回归）。
- [x] 3.5 `cargo fmt --check` + `cargo clippy`（lib，`--features tui`）：零 warning。
- [x] 3.6 `cargo doc --no-deps --features tui`：生成成功（无 c355 新增 doc warning，预存 warning 不在本变更范围）。
- [x] 3.7 `arch_guard`：4 通过（markdown.rs 属 `app::tui::components`，无跨层违规）。

## 反降级护栏自检（archive 前必过）

- [x] UserInput 与 AssistantText 实际经 `render_markdown`（非仅新增模块，spec tui60 user-assistant-shared-renderer；user_vs_assistant_share_renderer 测试 + to_lines 实现）。
- [x] pulldown-cmark 在 Cargo.toml + markdown.rs 实际使用，syntect 缺席（spec tui61 no-syntect-in-mvp）。
- [x] 流式 mutable 顶行文字仍为纯文本，不经 markdown（spec tui61 streaming-stays-plain；MutableLine widget 未改，render_markdown 仅经 to_lines 用于 finalized 行）。
- [x] 至少一个 TestBackend 测试验证代码块渲染出语言标签 + 背景色（spec tui61 code-block-structured；fenced_code_block_has_language_label + code_block_background_applied）。
- [x] tui41 CJK 回归测试全绿（commit_harness 3 测 + transcript_line 2 测）。
- [x] `llman sdd validate c355-tui-markdown-render --strict --no-interactive` 通过。
