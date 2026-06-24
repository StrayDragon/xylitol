---
depends_on: [c81-add-tui-skeleton]
---

# c82-fix-tui-core

## Why

c81-add-tui-skeleton 实现了 TUI 骨架的所有代码文件，但存在两个核心阻塞性 Bug 导致 TUI 完全不可用：

1. **事件循环饥饿**：agent 流结束后 `agent_stream.next()` 始终立即返回 `None`，而 `select!` 按分支顺序轮询且优先选中第一个就绪分支，导致输入事件和帧定时器**永远不被轮询**，屏幕无法刷新，键盘无响应。
2. **用户消息不显示**：`Transcript::apply(MessageStart { role: "user" })` 不创建任何 `TranscriptEntry`，导致用户提交的消息在转录中不可见。
3. **无渐进显示**：转录内容不跟随最新消息滚动，新内容被截断在视口之下，用户必须手动滚动才能看到。

本 change 修复以上 P0 问题，使 TUI 达到基本可用状态。

## What Changes

### 1. 修复事件循环饥饿

`run_tui_engine()` 中 agent 流结束后，`agent_stream.next().fuse()` 始终立即返回 `None`，在 `select!` 的首位分支中持续占先。

修复策略：引入 `agent_done: bool` 标志位，agent 流结束后不再 poll 已结束的流，只 poll 输入 + 帧定时器。

```rust
let mut agent_done = false;

loop {
    // pending_prompt 重启逻辑不变
    if let Some(prompt) = pending_prompt.take() {
        agent_stream = agent_loop.run(&prompt, session_id).await;
        agent_done = false;
    }

    let agent_fut = if agent_done {
        futures::future::pending::<Option<AgentEvent>>().boxed()
    } else {
        agent_stream.next().boxed()
    };

    tokio::select! {
        event = agent_fut => { ... }
        input_event = input_stream.next().fuse() => { ... }
        _ = frame_interval.tick() => { if dirty { render } }
    }
}
```

### 2. 修复用户消息转录

`state/transcript.rs` 的 `apply(MessageStart { role: "user" })` 分支改为创建 `TranscriptEntry::User` 并设置 `streaming_idx`，使后续 `TextDelta` 正确追加到该条目。

改动范围：`state/transcript.rs` ~3 行代码。

### 3. 实现转录视口滚动

`engine/transcript_renderer.rs` 中增加 `scroll_offset: usize`，保留最近 `transcript_lines` 条记录放入可见区域。在状态层新增 `Transcript::scroll_offset` 和 `scroll_up`/`scroll_down`/`reset_scroll` 方法。

改动范围：
- `state/transcript.rs` 新增滚动状态与方法
- `engine/transcript_renderer.rs` 滚动裁剪

### 4. 键盘映射：新增滚动快捷键

| 输入 | FocusCtx | 当前操作 | 变更后操作 |
|------|----------|----------|------------|
| `PgUp` | Transcript | `Action::None` | 向上滚动一页 |
| `PgDn` | Transcript | `Action::None` | 向下滚动一页 |
| `Up` | Transcript | `Action::None` | 向上滚动一行 |
| `Down` | Transcript | `Action::None` | 向下滚动一行 |

### 5. 状态栏：显示模型名称

`run_tui_engine()` 接收 `model_name` 参数，在 composer 状态栏中显示当前模型名，替代硬编码的 `"xylitol TUI"`。

## Capabilities

- `tui-interface`: TUI 交互模式（修复）

## Impact

- 修改 `src/interface/tui/mod.rs`（事件循环修复、状态栏传参）
- 修改 `src/interface/tui/state/transcript.rs`（用户消息 + 滚动）
- 修改 `src/interface/tui/engine/transcript_renderer.rs`（滚动裁剪）
- 修改 `src/interface/tui/engine/composer_renderer.rs`（状态栏显示模型名）
- 修改 `src/interface/tui/input/keymap.rs`（新增滚动按键绑定）
- 修改 `src/interface/tui/input/action.rs`（新增 ScrollUp/ScrollDown Action）
- 修改 `src/interface/cli/mod.rs`（传递模型名）
- 零影响 agent loop、session、config 等核心模块
