# c355-tui-markdown-render — Tasks

> 关键设计决策见 design.md：新增 `RenderedLine::to_lines(width)`（保留 `to_line`）承载多行；MarkdownStyle 函数式 token 注入（user/assistant 各一份）；pulldown-cmark Event 栈式状态机产出 `Vec<Line>`；UserInput 的 `❯ ` 作首 Span 不进 markdown 解析；流式文字不渲染 markdown（spec tui61）。

## 阶段 1：依赖与基础组件

- [ ] 1.1 `Cargo.toml`：加 `pulldown-cmark`（默认 features 即可，仅需 CommonMark；放 `tui` feature gate，参考 `unicode-width` 的 `optional = true` 模式）。`cargo build --features tui` 通过。
- [ ] 1.2 新增 `src/app/tui/components/markdown.rs`：定义 `MarkdownStyle` 结构（text/heading/code_inline/code_block_bg/code_block_lang/list_marker/quote 样式 token）+ `for_user(&Palette)` / `for_assistant(&Palette)` 构造器。
- [ ] 1.3 `markdown.rs` 实现 `render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>>`：pulldown-cmark `Parser::new(text)` 事件遍历，栈式状态机，MVP 元素（spec tui61）：标题/粗体+inline code/代码块（语言标签+背景，无高亮）/无序列表/引用/CJK 段落换行（复用 `wrap_to_width`）。
- [ ] 1.4 `src/app/tui/components/mod.rs` 注册 `pub(crate) mod markdown;`。

## 阶段 2：RenderedLine 多行化

- [ ] 2.1 `render.rs`：`impl RenderedLine` 新增 `pub fn to_lines(&self, width: u16) -> Vec<Line<'static>>`——`UserInput`/`AssistantText` 经 `render_markdown`（UserInput 首行加 `❯ ` Span）；`ThinkingText`/`ToolSummary`/`Status` 退化为 `vec![self.to_line()]`。保留 `to_line`。
- [ ] 2.2 `transcript_line.rs`：`TranscriptLine::rows()` 从 `wrap_line_to_width(&to_line())` 改为 `self.rendered.to_lines(self.width)`（markdown 内部已 wrap，去掉二次 `wrap_line_to_width`）。
- [ ] 2.3 回归 `transcript_line` tests（ASCII/CJK/long-wrap/user-prefix/tool-summary）：行为级断言应仍绿。若有红，定位是 markdown 路径漏了 wrap 还是 style。

## 阶段 3：回归与校验

- [ ] 3.1 回归 tui41 CJK 测试：`commit_cjk_line_double_width_visible` / `commit_cjk_long_line_wraps_by_display_width` / `commit_long_line_wraps_to_width` 全过（markdown 路径下段落仍走 CJK wrap）。
- [ ] 3.2 新增 markdown 元素 TestBackend 测试（markdown.rs 内）：标题/代码块（语言标签+背景）/无序列表/引用/粗体/inline code 各一例，断言 buffer 内容（spec tui41 行为级）。
- [ ] 3.3 `cargo test --lib`：全部通过（含新增 markdown 测试 + 现有 479）。
- [ ] 3.4 `cargo test --test bdd`：85 通过（无 TUI BDD 回归）。
- [ ] 3.5 `cargo fmt --check` + `cargo clippy`（lib+bins，`--features tui`）：零 warning。
- [ ] 3.6 `cargo doc --no-deps --all-features`：生成成功。
- [ ] 3.7 `arch_guard`：4 通过（markdown.rs 属 `app::tui::components`，无跨层违规）。

## 反降级护栏自检（archive 前必过）

- [ ] UserInput 与 AssistantText 实际经 `render_markdown`（非仅新增模块，spec tui60 user-assistant-shared-renderer）。
- [ ] pulldown-cmark 在 Cargo.toml + markdown.rs 实际使用，syntect 缺席（spec tui61 no-syntect-in-mvp）。
- [ ] 流式 mutable 顶行文字仍为纯文本，不经 markdown（spec tui61 streaming-stays-plain）。
- [ ] 至少一个 TestBackend 测试验证代码块渲染出语言标签 + 背景色（spec tui61 code-block-structured）。
- [ ] tui41 CJK 回归测试全绿。
- [ ] `llman sdd validate c355-tui-markdown-render --strict --no-interactive` 通过。
