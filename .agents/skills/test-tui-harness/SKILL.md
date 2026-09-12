---
name: test-tui-harness
description: >-
  Run and extend xylitol TUI automated verification (layered harness:
  key-sequence model asserts, insta snapshots, deterministic timing,
  proptest [planned, not yet landed], PTY/tmux E2E). Use when adding or changing packages/xylitol-tui
  components/engine, writing TUI tests, accepting snapshots, debugging flaky
  TUI tests, or validating terminal UI before handoff — even if the user only
  says "跑一下 TUI 测试" or "加个 editor 用例".
---

# TUI 分层自动验证（harness）

**边界 / 产品 vs 包 E2E 分工 SSOT**：[`packages/xylitol-tui/AGENTS.md`](../../../packages/xylitol-tui/AGENTS.md)「验证」。
应用端接线：`l8ng-write-tui`。门禁命令：根 `AGENTS.md`（`just qa` / `just qa-e2e`）。合约：`package-tui-testing`。

本 skill 只写**怎么写测 / 怎么跑**，不另立分工表。

## 何时用哪类验证

| 验证类型 | 验证什么 | 默认命令 / 落点 |
|---|---|---|
| 按键→状态 | 组件交互、光标、提交、列表选择 | `cargo test -p xylitol-tui --test harness_test`；`tests/support/mod.rs::TuiTestHarness` |
| snapshot | 整屏布局 / 换行 / 颜色回归 | `cargo test -p xylitol-tui --test snapshot_test`；`tests/snapshots/` |
| 时序 | paste-burst、debounce、动画 | 同步：`Instant` 注入或 `MockClock`；async：`#[tokio::test(start_paused = true)]` |
| proptest | Editor 崩溃边界与不变量 | **未落地**（落地时补 `packages/xylitol-tui/tests/property_test.rs`） |
| 真终端 E2E | 真 PTY / tmux · **agent_demo** | `#[ignore]`；`just test-tui-e2e`；就绪针 `DEMO_READY_NEEDLE`（见包 AGENTS「验证」） |

日常改组件：**先「按键→状态」，布局变了再 snapshot**；动时序必加时序用例；动 Editor 状态机考虑 proptest（落地后）；协议 / 真终端验证才上 E2E。

## 标准验证回路

见根 `AGENTS.md` 命令段。包内快测：`just test-tui`。

## 新增测试落点

- **交互**：扩 `tests/harness_test.rs`（或就近既有文件）：`TuiTestHarness::new().mount(...).keys("...").render().assert_…`
- **渲染回归**：扩 `tests/snapshot_test.rs`；接受：`INSTA_UPDATE=always cargo test -p xylitol-tui --test snapshot_test`（人工复核 `.snap`）
- **时序**：**禁止 `thread::sleep`**。同步用 `Instant` / `MockClock`；async 用 `start_paused`
- **不变量**：扩 `tests/property_test.rs`（proptest 落地后）
- **真终端 E2E**：扩 `tests/tui_e2e/`；spawn `agent_demo`；保持 `#[ignore]`；改首帧文案时同步 `DEMO_READY_NEEDLE`

优先扩既有测试文件。

## Gotchas

- **prek trailing-whitespace** 会删 `.snap` 行尾空格；snapshot 空行不得留尾空格。
- 宽度不变量失败是 `RenderError`，不是静默截断。
- 真终端 E2E 失败先查环境（PTY/tmux）与就绪针，再查产品逻辑。
- 应用端 harness **不替代**包侧按键→状态 / snapshot / 时序 / proptest；分工见包 `AGENTS.md`「验证」。

## 自检

- [ ] 新行为有对应验证类型且已自动化
- [ ] 无新增 `thread::sleep` 时序测
- [ ] snapshot 已人工复核（若有变更）
- [ ] `just test-tui` 绿；合并前 `just qa`
- [ ] 涉及协议/真终端：`just qa-e2e` 或说明跳过
