---
change_id: c355-ensure-terminal-restore-on-panic
title: 确保 panic 时终端恢复（补齐 spec tui15 的 panic 安全缺口）
status: proposed
priority: 355
depends_on:
  - c340-add-tui-inline-repl
author: agent
---

# c355-ensure-terminal-restore-on-panic

> **范围调整（2026-07）**：本变更原计划包含「umbrella ratatui → ratatui-core 切换」+「scrolling-regions」+「panic hook」三件事。前两件已拆出到 **c341**（ratatui-core 切换是更大范围的重构，且弃用内置 widget 的决策需要独立论证）。本变更现在**只聚焦 panic 安全**——spec tui15 在 c340 落地时留下的唯一未勾选项。

## Why

c340 的 `InlineTerminal::Drop` 调用了 `ratatui::restore()` + `disable_raw_mode()`，注释声称「即使 panic 也不留坏终端」。但 c340 的 tasks.md 第 16 行明确把 panic hook 标记为 defer（`InlineTerminal::Drop 已保证 raw mode 恢复；显式 panic hook 作为后续小变更`）。**事实是：Drop 只在值正常离开作用域时运行，panic 发生在 Drop 之前的同一线程、且未 `catch_unwind` 时，调用栈上的局部变量虽然会 unwind 析构，但这依赖 panic = unwind（而非 abort）。**

具体缺口：
- `InlineTerminal` 是 `run()` 的局部变量（`mod.rs:64`）。如果 panic 发生在 `run()` 内部，unwind 路径上 `InlineTerminal::drop` 确实会跑。**但**：
  1. 若 `Cargo.toml` 的 `[profile]` 设了 `panic = "abort"`（或未来某个依赖/profile 改了），Drop 不运行，终端卡在 raw mode。
  2. panic 发生在 `spawn_blocking`/`tokio::spawn` 的子任务里（c340 的 `spawn_keyboard_reader`、`spawn_drain`），子任务 panic 不会触发主循环 `InlineTerminal` 的 Drop——它只在该子任务的栈上 unwind，而该栈上没有 terminal 句柄。
  3. 信号（SIGTERM/SIGINT 经 Ctrl+C，但 c340 已用 Ctrl+C 做 abort 而非退出）绕过 unwind。
- `codex` 的 `tui.rs` 正是用 `std::panic::set_hook` 兜底：在 hook 里 `disable_raw_mode` + `restore`，保证无论 unwind 还是 abort，终端都不被搞坏。这是工业级 inline TUI 的标准做法。

### 调研证据

- **codex**：`codex-rs/tui/src/lib.rs` 在进入 TUI 前安装 panic hook，hook 体内调用终端恢复（`restore_terminal` / `disable_raw_mode`），确保 abort 路径也恢复。
- **pi**：TS 项目，`process.on('exit')` + `process.on('SIGINT')` 兜底恢复 ANSI 状态；等价的 Rust 做法是 panic hook + signal handler。
- **Rust 语义**：`std::panic::set_hook` 在 abort 模式下**仍然执行**（hook 在栈 unwind/进程终止前跑），是唯一能跨 `panic=abort` 兜底的机制。
- **xylitol 现状**：`src/app/tui/terminal.rs:95-102` 的 Drop 只覆盖正常 unwind，无 panic hook，无信号处理。

## What Changes

1. 在 `src/app/tui/` 新增 panic 安全入口（可在 `terminal.rs` 内或新建 `init.rs`）：
   - `install_terminal_restore_hook()`：用 `std::panic::set_hook` 包装默认 hook，在 hook 头部执行 `disable_raw_mode()` + `ratatui::restore()`（best-effort，忽略错误），再链式调用原 hook 打印 panic 信息。
2. `run()`（`mod.rs`）在 `InlineTerminal::enter()` **之前**调用 `install_terminal_restore_hook()`。
3. 补充信号兜底（可选，若简单）：对 SIGTERM/SIGHUP 注册恢复（SIGINT 已被 Ctrl+C 占用做 abort，不动）。若 signal 处理跨平台复杂，作为 follow-up，本变更至少覆盖 panic hook。
4. 测试：无法对真 panic 写确定性单测（panic hook 是全局的），用 `#[cfg(test)]` 验证 hook 安装函数可调用且不 panic；真终端验收记录在 tasks（手动跑一次故意 panic 的 dev build 确认终端恢复）。

## Capabilities

- `app-tui`（修改）：补齐 spec tui15 的 panic 安全要求，明确 panic hook 是 MUST。

## Impact

- **受影响代码**：`src/app/tui/terminal.rs`（或新增 `src/app/tui/init.rs`）、`src/app/tui/mod.rs`（run 调 hook 安装）。
- **受影响规范**：`app-tui` 的 tui15（panic 安全）从「Drop 覆盖」升级为「panic hook MUST」。
- **风险**：低。panic hook 是纯追加，不改正常路径。唯一注意：hook 是全局的，若进程里其他代码也装 hook 需链式调用（xylitol 目前无其他 hook）。

## 反降级护栏（防止本变更被降级为「只加个空函数不接线」）

- [ ] `run()` MUST 在 `InlineTerminal::enter()` 之前实际调用 panic hook 安装函数（非仅定义）。
- [ ] panic hook MUST 在 hook 体首部执行 `disable_raw_mode()` + 终端 restore（best-effort），且 MUST 链式调用原 hook（不吞 panic 信息）。
- [ ] hook 安装 MUST 是幂等的（重复调用不叠加多个 hook——用 `Once` 或检查是否已装）。
- [ ] 真终端验收：构造一次故意 panic 的 dev build，确认终端不被搞坏（raw mode 关闭、光标可见）。
