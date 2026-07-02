# c365 Design — 流式 mutable-last-line

> 涉及流式渲染架构调整 + ratatui inline 模式下的可变行实现，按 propose skill 要求写 design。

## 1. 核心难点：inline 模式下怎么实现"scrollback 最后一行可变"

ratatui 的 `Viewport::Inline` + `insert_before` 是**单向不可变**的——一行 insert_before 进 scrollback 就永久固定，不能改。codex 用自定义 terminal 绕过这个限制（`insert_history::*` 直接操作终端 buffer）。xylitol 用标准 ratatui，不能改 scrollback 里的行。

**解法**：mutable last line **不进 scrollback**，而在 tail 区（viewport 内）渲染。tail 区每帧重绘，旧内容自动被覆盖——这就是 codex 的 `sync_active_stream_tail` 机制本质。具体布局（流式时）：

```
[已 commit 的稳定行]      ← 终端原生 scrollback（insert_before 写入，永久）
─────────────────────  ← viewport 顶部（tail 区上边界）
[mutable last line]       ← pending_tail()，每帧重绘（tail 区第 1 行）
[thinking 指示器]         ← tail 区第 2 行（spinner + label）
[❯ 输入框]                ← tail 区最后一行（底锚）
─────────────────────  ← viewport 底部
```

流式时每帧：pending_tail 变长 → 第 1 行内容更新（视觉上"打字机在上行区生长"）。换行到达 → 该行 insert_before 进 scrollback（永久），pending_tail 重置为空，tail 区第 1 行变空等下一句。

**关键**：用户看到的"打字机在上行区"本质是 tail 区第 1 行（viewport 内最顶行），它紧贴 scrollback 最后一行，视觉上连成一体——就像文字在 scrollback 里生长。但它其实是 viewport 内每帧重绘的，不是真改 scrollback。

## 2. StreamBuffer 状态机（换行门控，对标 codex 极简版）

```rust
/// 换行门控的流式 buffer（对标 codex MarkdownStreamCollector 极简版）。
/// 累积 token；换行边界内的完整行可 drain 为稳定行；未换行尾部是 mutable tail。
pub struct StreamBuffer {
    buffer: String,
    committed_len: usize,  // 上次换行边界
}

impl StreamBuffer {
    pub fn push(&mut self, delta: &str) { self.buffer.push_str(delta); }

    /// 取出新完成的换行行（推进 committed_len）。无新换行则返回空。
    pub fn drain_complete_lines(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        while let Some(nl) = self.buffer[self.committed_len..].find('\n') {
            let abs = self.committed_len + nl;
            let line: String = self.buffer[self.committed_len..abs].to_string();
            self.committed_len = abs + 1;
            out.push(line);
        }
        out
    }

    /// 未换行的尾部（mutable last line 内容）。每帧重绘用。
    pub fn pending_tail(&self) -> &str { &self.buffer[self.committed_len..] }

    /// TurnEnd：把残留尾部作为最后一行取出，重置。
    pub fn finalize(&mut self) -> Option<String> {
        let tail = self.pending_tail().to_string();
        self.buffer.clear();
        self.committed_len = 0;
        if tail.is_empty() { None } else { Some(tail) }
    }
}
```

**对比 codex**：codex 的 `MarkdownStreamCollector`（`markdown_stream.rs:87-96`）逻辑完全一致（`rfind('\n')` 切边界 + `committed_source_len`），只是 codex 还做 markdown 增量解析。xylitol 纯文本，省掉解析。

## 3. TextDelta 处理改为增量 commit

当前（c360 后）`handle_xy_event` 的 TextDelta 分支：累积进 `pending`，换行时 `flush_complete_pending` 返回完整行（一次性 commit）。

c365 改为：用 `StreamBuffer`，每来 TextDelta 就 `drain_complete_lines()`，**产出的完整行立即返回**（mod.rs 立即 commit_to_scrollback）。pending_tail 不进 commit，留给 draw_tail_frame 每帧重绘。

```rust
XyEvent::TextDelta(text) => {
    self.stream_buf.push(text);
    let complete = self.stream_buf.drain_complete_lines();
    rendered.extend(complete.into_iter().map(RenderedLine::AssistantText));
}
```

mod.rs 的 `Msg::Xy` 分支已经对 `handle_xy_event` 返回的 lines 立即 `commit_to_scrollback`——所以**无需改 mod.rs**，只需 app.rs 把换行行实时返回。

## 4. draw_tail_frame 调整 → 组件化渲染（buffer 路线）

### 实施偏差纠正

