# Handoff: c82-fix-tui-core 实现

## 现状

c81 骨架已归档，旧 ratatui 路径（`render/` + `theme.rs` + `run_tui()`）已清理。
当前活跃变更：**c82-fix-tui-core** — 修复 TUI 完全不可用的三个阻塞性 Bug。

**变更路径**：`llmanspec/changes/c82-fix-tui-core/`
**实现前请阅读**：`proposal.md`、`design.md`、`tasks.md`

---

## 必须修复的三个 Bug

### Bug 1：事件循环饥饿（P0 · 最严重）

**症状**：键盘无响应，屏幕不刷新。整个 TUI 像死了一样。

**根因**：`src/interface/tui/mod.rs` 的 `run_tui_engine()` 事件循环中，
agent 流结束后 `agent_stream.next()` 永远立即返回 `Poll::Ready(None)`。
由于它是 `tokio::select!` 的**首个分支**，在多个分支同时就绪时优先选中它，
导致 `input_stream` 和 `frame_interval` 永远不被轮询。

代码中已预埋 FIXME 注释（第 67–76 行）：

```rust
// ══ FIXME(c82): agent_done flag to prevent select! starvation ══
```

**修复方案**：引入 `let mut agent_done = false`，流结束后替换为 `futures::future::pending().boxed()`：

```rust
let agent_fut: Pin<Box<dyn Future<Output = Option<AgentEvent>> + Send>> = if agent_done {
    futures::future::pending().boxed()
} else {
    agent_stream.next().boxed()
};

tokio::select! {
    event = agent_fut => { ... }
    ...
}
```

需要引入 `use std::pin::Pin;` 和 `use futures::Future;`。

---

### Bug 2：用户消息不显示（P0）

**症状**：用户输入文字并回车提交后，转录区看不到自己的消息。

**根因**：`src/interface/tui/state/transcript.rs` 中 `MessageStart { role: "user" }`
分支什么也不做（仅注释 `// Will be populated by TextDelta`），而 `TextDelta`
处理只在 `streaming_idx.is_some()` 或最后一个条目是 Assistant 时才追加。

**修复**：在 `"user"` 分支中创建 `TranscriptEntry::User { text: String::new() }`
并设置 `streaming_idx`：

```rust
"user" => {
    let idx = self.entries.len();
    self.entries.push(TranscriptEntry::User { text: String::new() });
    self.streaming_idx = Some(idx);
}
```

---

### Bug 3：无渐进滚动显示（P0）

**症状**：transcript 内容被简单截断到视口高度，新消息写在视口之外，用户看不到。

**根因**：`engine/mod.rs` 的 `compose_layout()` 中：

```rust
lines.truncate(transcript_h as usize);
```

没有偏移量，新内容超出视口即被丢弃。

**修复方案**（详见 `design.md`）：
1. `state/transcript.rs` 新增 `scroll_offset: usize` + 方法
2. `engine/transcript_renderer.rs` 从 `entries.len() - scroll_offset` 开始取行
3. `input/action.rs` 新增 `Action::ScrollUp` / `ScrollDown`
4. `input/keymap.rs` 新增 PgUp/PgDn、Up/Down 绑定

---

## 次要修复

### 状态栏模型名

`src/interface/cli/mod.rs` 当前调用 `run_tui_engine(&mut agent_loop, "", &session_id)`。
应获取模型名并传递，composer 状态栏硬编码的 `"xylitol TUI"` 需替换。

---

## 代码入口

| 文件 | 用途 |
|------|------|
| `src/interface/tui/mod.rs` | 事件循环（主修复点） |
| `src/interface/tui/state/transcript.rs` | JSON 状态机（用户消息 + 滚动） |
| `src/interface/tui/engine/mod.rs` | 布局组合 |
| `src/interface/tui/engine/transcript_renderer.rs` | 转录 ANSI 渲染 |
| `src/interface/tui/engine/composer_renderer.rs` | 输入区 ANSI 渲染 |
| `src/interface/tui/input/action.rs` | Action 枚举 |
| `src/interface/tui/input/keymap.rs` | 按键映射 |
| `src/interface/cli/mod.rs` | CLI 入口（传模型名） |

## 验证

```bash
# 编译
cargo build --features ui-tui

# 手动测试（需要 API key）
cargo run --features ui-tui -- tui

# 单元测试
cargo test --lib --features ui-tui -- tui
```

成功标准：
1. 键入字符后立即显示在 composer 中 ⬅️ 目前完全不显示
2. 回车提交后，用户消息出现在转录区，助手回复紧跟其后
3. 转录区可以 PgUp/PgDn 滚动浏览历史
4. Ctrl+C/D 正常退出
