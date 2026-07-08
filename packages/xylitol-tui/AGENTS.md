# packages/xylitol-tui

`pi-tui` 的 Rust 完整移植（differential rendering 引擎 + 组件库）。目标是替换 `src/app/tui/engine/`，所以**必须与 `kimi-code/packages/pi-tui` 对齐**，不夹带 xylitol 应用层耦合。

## 测试约定（c405 五层架构）

本 package 承载五层测试架构的第 1-4 层（in-process）；第 5 层在 workspace 顶层 `tests/tui_e2e/`。

| 层 | 落点 | 用于 |
|---|---|---|
| 1. 按键序列→状态 | `tests/support/mod.rs::TuiTestHarness` + `tests/harness_test.rs` | editor/input/select_list 的交互测试（抄 helix model 断言）|
| 2. insta snapshot | `tests/snapshot_test.rs` + `tests/snapshots/` | 整屏渲染回归（`viewport_snapshot` → `assert_snapshot!`）|
| 3. 时序 | `src/clock.rs`（`Clock`/`MockClock`）+ `#[tokio::test(start_paused)]` | paste-burst/debounce 确定性测试 |
| 4. proptest | `tests/property_test.rs` | editor 状态机崩溃边界（随机按键 + 不变量）|

### 新增测试时的规则

- **新组件交互测试**走第 1 层：`TuiTestHarness::new().mount(...).keys("...").render().assert_text_contains("...")`。
- **新组件渲染回归**走第 2 层：加 `tests/snapshot_test.rs` 用例，`INSTA_UPDATE=always cargo test --test snapshot_test` 接受后人工复核。
- **时序逻辑**（paste-burst、autocomplete debounce）走第 3 层：同步用 `MockClock`，async 用 `#[tokio::test(start_paused = true)]`。**禁止 `thread::sleep`**（必 flaky）。
- **editor 移植后**走第 4 层：加随机按键不变量（光标在界内、undo 恒等、buffer 合法 UTF-8）。
- snapshot 文件（`tests/snapshots/*.snap`）进版本控制，接受前必须人工复核（与根 AGENTS.md 一致）。

### E2E（第 5 层）

在 workspace 顶层 `tests/tui_e2e/`（非本 package）。spawn 本 package 的 `demo` example（解耦 LLM provider），验证 crossterm 真 PTY 行为 + tmux 真终端兼容性。全部 `#[ignore]`，经 `just test-tui-e2e` 跑。

## 移植规则

- 对齐 `kimi-code/packages/pi-tui` 源（`@moonshot-ai/pi-tui`）。模块对照见 `_HANDOFF.md`。
- `native-modifiers.ts` 跳过（macOS 原生二进制，不可移植）。
- `lib.rs` 的 re-export 对齐 pi 的 `index.ts`（API 边界 SSOT）。
