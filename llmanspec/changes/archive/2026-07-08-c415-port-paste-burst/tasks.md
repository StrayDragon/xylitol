# c415 Tasks — paste-burst 移植

> 纯状态机移植，~70 行 + ~10 测试。c405 第 1+3 层验证。

## 阶段 1：PasteBurst struct（1h）

- [x] 1.1 新建 `packages/xylitol-tui/src/paste_burst.rs`：移植 pi `PasteBurst`，方法接受 `Instant`（`on_plain_char`/`should_insert_newline_instead_of_submit`/`extend_window`/`reset`）
- [x] 1.2 4 个常量：`MIN_CHARS=8` / `CHAR_INTERVAL=8ms` / `ACTIVE_IDLE_TIMEOUT=30ms` / `ENTER_SUPPRESS_WINDOW=120ms`
- [x] 1.3 `lib.rs` 导出 `PasteBurst` + 常量
- [x] 1.4 `cargo check -p xylitol-tui` 通过

## 阶段 2：测试（第 1+3 层，1h）

- [x] 2.1 新建 `packages/xylitol-tui/tests/paste_burst_test.rs`
- [x] 2.2 pb01 测试：`burst_detected_after_8_fast_chars`（8 字符 1ms 间隔 → submit 返回 true）
- [x] 2.3 pb01 测试：`no_burst_slow_typing`（8 字符 20ms 间隔 → false）
- [x] 2.4 pb01 测试：`fewer_than_8_chars_no_burst`（5 快速字符 → false）
- [x] 2.5 pb02 测试：`time_boundary_7ms_inside` + `time_boundary_9ms_outside`（窗口边界）
- [x] 2.6 pb03 测试：`enter_submits_after_suppress_window`（burst 后 121ms → false）
- [x] 2.7 pb03 测试：`reset_clears_state`
- [x] 2.8 对齐 pi `editor.test.ts:42-60` 的 6 个 PasteBurst 测试逐一核对

## 阶段 3：验证（0.5h）

- [x] 3.1 `cargo test -p xylitol-tui` 全量通过
- [x] 3.2 `cargo clippy -p xylitol-tui --all-targets -- -D warnings` clean
- [x] 3.3 `llman sdd validate c415-port-paste-burst --strict` 通过
- [x] 3.4 更新 `_HANDOFF.md` 已知差距表（paste-burst 标完成）

## 校验命令

```bash
cargo test -p xylitol-tui
cargo clippy -p xylitol-tui --all-targets -- -D warnings
llman sdd validate c415-port-paste-burst --strict
```
