# c355 Design — panic 时终端恢复（补齐 spec tui15）

c340 的 `InlineTerminal::Drop` 调用 `init::restore()` + `disable_raw_mode()`，声称「即使 panic 也不留坏终端」。但 Drop 有两个盲区，本变更用 panic hook 补齐。

## 1. Drop 的两个盲区（为什么需要 panic hook）

### 盲区 A：panic = abort 时 Drop 不运行
`InlineTerminal` 是 `run()` 的局部变量。panic 发生时：
- `panic = "unwind"`（Rust 默认）：栈展开，`InlineTerminal::drop` 执行 → raw mode 恢复 ✓
- `panic = "abort"`：进程直接终止，**Drop 不执行** → 终端卡在 raw mode ✗

虽然 xylitol 当前 Cargo.toml 没设 `panic = "abort"`，但：release profile 或未来某依赖可能改它；且这是「防御性正确」——不该假设 profile 配置。

### 盲区 B：子任务 panic 不触发主循环的 Drop
c340 的事件循环用了 `spawn_blocking`（键盘读）和 `tokio::spawn`（agent 事件 drain）。如果 panic 发生在这些子任务里，展开的是**子任务的栈**——那个栈上没有 `InlineTerminal`，所以 Drop 不执行 → 终端卡住 ✗

### panic hook 为何能覆盖两个盲区
`std::panic::set_hook` 注册的 hook 在 panic 触发时执行，**无论 unwind 还是 abort**（abort 模式下 hook 先跑再终止进程）。hook 是全局的，不依赖任何栈上的局部变量。所以在 hook 里 `disable_raw_mode()` 能覆盖所有 panic 路径。

**证据**：codex 的 `tui.rs` 正是这套（`set_panic_hook` → `restore()` → 链式调原 hook）。Rust 文档明确：「The default hook prints the message to standard error. [...] Hooks may be used to [...] abort the process.」（abort 前 hook 跑）。

## 2. 实现

### `init.rs` 加 `install_terminal_restore_hook()`

```rust
use std::sync::Once;
static HOOK_INSTALLED: Once = Once::new();

/// Install a panic hook that restores the terminal before the default hook
/// runs. Covers the two Drop blind spots (panic=abort; child-task panic).
/// Idempotent via Once — safe to call multiple times.
pub fn install_terminal_restore_hook() {
    HOOK_INSTALLED.call_once(|| {
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Restore terminal FIRST (best-effort), then chain to original
            // so the panic message still prints to a usable terminal.
            let _ = crossterm::terminal::disable_raw_mode();
            original(info);
        }));
    });
}
```

**设计要点**：
- `Once` 保证幂等——多次调用 `run()` 或测试里多次安装不会叠加多个 hook。
- 先 `disable_raw_mode` 再链式调原 hook——确保 panic 信息打印时终端已恢复正常（否则 raw mode 下 panic 信息可能错乱）。
- 用 `init::restore()` 还是 `disable_raw_mode()`？`init::restore()` 只调 `disable_raw_mode`（inline 无 alt screen），等价。直接调 `disable_raw_mode` 更明确（hook 里不要做复杂操作）。
- 不在 hook 里调 `ratatui_core::restore()`——那需要终端句柄，hook 要保持简单、无依赖。

### `run()` 在 `InlineTerminal::enter()` 之前调用

```rust
pub async fn run(driver: &mut dyn Driver) -> Result<(), String> {
    init::install_terminal_restore_hook();  // ← 新增，在 enter 之前
    let mut term = InlineTerminal::enter()...
```

**为何在 enter 之前**：如果 `enter()` 本身 panic（极少，但如 ratatui init 失败），hook 已就位。

### Drop 不动
`InlineTerminal::Drop` 保持现状（正常退出路径的恢复）。panic hook 是**追加的兜底**，不替代 Drop。

## 3. 测试策略

panic hook 是全局状态，无法对「真 panic」写确定性单测。分层测试：

- **可单测**：`install_terminal_restore_hook()` 可调用且不 panic（纯调用测试）；幂等性（多次调用不叠加——无法直接断言 hook 数，但验证不 panic）。
- **手动验收**（记录在 tasks）：构造一次故意 panic 的 dev build，确认终端恢复。

不引入 `panic = "abort"` 测试（改 profile 影响整个 test binary）。

## 4. 不做的事（防 scope creep）

- ❌ 信号处理（SIGTERM/SIGHUP）——跨平台复杂，且 SIGINT 已被 Ctrl+C 占用做 abort。作为独立变更。
- ❌ 改 Drop 逻辑——Drop 是正常路径，panic hook 是兜底，不合并。
- ❌ catch_unwind——不必要，panic hook + Drop 已覆盖。
