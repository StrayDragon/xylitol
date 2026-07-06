# c396 Design — markdown 样式修复

> 范围：第一梯队纯样式修复。架构解耦（渲染粒度/viewport）见 c397。
> 参考：`_HANDOFF.md` 第一节症状对照表、第三节第一梯队。

## 核心决策

### D1. 标题分级采用 codex 的 6 级样式 + `# ` 前缀

codex `MarkdownStyles`（markdown_render.rs:89-105）的分级：
```
h1: bold + underlined
h2: bold
h3: bold + italic
h4/h5/h6: italic
```
+ `start_heading` 推入 `format!("{} ", "#".repeat(level))` 前缀（codex markdown_render.rs:579-580）。

xylitol 当前 `RenderStyle.heading` 是单一粗体，`Block::Heading(HeadingLevel)` 的 level 字段标注了 `#[allow(dead_code)]`（markdown_render.rs:120）——level 根本没被用到。

**改动**：
- `RenderStyle` 字段 `heading: Style` → `h1: Style, h2: Style, h3: Style, h4: Style, h5: Style, h6: Style`。
- `for_assistant/for_user/for_thinking` 各自填 6 级（assistant 用 codex 默认；user/thinking 在 base 上叠色）。
- `current_style()` 按 `Block::Heading(level)` 返回对应级。
- `start_block_or_inline(Tag::Heading { level, .. })` 在推入 block 前 push 一个前缀 span `"# " / "## " ...`，样式同级别。
- 删 `#[allow(dead_code)]`。

### D2. 引用块前缀：`> ` 而非 `▎`/`│`

spec tui66 原文举例 `▎`（左竖线），但允许「e.g. a left-bar」。各参考实现：
- codex：`│`（绿色，靠 indent_stack 重放）
- pi：`>`（保留源前缀）
- kimi-code：`▏`（dim）

**决策：采用 `> `**。理由：
1. 最接近 markdown 源语义，用户复制粘贴可还原 markdown。
2. 跨终端宽度稳定（`▎`/`▏` 在某些终端是半宽，对齐易错）。
3. codex 的 `│` 是 Unicode box-drawing，与 c395「不画 box-drawing 边框」精神冲突；`>` 是 ASCII 安全。
4. 嵌套引用 `>> ` 天然递推（每层加一个 `>`），无需额外缩进栈。

**改动**：
- `start_block_or_inline(Tag::BlockQuote)` 维护 `blockquote_depth: usize`（push 时 +1，pop 时 -1），每次 `flush_line` 前在 pending 行首推入 `"> ".repeat(depth)` span（quote 样式）。
- 测试 `blockquote_italic_no_pipe`（markdown_render.rs:496-505）断言反转：从「no pipe prefix」改为「has `> ` prefix」。`markdown.rs:blockquote_renders` 同理。

### D3. 有序列表：counter 状态机

当前 `Tag::List(Some(_))` 落入 `_ => {}`（markdown_render.rs:266），数字 marker 丢失。

**改动**：`Writer` 增 `list_counter: Option<u64>`（Some = 有序，记录当前序号；None = 无序）。
- `Tag::List(start)` → `list_counter = start.map(|s| s)`（pulldown-cmark 的 start 是 `u64`，通常 1）。
- `Tag::Item`：
  - `Some(n)` → push span `format!("{n}. ")`（light_blue），然后 `list_counter = Some(n+1)`。
  - `None` → push span `"• "`（现状）。
- `TagEnd::List` → `list_counter = None`。

> **不做嵌套**：codex 的 indent_stack + 悬挂缩进依赖渲染器看到整个列表（架构前提），xylitol 段落切分会打断。嵌套列表留 c397 解耦后再做（future.md 记录）。

### D4. 代码块留白：保留无条件空行

commit d877b63 的工作正确——架构未解耦前，段落独立 render 看不到前文，无条件前后空行是唯一能保证代码块不贴正文的手段。**本次不改这个行为**，只更新注释指向 c397。

`finish()` 的 `is_empty` 兜底保留。

### D5. 高亮主题自适应：分层降级

**问题**：写死 CatppuccinMocha 在亮色终端（白底）刺眼。

**分层探测**（首个命中采用，全部 miss 兜底暗色）：

