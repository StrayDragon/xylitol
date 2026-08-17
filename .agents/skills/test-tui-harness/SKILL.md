---
name: test-tui-harness
description: >-
  Run and extend xylitol TUI automated verification (five-layer harness:
  key-sequence model asserts, insta snapshots, deterministic timing,
  proptest, PTY/tmux E2E). Use when adding or changing packages/xylitol-tui
  components/engine, writing TUI tests, accepting snapshots, debugging flaky
  TUI tests, or validating terminal UI before handoff — even if the user only
  says "跑一下 TUI 测试" or "加个 editor 用例".
---

# TUI 五层自动验证（harness）

**边界 / 产品 vs 包 E2E 分工 SSOT**：[`packages/xylitol-tui/AGENTS.md`](../../../packages/xylitol-tui/AGENTS.md)「验证」。
应用面接线：`l8ng-write-tui`。闸门命令：根 `AGENTS.md`（`just qa` / `just qa-e2e`）。合约：`package-tui-testing`。

本 skill 只写**怎么写测 / 怎么跑**，不另立分工表。

## 何时用哪一层

| 层 | 验证什么 | 默认命令 / 落点 |
|---|---|---|
| 1. 按键→状态 | 组件交互、光标、提交、列表选择 | `cargo test -p xylitol-tui --test harness_test`；`tests/support/mod.rs::TuiTestHarness` |
| 2. snapshot | 整屏布局 / 换行 / 颜色回归 | `cargo test -p xylitol-tui --test snapshot_test`；`tests/snapshots/` |
| 3. 时序 | paste-burst、debounce、动画 | 同步：`Instant` 注入或 `MockClock`；async：`#[tokio::test(start_paused = true)]` |
| 4. proptest | Editor 崩溃边界与不变量 | `cargo test -p xylitol-tui --test property_test` |
| 5. E2E | 真 PTY / tmux · **agent_demo** | `#[ignore]`；`just test-tui-e2e`；就绪针 `DEMO_READY_NEEDLE`（见包 AGENTS「验证」） |

日常改组件：**先 1，布局变了再 2**；动时序必加 **3**；动 Editor 状态机考虑 **4**；协议/真终端才上 **5**。

## 标准验证回路

见根 `AGENTS.md` 命令段。包内快测：`just test-tui`。

## 新增测试落点

- **交互**：扩 `tests/harness_test.rs`（或就近既有文件）：`TuiTestHarness::new().mount(...).keys("...").render().assert_…`
- **渲染回归**：扩 `tests/snapshot_test.rs`；接受：`INSTA_UPDATE=always cargo test -p xylitol-tui --test snapshot_test`（人工复核 `.snap`）
- **时序**：**禁止 `thread::sleep`**。同步用 `Instant` / `MockClock`；async 用 `start_paused`
- **不变量**：扩 `tests/property_test.rs`
- **E2E**：扩 `tests/tui_e2e/`；spawn `agent_demo`；保持 `#[ignore]`；改首帧文案时同步 `DEMO_READY_NEEDLE`

优先扩既有测试文件。

## Gotchas

- **prek trailing-whitespace** 会删 `.snap` 行尾空格；snapshot 空行不得留尾空格。
- 宽度不变量失败是 `RenderError`，不是静默截断。
- 第 5 层失败先查环境（PTY/tmux）与就绪针，再查产品逻辑。
- 应用面 harness **不替代**包 1–4；分工见包 `AGENTS.md`「验证」。

## 自检

- [ ] 新行为落在正确层且有自动化
- [ ] 无新增 `thread::sleep` 时序测
- [ ] snapshot 已人工复核（若有变更）
- [ ] `just test-tui` 绿；合并前 `just qa`
- [ ] 涉及协议/真终端：`just qa-e2e` 或说明跳过
