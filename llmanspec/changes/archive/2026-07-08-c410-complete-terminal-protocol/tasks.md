# c410 Tasks — terminal 协议补齐

> 每块 ≤2h。引用 spec req：tp01-tp04。

## 阶段 1：parse_kitty_flags 纯函数 + 单测（1h，第 1 层）

- [x] 1.1 在 `terminal.rs` 加 `parse_kitty_flags(seq: &str) -> Option<u32>`：匹配 `CSI ?Nu`（`\x1b[?Nu`）→ N；非匹配 → None。对齐 pi `parseKeyboardProtocolNegotiationSequence`。
- [x] 1.2 单测：`parse_nonzero_flags`（`\x1b[?7u` → 7）、`parse_zero_flags`（`\x1b[?0u` → 0）、`parse_nonmatch`（DA 响应 `\x1b[?62;4;52c` → None）、`parse_garbage`（`abc` → None）
- [x] 1.3 `cargo test -p xylitol-tui --lib terminal` 通过

## 阶段 2：modifyOtherKeys + OSC 标题/进度（1h）

- [x] 2.1 `CrosstermTerminal` 加 `enable_modify_other_keys()`（写 `CSI >4;2m`）/ `disable_modify_other_keys()`（写 `CSI >4;0m`），带状态字段 `modify_other_keys_active: bool`
- [x] 2.2 Terminal trait + CrosstermTerminal 加 `set_title(title: &str)`（写 `OSC 0;title BEL`）
- [x] 2.3 Terminal trait + CrosstermTerminal 加 `set_progress(active: bool)`（写 `OSC 9;4;3` / `OSC 9;4;0`，无 keepalive）
- [x] 2.4 Terminal trait 加 `move_by(lines: i32)`（写 `CSI nB` / `CSI nA`）
- [x] 2.5 VirtualTerminal（test harness）为这些新方法加 no-op 实现
- [x] 2.6 `cargo test -p xylitol-tui` 通过（不破坏现有测试）

## 阶段 3：Kitty 协议探测（start 时，1.5h）

> design 决策 2：运行时走 crossterm `push_enhancement_flags`，不自己解析 CSI 响应。

- [x] 3.1 确认 crossterm 0.29 的 `PushKeyboardEnhancementFlags` API（`crossterm::terminal::window_size`? 查 docs——实际是 `crossterm::event::PushKeyboardEnhancementFlags` + `KeyboardEnhancementFlags`）
- [x] 3.2 `CrosstermTerminal` 加 `start()` 方法：enable_raw_mode + 开 bracketed paste（`\x1b[?2004h`）+ 发 Kitty 查询序列（`CSI >7u CSI ?u CSI c`）+ 调 crossterm push_enhancement_flags(ALL) → 成功则 `set_kitty_protocol_active(true)`，失败则 `enable_modify_other_keys()`
- [x] 3.3 `CrosstermTerminal` 加 `stop()` 方法：disable bracketed paste + 若 kitty pushed 则 pop（`CSI <u` + crossterm pop_enhancement_flags）+ disable_modify_other_keys + disable_raw_mode + show_cursor
- [x] 3.4 让 `tui.rs::start_impl` 调用 `terminal.start()` / `terminal.stop()`（替代当前内联的 enable_raw_mode / disable_raw_mode / bracketed paste 写入）
- [x] 3.5 VirtualTerminal 的 start/stop 为 no-op（test harness 不碰真终端）
- [x] 3.6 `cargo test -p xylitol-tui` 通过

## 阶段 4：drainInput + 测试（1h）

- [x] 4.1 `CrosstermTerminal` 加 `drain_input(max_ms: u64, idle_ms: u64)`：kitty pop 后 poll+read 掉残留事件（防慢 SSH 泄漏），对齐 pi `drainInput`
- [x] 4.2 `stop()` 内部先调 `drain_input(200, 30)` 再做清理
- [x] 4.3 单测：drain_input 的纯逻辑（用 MockClock 控制 timeout——但 drain 本身依赖 event::poll，难单测；改为验证 stop() 写出的清理序列正确，drain 的 poll 行为靠 E2E）

## 阶段 5：E2E（第 5a 层，1h）

- [x] 5.1 在 `tests/tui_e2e/pty.rs` 加测试 `kitty_query_emitted_at_start`：spawn demo，读启动字节流，断言含 `CSI >7u`（`\\x1b[>7u`，对应 scenario tp01）
- [x] 5.2 加测试 `title_set_via_osc_when_started`：spawn demo，读字节流，断言含 `OSC 0;` + BEL（如果 demo 设标题）或确认 set_title 可调用
- [x] 5.3 标 `#[ignore]`，本地 `cargo test --test tui_e2e -- --ignored terminal` 实跑验证

## 阶段 6：lib.rs 导出 + 文档（0.5h）

- [x] 6.1 `lib.rs` 导出 `parse_kitty_flags`（公开诊断 API）
- [x] 6.2 更新 `_HANDOFF.md` 已知差距表：terminal.rs 从「85 行偏薄」改为「✅ kitty 探测 + modifyOtherKeys + OSC 完成」
- [x] 6.3 更新 `packages/xylitol-tui/AGENTS.md`（如需要）

## 阶段 7：全量校验（0.5h）

- [x] 7.1 `cargo test -p xylitol-tui` 全量通过
- [x] 7.2 `cargo clippy -p xylitol-tui --all-targets -- -D warnings` clean
- [x] 7.3 `just test-tui-e2e`（kitty/title 相关 E2E 实跑）
- [x] 7.4 `llman sdd validate c410-complete-terminal-protocol --strict` 通过
- [x] 7.5 手动 `cargo run --example demo -p xylitol-tui` 启动不崩

## 校验命令

```bash
cargo test -p xylitol-tui                              # 第 1 层单测
cargo clippy -p xylitol-tui --all-targets -- -D warnings
cargo test --test tui_e2e -- --ignored terminal        # 第 5a 层 E2E
llman sdd validate c410-complete-terminal-protocol --strict
```
