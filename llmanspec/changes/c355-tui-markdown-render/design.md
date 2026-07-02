# c355 Design — MarkdownRenderer 接口与 RenderedLine 多行化

## 核心张力

当前 `RenderedLine::to_line(&self) -> Line<'static>` 返回**单行**，`TranscriptLine::rows()` 拿这单行再 `wrap_line_to_width` 物理换行。markdown 渲染后一个逻辑块（代码块、列表）天然是**多行**的，单行签名承载不了。

而 `commit_height`（计算 `insert_before` 区域高度）、`render_commit_lines_into_buf`（往 buffer 写）、`TranscriptLine::row_count` 都依赖 `to_line` 的单行返回。改 `to_line` 签名会牵动这三个调用点。

## 设计决策

### 决策 1：新增 `to_lines`，保留 `to_line`（而非替换）

```rust
impl RenderedLine {
    /// 单行快照（用于不渲染 markdown 的场景，如未来 log）。
    pub fn to_line(&self) -> Line<'static> { /* 现状不变 */ }

    /// 多行快照：UserInput/AssistantText 经 MarkdownRenderer；其余变体退化
    /// 为 `[to_line()]`（单元素 Vec）。宽度用于 markdown 内段落换行。
    pub fn to_lines(&self, width: u16) -> Vec<Line<'static>> { ... }
}
```

**选项 A（采纳）**：新增 `to_lines(width)`，保留 `to_line`。`TranscriptLine` / `commit_height` / `render_commit_lines_into_buf` 改用 `to_lines`。

**选项 B（否决）**：把 `to_line` 签名改成 `to_lines`，删除旧 `to_line`。
否决理由：`to_line` 有 4 个直接调用点（user_message_tests、commit_harness、transcript_line tests、cursor 间接），全删要逐个改；且 `ToolSummary` / `Status` / `ThinkingText` 这类单行变体本就不需要 markdown，保留 `to_line` 作为它们的单行出口更直白。`to_lines` 内部对非 markdown 变体就是 `vec![self.to_line()]`。

**代价**：两个方法并存，但语义清晰（`to_line` = 单行原样，`to_lines` = 渲染后多行）。代码上 `to_lines` 是主路径，`to_line` 退为内部 helper + 单行测试用。

### 决策 2：MarkdownStyle 结构（函数式 token 注入）

抄 pi 的 `MarkdownTheme` 函数式注入思路，但 Rust 里用结构体 + `Line<'static>` 产出（不走 ANSI 字符串拼接）：

```rust
pub struct MarkdownStyle {
    pub text: Style,              // 普通正文
    pub heading: Style,           // # 标题（粗体 + 主色）
    pub code_inline: Style,       // `code`（反色调）
    pub code_block_bg: Option<Color>, // ``` 代码块背景
    pub code_block_lang: Style,   // 代码块语言标签色
    pub list_marker: Style,       // - 列表项标记
    pub quote: Style,             // > 引用（dim + 斜体感）
}

impl MarkdownStyle {
    pub fn for_user(p: &Palette) -> Self { /* text = p.user_prompt(), ... */ }
    pub fn for_assistant(p: &Palette) -> Self { /* text = p.assistant(), ... */ }
}
```

`render_markdown(text, width, style)` 产出 `Vec<Line<'static>>`，每个 markdown 块对应若干 `Line`，inline span（粗体/code）合并进同一 `Line` 的 `Span` 序列。

### 决策 3：pulldown-cmark Event 遍历状态机

pulldown-cmark 是 pull parser：`Parser::new(text)` 产出 `Event` 迭代器（`Start(Tag)` / `End(Tag)` / `Text(CowStr)` / `Code(CowStr)` / `SoftBreak` 等）。遍历时维护一个小的栈式状态：

