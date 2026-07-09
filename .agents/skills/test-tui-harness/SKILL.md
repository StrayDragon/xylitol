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

改 `packages/xylitol-tui` 或应用面 TUI 渲染相关行为时用本 skill。边界与架构决议见 `packages/xylitol-tui/AGENTS.md`；应用面接线见 `write-tui`。

## 何时用哪一层

| 层 | 验证什么 | 默认命令 / 落点 |
|---|---|---|
| 1. 按键→状态 | 组件交互、光标、提交、列表选择 | `cargo test -p xylitol-tui --test harness_test`；`tests/support/mod.rs::TuiTestHarness` |
| 2. snapshot | 整屏布局 / 换行 / 颜色回归 | `cargo test -p xylitol-tui --test snapshot_test`；`tests/snapshots/` |
| 3. 时序 | paste-burst、debounce、动画 | 同步：`Instant` 注入或 `MockClock`；async：`#[tokio::test(start_paused = true)]` |
| 4. proptest | Editor 崩溃边界与不变量 | `cargo test -p xylitol-tui --test property_test` |
| 5. E2E | 真 PTY / tmux 协议与主场景 | 全 `#[ignore]`；`just test-tui-e2e`（或 `test-tui-e2e-pty` / `test-tui-e2e-tmux`） |

日常改组件：**先 1，布局变了再 2**；动时序逻辑必加 **3**；动 Editor 状态机考虑 **4**；协议/真终端行为才上 **5**。

## 标准验证回路（改完必跑）

```bash
# 包内 1–4（快）
cargo test -p xylitol-tui

# 提交 / PR 前（仓库级）
just qa

# 真终端（慢，按需）
just test-tui-e2e
```

只改某一层时，可只跑对应 `--test`；合并前仍应用 `cargo test -p xylitol-tui` 兜底。

## 新增测试落点

- **交互**：扩 `tests/harness_test.rs`（或就近既有 harness 文件），模式：
  `TuiTestHarness::new().mount(...).keys("...").render().assert_…`
- **渲染回归**：扩 `tests/snapshot_test.rs`；接受快照：
  `INSTA_UPDATE=always cargo test -p xylitol-tui --test snapshot_test`
  接受后**人工复核** diff；`.snap` 进版本控制。
- **时序**：**禁止 `thread::sleep`**（必 flaky）。同步用 `Instant` 参数（优先）或 `MockClock`；async 用 `start_paused`。
- **不变量**：扩 `tests/property_test.rs`（光标在界内、undo 恒等、合法 UTF-8 等）。
- **E2E**：扩 `tests/tui_e2e/`；spawn `agent_demo`（单一主场景，避免 kitchen-sink 漂移）；保持 `#[ignore]`。

优先扩既有测试文件，不为小特性新建文件。

## Gotchas

- **prek trailing-whitespace** 会删 `.snap` 行尾空格；snapshot 空行不得留尾空格（`viewport_snapshot` 已处理）。
- 宽度不变量失败是 `RenderError`，不是静默截断；测试断言要对齐这一语义。
- example / demo 布局问题先查宽度预算，勿误判为终端协议缺陷。
- 第 5 层依赖本机 PTY/tmux；失败时先看是否环境缺失，再查产品逻辑。
- 应用面 seam 测试（Driver / slash / `XyEvent`→UI）不替代包内 1–4；包测通用组件，面测业务接线。

## 自检清单

- [ ] 新行为落在正确层，且有自动化覆盖
- [ ] 无新增 `thread::sleep` 时序测试
- [ ] snapshot 已人工复核（若有变更）
- [ ] `cargo test -p xylitol-tui` 绿
- [ ] 涉及协议/真终端时跑过 `just test-tui-e2e`（或说明为何跳过）
