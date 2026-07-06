# c397 Design — 渲染粒度与 commit 粒度解耦

> 范围：第二梯队架构解耦。样式修复见 c396。
> 参考：`_HANDOFF.md` 第二节根因分析、第三节第二梯队；codex `streaming/controller.rs`。

## 问题回顾

当前流式管线（app.rs:71-119 `drain_complete_paragraphs`）：

```
TextDelta → stream_buf.push(delta)
         → drain_complete_paragraphs()       [fence 外逐行 / fence 内整块]
         → 每个 commit 单元 = 段落字符串
         → handle_xy_event 包装成 RenderedLine::AssistantText
         → RenderedLine::to_lines → render_markdown(段落)   [独立调用，看不到前文]
         → insert_before（不可逆）
```

**核心病灶**：commit 单元 = 渲染单元 = 段落。每个段落独立 `render_markdown`，渲染器无法判断前文，导致：
1. 代码块前后被迫无条件加空行（d877b63），因 renderer 不知前面有没有内容。
2. 列表/引用连续性：codex 靠 indent_stack 在整个列表/引用上重放前缀；xylitol 段落切分会打断。
3. `needs_newline` 跨段落无效，只能退化。

## 目标架构（codex 简化版）

```
TextDelta → stream_buf.push(delta)
         → raw_source.append(delta)                  [append-only]
         → full_rendered = render_markdown(raw_source, width, style)  [整条消息]
         → stable_boundary = last newline offset in raw_source
         → new_stable_lines = full_rendered[committed..stable_rendered_idx]
         → commit new_stable_lines via insert_before  [粒度=行]
         → committed = stable_rendered_idx
         → mutable region = full_rendered[committed..]  [每帧重绘]

TurnEnd → commit full_rendered[committed..] (含残余 unstable)，clear raw_source
```

**与 codex 的差异**（刻意简化）：
| 维度 | codex | xylitol (c397) |
|---|---|---|
| commit 动画 | tick → dequeue 节流 | 立即 commit（无动画） |
| finalize | ConsolidateAgentMessage 二次 canonicalize | 无（turn end 一次性 commit 残余） |
| resize 重渲 | stable 区可替换（AgentMarkdownCell） | 不支持（已 commit 不可逆，同现状） |
| table holdback | TableHoldbackScanner | 不做（段落切分已移除，表格整体渲染） |

xylitol 保留「commit 不可逆」现状，只升级**渲染粒度**为整条消息。这够用：流式观感立即改善，resize 自适应属第三梯队（#7）。

## 核心数据结构

### StreamBuffer（重构）

```rust
pub struct StreamBuffer {
    raw_source: String,
    /// 已 commit 进 scrollback 的渲染行数（在 full_rendered 里）。
    /// 注意：这是「渲染行」索引，随每次全量 render 重建而变化（结构重排）。
    committed_rendered_len: usize,
    /// 上次全量渲染的结果缓存（供 mutable 区取未 commit 行）。
    /// 每次 push 后整体重算。
    last_full_render: Vec<Line<'static>>,
}
```

**关键不变量**（借鉴 codex `controller.rs:1085-1200` 测试）：
> 流式逐 delta 的最终 committed 行序列 == 对完整 raw_source 一次性 render 的结果。

### 稳定边界计算

「稳定」= 最后一个换行边界之前的渲染行。pulldown-cmark 解析时，渲染行与源字节的对应需通过 `raw_source[..last_newline]` 单独 render 得到行数。简化：

```rust
fn stable_rendered_len(&self) -> usize {
    // 对 raw_source 截到最后一个换行，单独 render，取行数。
    // 这是「稳定」渲染行数——这些行不会因后续 delta 改变。
    let stable_src = match self.raw_source.rfind('\n') {
        Some(i) => &self.raw_source[..=i],
        None => return 0, // 无完整行，全部在 mutable
    };
    render_markdown(stable_src, width, style).len()
}
```

> **性能**：每次 push 做两次 render（stable_src + full raw_source）。对典型消息（几百行）成本可接受（pulldown-cmark 微秒级）。长消息（万行）可优化为增量，但不在本变更范围——先正确再优化。

### commit 流程（handle_xy_event）

```rust
XyEvent::TextDelta(text) => {
    self.stream_buf.push(text, width, style);
    let new_stable = self.stream_buf.drain_new_stable_lines();
    // new_stable: full_rendered[committed..stable_rendered_len]
    if !new_stable.is_empty() {
        rendered.extend(new_stable.into_iter().map(|line| RenderedLine::Rendered(line)));
        // committed_rendered_len 在 drain 内 advance
    }
}
```

