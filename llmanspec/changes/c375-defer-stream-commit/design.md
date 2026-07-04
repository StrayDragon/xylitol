# c375 Design — 延迟 commit 的数据流与 mutable 显示

## 核心张力

markdown 高亮需要完整段落（围栏上下文），但流式逐字到达。ratatui inline 下已 commit 行不可回溯修改。

## 设计决策：延迟到 TurnEnd（而非段落边界或 codex 式重渲）

### 选项 A（采纳）：TurnEnd 整段 commit

流式期间整段在 mutable 区累积（纯文本，逐字效果保留）；TurnEnd 时整段 `render_markdown` + commit。

**优点**：实现最简（只改 handle_xy_event 返回时机）；高亮完整；不触及 insert_before 不可变性。
**代价**：流式期间代码块单色（高亮在 TurnEnd 才出现）；长回复 mutable 区可能 drop 顶部（TAIL_HEIGHT=终端/2 缓解）。

### 选项 B（否决）：段落级 commit（空行边界）

按空行分段 commit。更细腻但：代码块通常不含空行（整块是一个段落），仍需整块累积到块后空行；跨段代码块（少见）需特殊处理。复杂度高，收益不明显。

### 选项 C（否决）：codex 式流式全程高亮

需 source-backed cell + 每 delta 全量重渲 + scrollback reflow + 自实现 terminal。架构改动过大。

## 数据流改造

### TextDelta（app.rs:198-210）

```rust
XyEvent::TextDelta(text) => {
    if self.thinking_phase {
        // thinking→text 转换：把已累积的 thinking_finalized 留着，
        // TurnEnd 时和 text 一起 commit（或在这里提前 commit thinking 段）
        self.thinking_phase = false;
    }
    self.finalized.push_str(text);  // 累积整段，不 commit
    // 不再 drain_complete_lines + 逐行返回
}
```

返回空 Vec（不触发 commit_to_scrollback）。

### ThinkingDelta（app.rs:191-197）

同理累积到 `thinking_finalized` 字段（新增）。TurnEnd 时整段 commit 为 ThinkingText。

### TurnEnd（app.rs:218-226）

```rust
XyEvent::TurnEnd { .. } => {
    if !self.thinking_finalized.is_empty() {
        rendered.push(RenderedLine::ThinkingText(std::mem::take(&mut self.thinking_finalized)));
    }
    if !self.finalized.is_empty() {
        rendered.push(RenderedLine::AssistantText(std::mem::take(&mut self.finalized)));
    }
}
```

### pending_tail（app.rs:260-277）

```rust
pub fn pending_tail(&self) -> Option<(&str, MutableKind)> {
    if self.thinking_phase {
        if !self.thinking_finalized.is_empty() {
            return Some((&self.thinking_finalized, K::Thinking));
        }
        return Some(("Thinking…", K::Thinking));
    }
    if !self.finalized.is_empty() {
        return Some((&self.finalized, K::Text));
    }
    // tool_status ...
}
```

mutable 区显示整段 `finalized`（MutableLine 多行 wrap + drop 顶部已就绪）。

### end_stream（app.rs:243-249）

防御性：如果 stream 异常结束（无 TurnEnd），残留 finalized 需 commit。但 end_stream 不返回 Vec（它不是 handle_xy_event）。**方案**：mod.rs 的 XyDone 处理在调 end_stream 前，先调一次 handle_xy_event 兜底——或让 end_stream 也返回 Vec。简单起见：XyDone 前如果有残留，构造一个最后的 commit。

## stream_buf 的去留

延迟 commit 后，`stream_buf`（StreamBuffer 的 drain_complete_lines 机制）不再需要。但 `pending_tail` 原读 `stream_buf.pending_tail()`——改成读 `finalized` 后，stream_buf 可移除。**决策**：移除 stream_buf，直接用 `finalized: String` 累积。thinking_buf 同理移除，用 `thinking_finalized: String`。

## TAIL_HEIGHT 动态化

```rust
pub fn enter() -> io::Result<Self> {
    let h = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
    let tail_height = (h / 2).max(6);
    // ... Viewport::Inline(tail_height)
}
```

## 多轮工具调用（中间 TurnEnd）

c370 下一个 user turn 可能有多个中间 TurnEnd。每个 TurnEnd commit 当前 `finalized` 段并 clear——每段独立高亮（ReAct 每轮输出独立成块，通常正确）。

## 不在本变更范围

- codex 式流式全程高亮
- 表格 holdback
- OSC8