```rust
fn render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new(); // 当前 inline 行的 span
    let mut stack: Vec<BlockKind> = Vec::new(); // 段落/代码块/引用/列表 的嵌套上下文

    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Paragraph) => { stack.push(BlockKind::Paragraph); }
            Event::Start(Tag::Heading { level, .. }) => { stack.push(BlockKind::Heading(level)); }
            Event::Start(Tag::CodeBlock(kind)) => { stack.push(BlockKind::CodeBlock(extract_lang(kind))); }
            Event::Start(Tag::BlockQuote) => { stack.push(BlockKind::Quote); }
            Event::Start(Tag::List(None)) => { stack.push(BlockKind::UnorderedList); }
            Event::Start(Tag::Item) => { /* 新列表项：先 flush 当前行，push 列表项标记 */ }
            Event::Start(Tag::Strong) => { /* 标记后续 Text 用 bold style */ }
            Event::Start(Tag::Emphasis) => { /* 标记 italic */ }
            Event::Text(s) => {
                // 按 stack 顶决定 style，按宽度换行，push 进 current_spans 或 out
            }
            Event::Code(s) => { current_spans.push(Span::styled(s.into_string(), style.code_inline)); }
            Event::SoftBreak | Event::HardBreak => { flush_line(&mut out, &mut current_spans); }
            Event::End(_) => { /* pop stack，代码块/标题结束时 flush */ }
            _ => {}
        }
    }
    flush_line(&mut out, &mut current_spans);
    out
}
```

**关键点**：
- inline span（Strong/Emphasis/Code）合并进当前 `Line` 的 `Span` 序列，不产新行（一个段落是一行，跨 SoftBreak 才换行 + wrap）。
- block 元素（Heading/CodeBlock/Quote/Item）产新行，代码块整体加背景色 + 缩进 + 首行语言标签。
- 段落正文 `width` 换行：复用现有 `wrap_to_width`（CJK 宽度），把 `Text` 内容按 `width` 切片后逐片成 `Line`。
- 代码块**不做**宽度换行内的语法高亮（spec tui61），只做结构化（背景 + 缩进 + 语言标签）。

### 决策 4：UserInput 的 `❯ ` 前缀不进 markdown

现状 `RenderedLine::UserInput` 渲染为 `format!("❯ {prompt}")` 整体塞 `Line::styled`。若 prompt 含 `# 标题`，会被 markdown 解析成 heading。

**决策**：`❯ ` 作为独立首 `Span`（用 `user_prompt` style），prompt 正文经 `render_markdown(prompt, width, user_style)`。首行 = `❯ ` Span + prompt 第一行 Spans；后续行无前缀。这样 user 输入 `# foo` 仍渲染成 heading（与 assistant 一致的结构），但视觉上有 `❯ ` 标识来自用户。

### 决策 5：流式文字不渲染 markdown（spec tui61）

mutable 顶行的流式文字（`MutableLine` widget）保持纯文本（现状）。markdown 渲染只发生在 TextDelta 经 `insert_before` 提交到 scrollback 时（`commit_to_scrollback` 路径调 `to_lines`）。流式增量 markdown 留后续。

## 调用点影响清单（to_line → to_lines）

| 调用点 | 改动 |
|---|---|
| `TranscriptLine::rows()` | `wrap_line_to_width(&to_line())` → `to_lines(width)`（markdown 已含换行，不再二次 wrap；但代码块/长行可能超宽，需在 markdown 内部 wrap） |
| `TranscriptLine::row_count()` | `rows().len()` 不变 |
| `commit_height` | 内部 `TranscriptLine::new(l, width).row_count()` 不变 |
| `render_commit_lines_into_buf` | `TranscriptLine::new(line, width).render(area, buf)` 不变（widget 内部已用多行） |
| `user_message_tests` / `transcript_line tests` | 断言行为（含 `❯` / 文本可见），不绑实现，应仍绿 |

**关键**：tui41 的 CJK 回归测试（`commit_cjk_long_line_wraps_by_display_width` 等）是行为级断言（`row_text` 读 buffer 文本），不绑 `to_line` 还是 `to_lines`，只要文本可见 + 按显示宽度换行就过。markdown 路径下普通段落仍走 `wrap_to_width`（CJK 兼容），故应仍绿。

## 不在本变更范围

- syntect 语法高亮（代码块只结构化）——独立后续
- 表格渲染——独立后续
- 流式增量 markdown——独立后续
- 命令面板 / 审批浮层（c356/c357）——独立变更
