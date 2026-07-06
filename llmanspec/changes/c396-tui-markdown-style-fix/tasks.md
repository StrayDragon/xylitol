# c396-tui-markdown-style-fix — Tasks

> 顺序执行；每步后跑列出的校验。架构解耦（渲染粒度/viewport）在 c397，本变更不碰。

## 1. 标题分级渲染

- [ ] `markdown_render.rs`：`RenderStyle` 字段 `heading: Style` → `h1..h6: Style`（6 个），删 `Block::Heading` 的 `#[allow(dead_code)]`。
- [ ] `for_assistant/for_user/for_thinking` 各填 6 级样式（assistant: H1 bold+underline / H2 bold / H3 bold+italic / H4-6 italic；user/thinking 叠对应 base 色）。
- [ ] `current_style()`：`Some(Block::Heading(level))` → 按级别返回 `h1..h6`。
- [ ] `start_block_or_inline(Tag::Heading { level, .. })`：推入前缀 span `format!("{} ", "#".repeat(level as usize))`，样式同级别。
- [ ] 校验：`cargo test -p xylitol --lib markdown_render`（新增 `heading_h1_underlined` / `heading_h3_bold_italic` / `heading_has_hash_prefix` 通过）。

## 2. 引用块 `> ` 前缀

- [ ] `Writer` 增 `blockquote_depth: usize`；`start_block_or_inline(Tag::BlockQuote)` 时 `flush_line` 后 `blockquote_depth += 1`；`end_block_or_inline(TagEnd::BlockQuote)` 时 `flush_line` + `blockquote_depth -= 1`。
- [ ] 在每次向 pending 推文本前（`flush_line` 重置后），若 `blockquote_depth > 0`，先 push 前缀 span `"> ".repeat(depth)`（quote 样式）。
- [ ] 改写测试 `blockquote_italic_no_pipe`（markdown_render.rs:496）→ `blockquote_has_gt_prefix`：断言 row 含 `"> "`。
- [ ] 新增 `nested_blockquote_double_prefix`：`>> nested` → row 含 `"> > "`。
- [ ] 更新 `markdown.rs:blockquote_renders`（markdown.rs:177）与 `nested_blockquote_renders`（markdown.rs:257）断言含 `> `。
- [ ] 校验：`cargo test -p xylitol --lib markdown` + `cargo test -p xylitol --lib markdown_render`。

## 3. 有序列表数字 marker

- [ ] `Writer` 增 `list_counter: Option<u64>`（Some=有序当前号，None=无序）。
- [ ] `start_block_or_inline`：`Tag::List(start)` → `list_counter = start`（pulldown-cmark 给 `Option<u64>`）；`TagEnd::List` → `list_counter = None`。
- [ ] `Tag::Item`：`Some(n)` → push `format!("{n}. ")` span（`Style::default().fg(Color::LightBlue)`），`list_counter = Some(n+1)`；`None` → push `"• "`（现状）。
- [ ] 新增测试 `ordered_list_renders_numbers` / `ordered_list_custom_start`。
- [ ] 校验：`cargo test -p xylitol --lib markdown_render`。

## 4. 代码块留白注释更新（不改行为）

- [ ] `markdown_render.rs:249-253` 注释更新：标注「streaming 段落切分的临时缓解，根治见 c397 渲染粒度解耦」。
- [ ] 不改逻辑（commit d877b63 的无条件空行保留）。

## 5. 高亮主题自适应

- [ ] `syntect_highlight.rs`：`theme()` 改名 `pick_theme()` 返回 `EmbeddedThemeName`，按 `COLORFGBS` 探测（bg>=7 → Latte，否则 Mocha；unset → Mocha）。
- [ ] 结果缓存在 `OnceLock<EmbeddedThemeName>`，`highlight()` 取缓存。
- [ ] 新增单元测试 `pick_theme_light_when_colorfgbs_light` / `pick_theme_dark_when_colorfgbs_unset`（env 操作需串行，`--test-threads=1` 或 `serial_test`；优先用独立函数纯测逻辑避免 env 抖动）。
- [ ] 校验：`cargo test -p xylitol --lib syntect_highlight`。

## 6. 全量校验

- [ ] `just fmt`（rustfmt）。
- [ ] `just lint`（clippy 无新告警；删了 `#[allow(dead_code)]` 后确认无残留）。
- [ ] `just test`（全绿；含更新的 markdown/markdown_render 测试）。
- [ ] `cargo run -- --help`（无回归）。
- [ ] `just qa`（fmt+clippy+test+docs+prek 全过）。
- [ ] 手动：在带 `COLORFGBS` 的终端跑 TUI，长文档（含标题/引用/有序列表/代码块）观察观感改善。

## 7. SDD 归档准备

- [ ] `_HANDOFF.md` 第三节第一梯队各项标记完成（或在本变更 archive 时删除 HANDOFF 对应段落）。
- [ ] `llman sdd validate c396-tui-markdown-style-fix --strict` 通过。
- [ ] archive 后启动 c397（depends_on: [c396]）。