| 层 | 信号 | 可靠性 | 实现 |
|---|---|---|---|
| 1 | `COLORFGBS` env（`"fg;bg"`，bg<7 暗） | 高（多数现代终端传播） | `std::env::var` |
| 2 | OSC 11 背景查询 | 中（需同步读响应） | crossterm 不直接支持，**默认放弃** |
| 3 | 兜底暗色 | — | 现状 |

**关键降级**：原计划用 OSC 11（发 `\e]11;?\e\\` 查询背景色），但 ratatui inline viewport 下同步读取 stdin 响应会**与用户输入竞态**（响应字节可能混入输入流，或查询期间用户按键被吞）。crossterm 无安全 API。因此**本变更只实现 COLORFGBS + 兜底**，OSC 11 探测记 future.md（需独立的异步查询窗口设计，超出样式修复范围）。

**实现**：
```rust
fn pick_theme() -> EmbeddedThemeName {
    if let Ok(fgbg) = std::env::var("COLORFGBS") {
        let parts: Vec<&str> = fgbg.split(';').collect();
        if parts.len() >= 2 {
            if let Ok(bg) = parts[1].parse::<u8>() {
                return if bg >= 7 { EmbeddedThemeName::CatppuccinLatte }
                       else { EmbeddedThemeName::CatppuccinMocha };
            }
        }
    }
    EmbeddedThemeName::CatppuccinMocha  // 兜底暗色
}
```
结果缓存在 `OnceLock`，进程内一次。

**用户体验**：无 `COLORFGBS` 的终端（如裸 Linux VT）仍暗色——与现状一致，无回退。设了 `COLORFGBS` 的（iTerm/Alacritty/Tmux/Kitty/GNOME Terminal 默认或可配）自动适配。

## 测试策略

沿用 `markdown_render.rs` 的 `render_to_buf` + `row_text` 测试框架（TestBackend）：

- `heading_h1_underlined` / `heading_h2_bold` / `heading_h3_bold_italic`：断言样式 modifier（通过 cell style 检查，非文本）。
- `heading_has_hash_prefix`：`# Title` → row 含 `"# Title"`。
- `blockquote_has_gt_prefix`：`> quote` → row 含 `"> "`（反转旧断言）。
- `nested_blockquote_double_prefix`：`>> nested` → row 含 `"> > "`。
- `ordered_list_renders_numbers`：`1. a\n2. b` → row0 含 `"1. "`，row1 含 `"2. "`。
- `ordered_list_custom_start`：`3. c` → row 含 `"3. "`。
- `highlight_theme_light_when_colorfgbs_light`：设 `COLORFGBS=0;15`，断言用了 Latte（可通过对比 span fg 与暗色主题不同来区分，或直接测 `pick_theme()` 单元）。

更新/删除的旧测试：
- `blockquote_italic_no_pipe`（markdown_render.rs:496）→ 改写为 `blockquote_has_gt_prefix`。
- `markdown.rs:blockquote_renders`（markdown.rs:177）→ 断言含 `> `。

## 不在范围

- 渲染粒度解耦（c397 StreamBuffer raw_source 累积）
- 动态 mutable region 高度（c397）
- 嵌套列表缩进/悬挂缩进（c397 解耦后）
- OSC 11 背景查询（future.md）
- 表格 Unicode 边框（c395 已决定空格对齐，不改）

## 归档说明（2026-07-06 补）

c396 的样式代码（commit 19b115e）已落地并通过单测，但截图暴露：在 ratatui inline-viewport 模型下，流式段落切分会破坏 markdown 结构（代码块围栏原样显示、无高亮、无空行），样式修复的完整效果无法体现。

决策：**归档 c396，样式资产转入 c399**（TUI 渲染层重写为 pi-tui line-array + differential render）。c399 的 Markdown widget 会复用本变更的样式逻辑（标题分级 / 引用 `>` 前缀 / 有序列表数字 / COLORFGBS 主题自适应），产出类型从 ratatui `Line` 改为自有 `StyledLine`（~50 行样式映射）。

c397（渲染粒度解耦）/ c398（resize 自适应）删除——两者需求在 c399 的 line-array 模型里天然满足（整源 render + width-change full redraw）。
