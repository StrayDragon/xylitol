---
change_id: c405-add-tui-test-harness
title: "add five-layer TUI test harness — 覆盖按键序列/时序/快照/状态机/真终端动态行为"
status: draft
priority: 405
depends_on: []
author: agent
---

# c405-add-tui-test-harness

## Why

`packages/xylitol-tui` 正在完整移植 pi-tui（当前 189 测试 / 22 模块对齐），即将替换 `src/app/tui/engine/`。但在调研 pi-tui 测试机制后发现：**现有 harness 能验证「动态行为」的能力严重不足**，无法支撑接下来的 editor/paste-burst/autocomplete 完整移植（三者都有复杂的按键序列 + 时序逻辑）。

### 证据：三方对比

| 维度 | pi-tui（TS 参考） | xylitol-tui（现状） | 差距 |
|---|---|---|---|
| 测试规模 | 13,341 行测试 / 7,642 源码 ≈ 1.75:1 | 189 测试 | 数量级差距 |
| 键盘序列→状态 | 数百个 `handleInput(seq)` case | 仅 1 个真端到端 case（测单字符 `'a'`） | editor/input 的交互几乎没测 |
| 时序测试 | `mock.timers` 冻结时钟（paste-burst/debounce） | 仅 render 节流（`tokio::test`） | paste-burst/autocomplete debounce 无法测 |
| snapshot | 刻意不用（TS ANSI 脆弱） | 无 | Rust 的 insta 有 inline+diff review，适用 |
| E2E 真终端 | 无 | 无 | crossterm 真实事件解析 + 真终端兼容性无验证 |

### 证据：现状盲区

- `stdin_buffer.rs`（158 行）—— **零测试**，逻辑复杂（bracketed-paste 边界、CSI/OSC 拆分）
- `autocomplete.rs`（534 行）—— **零测试**
- `editor.rs`（408 行）—— 7 个测试全是直调 API，**没有经 `handle_input`/`dispatch_input` 的按键序列测试**
- 差分渲染「同实例第二帧」—— 受 `Box<dyn Component>` 所有权限制，现有测试重建了第二个 TUI 实例（注释自认 workaround）

### 证据：业界共识（helix/zellij/ratatui）

- **helix**：`test_key_sequence!` 宏——按键序列→**model 断言**（不测渲染），数百 case。教训：model 转换用状态断言大量测，渲染用 snapshot 少量精测。
- **zellij**：`Grid` 用 `vte` crate（与本项目同源）做单元测试，断言 grid 状态。
- **ratatui 官方**：推荐 `insta` snapshot + `TestBackend`。
- **业界共识**：时序一律用 mock clock（Rust 用 `#[tokio::test(start_paused=true)]`），禁止 `thread::sleep`（必 flaky）；PTY 级集成测试只做冒烟（3-5 条），不进主矩阵。

### 为什么 c405 排在 editor/paste-burst/autocomplete 移植之前

**用户明确决策（路线 B：先补齐 package 再替换 src/app/tui/engine/）**。完整移植 editor/paste-burst/autocomplete 前必须有测试基建，否则移植出来的代码无法验证「动态行为是否对齐 pi」——单测只能验证 API 直调，按键序列交互和时序逻辑会重蹈 c399 重写时的盲区。本变更**建立可自动化验证的机制**，是后续所有移植工作的前置依赖。

## What Changes

建立**五层 TUI 测试架构**，每层职责不重叠，覆盖从纯函数到真终端的全部动态行为：

### 第 1 层：按键序列 → 状态断言（抄 helix）

通用化现有局部 `MutableComponent`（现仅在 `virtual_terminal_test.rs`）进 `tests/support/`，加 `keys(seq)` / `assert_state` / `assert_screen_contains` / `assert_cursor` 便捷断言 helper。不引入新依赖。

覆盖目标：editor/input/select_list/settings_list 的按键交互，对齐 pi `input.test.ts`（16 case）的密度。

### 第 2 层：insta 整屏 golden（ratatui 官方推荐）

`viewport()` 输出直接喂 `insta::assert_snapshot!`。每个组件/屏幕状态一张快照，`cargo insta review` 接受。手写 `assert_eq` 继续用于精确结构断言，两者混用。

引入依赖：`insta = "1"`（dev-dependency）。

### 第 3 层：时序测试基础设施

移植 paste-burst/autocomplete 时同步建。两条路：
- **Clock trait 注入**（paste-burst 同步逻辑）：`PasteBurst::new(clock: impl Clock)`，测试传 `MockClock`
- **tokio paused time**（autocomplete debounce）：`#[tokio::test(start_paused=true)]` + `tokio::time::advance`

**关键约束**：时间源必须可注入，禁止 `Instant::now()` 硬编码（否则无法确定性测试）。

### 第 4 层：proptest 状态机（editor 移植完成后）

定义 editor 不变量（光标在界内、undo 恒等、buffer 合法 UTF-8），随机按键 1000 次/case 抓崩溃。引入依赖：`proptest = "1"`（dev-dependency）。

### 第 5 层：E2E 终端集成（portable-pty 主力 + tmux 冒烟）

- **5a. portable-pty**（in-process 主力）：spawn 二进制 + 喂按键序列 + 读字节流喂给现有 `VirtualTerminal` → cell-grid 断言。测 crossterm 真实事件解析（Ctrl+方向键、Alt+方向键多字节序列）。
- **5b. tmux**（真终端冒烟，3-5 条）：手写 ~80 行薄 wrapper（不引 crate），`capture-pane -e -p` 抓带 SGR 屏幕，`send-keys` 注入按键。验证真终端兼容性 + 颜色回归。

引入依赖：`portable-pty = "0.9"`（dev-dependency）。

## Capabilities

- `tui-testing`（新建：五层测试架构 + 动态行为测试规范）
- `test-infra`（modify：加 E2E 测试隔离约束）

## Impact

- `packages/xylitol-tui/Cargo.toml`：dev-dependencies 加 `insta`、`proptest`、`portable-pty`
- `packages/xylitol-tui/tests/support/mod.rs`：通用化 `MutableComponent` + 键序列 helper + Clock trait
- `packages/xylitol-tui/tests/`：新增 snapshot 目录、property 测试文件
- `tests/tui_e2e/`（workspace 顶层新增）：portable-pty 集成测试 + tmux wrapper + 冒烟测试
- `justfile`：加 `test-tui-e2e` 任务
- `llmanspec/specs/tui-testing/spec.toon`（新建）
- `llmanspec/specs/test-infra/spec.toon`（modify：加 E2E 隔离要求）

## Non-goals

- 本变更**不移植**任何 pi-tui 功能（editor 扩展、paste-burst、autocomplete 补全）。那些是后续独立变更（c410+）。
- 本变更**不替换** `src/app/tui/engine/`。替换是后续变更（依赖本 harness 建好后）。
- 第 4 层 proptest 的**具体不变量定义**留给 editor 移植变更（c410+），本变更只建骨架。
