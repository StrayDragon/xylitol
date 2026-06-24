# c82-fix-tui-core — Design

## Context

- c81 实现了 P0 骨架，但存在三个阻塞性 Bug
- TUI 使用 `run_tui_engine()` 引擎路径（ANSI 差分渲染，非 ratatui）
- 事件循环为 `tokio::select!` 三路分支（agent / input / timer）

## 问题根因分析

### 1. 事件循环饥饿

**根因**：`agent_stream.next()` 在流结束后返回 `Poll::Ready(None)` 且永不阻塞。`tokio::select!` 按分支**声明顺序**检查就绪状态，`agent_stream` 作为首位分支始终立即就绪，导致 `input_stream` 和 `frame_interval` 分支被饿死。

**证据**：当 `select!` 在循环中反复执行时：
1. 轮询 `agent_stream.next()` → `Poll::Ready(None)` → 选中
2. 执行 body: `match None => { dirty = true; }`
3. 下一轮，`agent_stream.next()` 再次立即就绪
4. `frame_interval.tick()` 和 `input_stream.next()` 永不获得轮询机会

**修复**：引入 `agent_done: bool` 标志。当流结束后，用 `futures::future::pending()` 替代活的 agent future，使该分支在 `select!` 中永不就绪。

### 2. 用户消息不显示

**根因**：`state/transcript.rs` 中 `MessageStart { role: "user" }` 分支为：
```rust
"user" => {
    // Will be populated by TextDelta
}
```
但 `TextDelta` 处理逻辑仅在 `streaming_idx.is_some()` 或最后条目为 `Assistant` 时追加，用户消息无条目可追加。

**修复**：`"user"` 分支改为创建 `TranscriptEntry::User { text: String::new() }` 并设置 `streaming_idx`。

### 3. 无渐进显示

**根因**：`engine/compose_layout()` 中转录区域单纯截断到 `transcript_h`，没有滚动偏移量。新内容写在视口之外，用户看不到最新消息。

**修复**：Transcript 新增 `scroll_offset: usize` 字段，render 时从 `entries.len() - scroll_offset` 开始取行。

## 架构决策

### D1: agent_done 标志优于动态 select! 重构

两种方案对比：

| 方案 | 实现 | 复杂度 |
|------|------|--------|
| A: agent_done + pending() | 引入布尔标志，流结束后替换为 `pending()` | 低，~5 行改动 |
| B: 分两阶段循环 | stream_live / stream_dead 两个 loop | 中，代码冗余 |

选择方案 A：改动最小，对现有代码干扰最低。

### D2: Transcript 滚动状态

在 `Transcript` 中新增：
```rust
pub(crate) scroll_offset: usize,  // 0 = 最新，1 = 上移一行
```

方法：
- `scroll_up(n)`: offset += n，不超过 `entries.len().saturating_sub(1)`
- `scroll_down(n)`: offset = offset.saturating_sub(n)，最小 0
- `reset_scroll()`: offset = 0（新消息到达时自动重置）

### D3: 状态栏模型名

`run_tui_engine()` 新增 `model_name: &str` 参数，CLI 入口传递 `agent_loop.session.model_id()`。composer 渲染器接收后显示在状态栏。

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/interface/tui/mod.rs` | 修改 | 事件循环修复 + model_name 参数 |
| `src/interface/tui/state/transcript.rs` | 修改 | 用户消息创建 + scroll_offset |
| `src/interface/tui/engine/transcript_renderer.rs` | 修改 | 滚动裁剪 |
| `src/interface/tui/engine/composer_renderer.rs` | 修改 | 状态栏显示模型名 |
| `src/interface/tui/input/keymap.rs` | 修改 | PgUp/PgDn/Up/Down 绑定 |
| `src/interface/tui/input/action.rs` | 修改 | 新增 ScrollUp/ScrollDown Action |
| `src/interface/cli/mod.rs` | 修改 | 传递模型名 |

## 测试策略

- 单元测试覆盖 Transcript 状态机变更（用户消息、滚动）
- 单元测试覆盖 keymap 新绑定
- 手动烟雾测试验证：键入字符可显示、提交后消息可见、长对话可滚动