c365 首次实施走了 "escape 直写终端 + DECSTBM scroll-region" 路线（`raw_render.rs`），绕过 ratatui buffer。该路线与多条 spec 冲突：tui41（TestBackend 可验证——escape 写的内容 TestBackend 看不到）、tui42（渲染层消费 RenderedLine 经 buffer）、tui21/tui5（escape 绑死物理坐标 + 自设滚动区，无法 widget 化、无法重定位）。且实测导致 commit 行与 mutable 行写同一物理行（`viewport_top - 1`）互相覆盖、TurnEnd `clear_mutable` 擦掉已 commit 内容——即用户观察到的 "正文 stream 后被清空"。**纠正回 design 原定的 ratatui-buffer 路线**：mutable 行在 tail 区顶行每帧重绘（buffer 内），背景透明融进 scrollback；所有 commit 走 `insert_before`。删 `raw_render.rs`。

### 渲染布局（buffer 路线）

流式时 tail 区（ratatui buffer，`Viewport::Inline` 内）从上到下：

```
[mutable last line（pending_tail，可能 wrap 多行）]  ← 顶行，透明 bg，融进 scrollback
[thinking 指示器]                                    ← input_bg
[❯ 输入框]                                           ← 底锚，input_bg
```

非流式时只有 `[❯ 输入框]`。mutable 行每帧重绘（buffer diff 自动只刷变化的 cell），换行时该完整行 `insert_before` 进 scrollback（永久），pending_tail 重置。**视觉上文字在正文区生长**：mutable 顶行紧贴 scrollback 最后一行、透明背景无缝衔接；换行只上移一行（非 c360 的多行块整体跳动）。

### wrap 策略

pending_tail 超宽时 **wrap 到多行**（`wrap_to_width`，CJK 按显示宽度），Tail 布局 bottom-anchored：`[MutableLine wrapped rows][Thinking][Input]` 从底往上排，capacity = tail height，超出 capacity 的 mutable **顶部行丢弃**（完整内容换行后进 scrollback，不丢失）。满足 "到宽度自动换行"，且不破坏生长视觉（wrap 出的多行中只有最后一行在生长，前面的是已稳定的 wrap 结果）。

### mutable line 锡定：top-anchored（二次纠正）

首版组件化把 `MutableLine` 实现为 **bottom-anchored**（贴 mutable_area 底部），真终端冒烟发现：单行 mutable 坐在 mutable_area 底行（`indicator_y - 1`），和 viewport 上方的 scrollback 之间空出多行——即用户观察到的 "正在打的字和 ❯ 你好 之间有空行"。**纠正为 top-anchored**：mutable 从 `mutable_area.y`（viewport 顶行）向下生长，单行时紧贴 scrollback 最后一行。超容量（wrap 行数 > mutable_area 高度）时仍**丢弃顶部**：`start = total - fit`，从 `area.y` 画保留的底部行——这样 mutable 始终锡定 scrollback（area.y 不空），同时最新字符（底部）可见。完整内容换行后进 scrollback，不丢失。

### inline viewport 固定高度限制 → route B（底部面板 border + bg）

`Viewport::Inline(N)` 是**固定 N 行的保留区**：流式时塞得满，turn 结束后 mutable 进 scrollback、thinking 消失，viewport 内只剩底部 input，上方 `N - 1` 行是**保留区空行**。route B 用一个带 border + bg 填充的 `BottomPanel` 包裹 status/thinking/input：idle 时面板填充整个 tail 区，空行变 "面板内部"（不再是空终端行）。

### 组件拆分（route B 后）

`ThinkingIndicator` 拆为两个独立 widget（spinner 与 reasoning 解耦）：

| widget | 职责 |
|---|---|
| `StatusIndicator` | spinner + `Working`/工具状态 label（执行进度） |
| `ThinkingBlock` | reasoning 显示（当前 `Thinking…` 占位，未来收 ThinkingDelta 可展开） |

新增 `BottomPanel`：带 border + panel_bg 的 chrome 容器，组合 `StatusIndicator` + `ThinkingBlock` + `InputPrompt`。`Tail` 改为组合 `MutableLine`（顶，透明，紧贴 scrollback）+ `BottomPanel`（底，bordered+bg）。`TAIL_HEIGHT` 调到 6（mutable 1 + 面板 5：顶 border + status + thinking + input + 底 border）。

### mutable line 锡定（top-anchored + 紧贴面板）

mutable 区域大小 = 实际 wrap 行数（capped 到面板上方可用空间），位置在面板顶 border 上方紧贴（available-above 区的底部）。`MutableLine` 在这个小区域内 top-anchored → 文字紧贴面板顶 border 往下生长，透明 bg 融进 scrollback。超容量从顶部丢弃（最新字符可见，完整内容换行后进 scrollback）。

### 组件化（widget 拆分）

渲染层拆成可复用、可单独 TestBackend 验证的 widget，每个是自包含的 "给我一块 area 我画好" 组件（tui21：自建 widget on ratatui-core primitives）。落在 `src/app/tui/components/`：

