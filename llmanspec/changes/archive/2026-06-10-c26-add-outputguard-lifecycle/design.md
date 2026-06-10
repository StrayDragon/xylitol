# Design: OutputGuard + AgentSession 生命周期

## 1. OutputGuard

### pi 设计
`output-guard.ts` 劫持 `process.stdout.write`，将其重定向到 `process.stderr.write`。保留原始引用以便恢复。`write_raw_stdout` 使用保存的原始引用绕过劫持，直接写入 stdout。

### Rust 实现方案

Rust 没有 `process.stdout.write` 可替换 — 它是全局的。方案：

```rust
use std::sync::atomic::{AtomicBool, Ordering};

static STDOUT_TAKEN_OVER: AtomicBool = AtomicBool::new(false);
static ORIGINAL_STDOUT: std::sync::Mutex<Option<Box<dyn Fn(&str) + Send + Sync + 'static>>> = ...;

// 但在 Rust 中替换 stdout 更简单：
// print/write 宏默认写入 stdout。要劫持，需要：
// 1. 用 AtomicBool 标记 takeover 状态
// 2. 在 print 模式中，通过 writeln!(stderr, ...) 代替 println!
// 3. write_raw_stdout 通过 std::io::stdout().write() 绕过检查

struct OutputGuard {
    taken_over: bool,
}
```

**简化方案**（与 pi 语义对齐）：
- 不真的替换全局 stdout (Rust 限制)，而是提供一个 guard 标记
- `take_over_stdout()` → 设置标记为 true，返回 guard
- `restore_stdout()` → 设置标记为 false
- `write_raw_stdout(text)` → 直接 `std::io::stdout().write_all(text)`，不检查标记
- `is_stdout_taken_over()` → 返回标记
- 调用者（AgentSession::enter_print_mode）在写输出前检查此标记

## 2. AgentSession 生命周期

### pi 的 AgentSession 职责
- `start()`: 创建/恢复 Agent，订阅事件，设置回调
- `subscribe()`: 注册事件处理器 (模型响应, 工具调用, turn 开始/结束)
- `onNewAgentMessage()`: 收到新消息 → 持久化到 session
- `startNewSession(parent?)`: 创建新会话（可能 fork）
- `switchToSession(id)`: 切换到已有会话（加载 + 验证 CWD）

### 实现方案

```rust
impl AgentSession {
    // 新增字段
    event_bus: Option<AgentEventBus>,

    // 懒加载 event bus
    fn ensure_event_bus(&mut self) -> &mut AgentEventBus;

    // Turn lifecycle
    fn begin_turn(&mut self);
    fn end_turn(&mut self);

    // Session lifecycle
    async fn start_new_session(&mut self, name: &str, parent: Option<&str>) -> Result<()>;
    async fn resume_session(&mut self, id: &str) -> Result<()>;
}
```

## 3. 集成架构

```
AgentSession
├── OutputGuard (stdout takeover marker)
│   ├── enter_print_mode() — takeover stdout
│   └── leave_print_mode() — restore stdout
├── AgentEventBus (lazy init)
│   ├── begin_turn() → emit TurnStart
│   └── end_turn() → emit TurnEnd + persist message
├── SessionManager
│   └── resume_session() → load_validated() with CWD assert
└── MessageQueue (existing)
```