**RenderedLine 新增变体** `Rendered(Vec<Line>)`：携带预渲染行，`to_lines` 直接返回（绕过 markdown.rs 的二次 render）。保留 `AssistantText(String)` 给 finalize 残余或非流式路径。

### mutable region（pending_tail 改签名）

```rust
// 旧：pending_tail() -> &str（未终止文本）
// 新：pending_rendered() -> &[Line<'static>]（未 commit 的渲染行）
pub fn pending_rendered(&self) -> &[Line<'static>] {
    &self.stream_buf.last_full_render[self.stream_buf.committed_rendered_len..]
}
```

`MutableLine` widget 改接收 `&[Line]` 而非 `&str`，多行动态高度。

## 动态 mutable region 高度（terminal.rs）

当前 `TAIL_HEIGHT: u16 = 6` 固定。问题：mutable 区只有 2 行，流式长段落换行时尾部被截。

**方案**：`TAIL_HEIGHT` 不再 const，改由当前 mutable 行数 + status(1) + panel(3) 决定：

```rust
fn desired_tail_height(mutable_rows: u16) -> u16 {
    // mutable 动态 + status 1 + panel 3（border+input+border）
    mutable_rows.min(viewport_height.saturating_sub(4)) + 4
}
```

**实现难点**：ratatui `Viewport::Inline(N)` 在 enter 时定 N，运行期改 N 需 `terminal.resize(...)` 或重新 `begin()/end()` 流式帧。codex 用 `CustomTerminal` fork 重写 viewport_area（HANDOFF 第三梯队 #8，成本最高）。

**xylitol 简化**：不改 `Viewport::Inline` 的 N，而是在 `draw_tail_frame` 里按 mutable 行数**动态布局** tail 内部（mutable 区 0..k，status k，panel k+1..k+3），k 上限 = N - 4。即 mutable 区最多 N-4 行，超出按现状「顶部丢弃」。这避免动 viewport，只用内部布局。

> 若实测 N=6 对长段落仍不够（mutable 最多 2 行），把 `TAIL_HEIGHT` 提到 8 或 10（给 mutable 更多预算），仍 const。真正的「无上限动态」属第三梯队 #8。

## 删除无条件空行 hack

架构解耦后（renderer 看整条消息），`markdown_render.rs:249-253` 改回有条件：

```rust
Tag::CodeBlock(kind) => {
    self.flush_line();
    if !self.lines.is_empty() {
        self.lines.push(Line::raw(""));  // 仅当前面有内容时才加空行
    }
    // ...
}
```

代码块**后**空行（markdown_render.rs:359）同理加守卫。`finish()` 的 is_empty 兜底保留。

## 测试策略

### 1. 流式 == 整体渲染 不变量（核心）

```rust
#[test]
fn streaming_matches_full_render() {
    let md = "# H1\n\npara with **bold**.\n\n- a\n- b\n\n> quote\n\n```rs\nfn x(){}\n```\n";
    // 流式：逐字 delta
    let mut stream = StreamBuffer::new(width, style);
    let mut committed_via_stream: Vec<Line> = Vec::new();
    for ch in md.chars() {
        let stable = stream.push_and_drain(&ch.to_string());
        committed_via_stream.extend(stable);
    }
    committed_via_stream.extend(stream.finalize_drain()); // TurnEnd 残余
    // 整体：一次性
    let full = render_markdown(md, width, style);
    assert_eq!(lines_text(&committed_via_stream), lines_text(&full));
}
```

### 2. 现有测试适配

- `stream_push_drains_complete_lines_on_newline` 等（app.rs:500-588）：drain 返回类型从 `Vec<String>` 改 `Vec<Line>`，断言改为渲染行文本。
- `fence_aware_commit_accumulates_code_block_until_close`：新模型下代码块天然整体渲染（整源 render），断言更新。
- mutable_line 测试：`pending_tail() -> &str` 改 `pending_rendered() -> &[Line]`。

### 3. 回归

- streaming/wrapping/CJK（render.rs cursor_tests, commit_harness）：确保多行 mutable 不破坏 CJK 对齐。
- turn-end-flush（tui31）：TurnEnd 必须把残余 unstable 行 commit，mutable 清空。
- multi-round tool turn（tui52）：text → tool → continuation text 的多次 push/finalize 不丢行。

## 不在范围

- stable 区可替换（resize 重渲，第三梯队 #7）
- CustomTerminal fork 动态 viewport_area（第三梯队 #8）
- commit-tick 动画（codex 风格逐行 dequeue 节流）
- TableHoldbackScanner（表格流式 holdback；整源 render 后表格天然完整）
- 嵌套列表缩进/悬挂缩进（架构解耦后可做，但留后续）