| widget | 职责 | 输入 | 渲染目标 |
|---|---|---|---|
| `TranscriptLine` | 把单个 `RenderedLine` 渲染进 Buffer（wrap + CJK + 样式） | `&RenderedLine`, width | insert_before buffer / TestBackend buffer |
| `MutableLine` | pending_tail 顶行，wrap 多行，透明 bg | `&str`, area | tail buffer 顶区 |
| `ThinkingIndicator` | spinner + label | spinner_idx, status | tail buffer 一行 |
| `InputPrompt` | `❯` + input buffer + 光标 + input_bg block | `&str`, area | tail buffer 底行 |
| `Tail` | 组合上述，bottom-anchored 布局，算 capacity/丢弃 | `&TuiApp`, area | 整个 tail buffer |

`draw_tail_frame` 退化为 `Tail::render(area, buf, app)`；`commit_to_scrollback` 改吃 `&[RenderedLine]`（不是 `&[Line]`），内部用 `TranscriptLine` widget 渲染进 insert_before buffer。每个 widget 有独立 TestBackend 测试（tui41 扩展）。

### 未来布局扩展位

用户未来目标底部操作区（带 border 的多行输入 + 状态行 + 统计行，类 pi）。当前组件为它留位但**不预先实现**（避免空壳死码，tui5）：

- `InputPrompt`（单行）→ 未来包 border + 多行编辑升级为 `InputArea`，是扩展非重写。
- `ThinkingIndicator` → 未来进状态行左槽。
- `Tail` 用 **bottom-anchored 布局**（从底往上排），未来底部加 `StatusBar`（统计+模型）只是再往上 push 一个 widget。
- `StatusBar` / border / 多行编辑 **本次不做**（无数据源即空壳）。

## 5. Tail 高度调整

当前 `TAIL_HEIGHT = 8`（terminal.rs:26）。c365 布局：mutable line(1) + thinking(1) + 输入(1) = 最少 3 行，加缓冲可设 4-5 行。流式时 pending_tail 单行不 wrap，8 行绰绰有余。**保持 8 不变**（留余量给未来多行场景）。

## 6. harness 覆盖（扩展 c360 的 TestBackend harness，组件化后每个 widget 独立可测）

StreamBuffer 状态机单测（`app.rs` 内）：
- `stream_push_drains_complete_lines_on_newline`：push "a\nb" → drain ["a"]，pending_tail "b"
- `stream_pending_tail_grows_without_commit`：push "abc"（无换行）→ drain 空，pending_tail "abc"
- `stream_finalize_flushes_residual`：push "a\nb" → finalize "b"

组件 TestBackend 测试（`components/` 各 widget 内或 `render.rs` harness）：
- `TranscriptLine`：ASCII / CJK / 长 wrap 多行，渲染进 buffer 断言（复用 c360 `row_text` 工具）
- `MutableLine`：pending_tail wrap 多行、透明 bg（`Color::Reset`）
- `ThinkingIndicator`：spinner glyph + label 出现在指定行
- `InputPrompt`：`❯` 前缀 + 输入内容 + input_bg block + 光标位置（CJK 显示宽度）
- `Tail` 组合：流式时顺序 [MutableLine wrapped][Thinking][Input] bottom-anchored；非流式时只有 [Input]；mutable 超容量顶部丢弃；TurnEnd 后 tail 无残留 reply 文本

**关键回归**：`streaming_text_not_in_ratatui_buffer_*`（c365 escape 时期的反向测试，断言 buffer 里 "没有" streaming 文字）**删除**，替换为正向测试（buffer 里 "有" mutable 文字）——escape 路线已弃。

## 7. 不做的事（防 scope creep）

- ❌ 不实现 codex table_holdback（纯文本无表格）
- ❌ 不实现 AdaptiveChunkingPolicy / commit_tick 动画线程（换行即固化，无动画需求）
- ❌ 不实现 consolidation（finalize 重排，inline 场景不需要）
- ❌ 不做 markdown 增量解析（纯文本）
- ❌ pending_tail 超宽时不 wrap（MVP 截断，记 design 待后续）

## 8. 风险

### R1：wrap 丢弃顶部 mutable 行【低】
pending_tail 超宽 wrap 多行后超出 tail capacity 的顶部行被丢弃，但**不丢失**——换行后完整内容进 scrollback 自然 wrap。用户只是暂时看不到行首。可接受。

### R2：mutable 顶行与 scrollback 最后一行视觉断裂【低】
两者紧邻但背景色可能不同（scrollback 终端默认色，tail 区有 input_bg）。mutable 顶行用透明 bg（`Color::Reset`）融进 scrollback；只有 thinking + input 行带 input_bg。c365 已明确此分工。

### R3：增量 commit 的 insert_before 频率【低】
每来一个换行就 insert_before 一次。ratatui insert_before 有 scrolling-regions 优化（tui22 已启用），单行 insert 开销极小。可接受。

### R4：组件边界划分不当【低】
5 个 widget 的职责边界若模糊会导致循环依赖或重复逻辑。缓解：每个 widget 只消费 UI 数据类型（`RenderedLine`/`&str`/spinner idx），不持有 app 引用；`Tail` 是唯一组合点，其余 widget 互不引用。TestBackend 独立测试强制每个 widget 自包含。
