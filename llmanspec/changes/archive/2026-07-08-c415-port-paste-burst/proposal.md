---
change_id: c415-port-paste-burst
title: "port paste-burst — 非 bracketed paste 的 Enter 抑制检测器（pi paste-burst.ts → Rust）"
status: draft
priority: 415
depends_on: []
author: agent
---

# c415-port-paste-burst

## Why

`paste-burst.ts` 是 pi-tui 独有模块（61 行，editor.ts 依赖它），用于检测**非 bracketed paste 的粘贴突发**：用户在终端粘贴一大段文本（终端不发出 bracketed paste 标记时），紧接着的 Enter 应插入换行而非提交草稿。没有它，粘贴带换行的内容会意外提交。

按 _HANDOFF §二 阶段 6.2，paste-burst 是 editor（6.4）的依赖——editor 移植前必须先有它。c405 第 3 层 Clock/MockClock 已就位，本变更验证「时序逻辑确定性测试」机制。

### 证据

pi `editor.ts` 在 5 处调用 PasteBurst：
- `handleInput` 里每个普通字符触发 `onPlainChar(Date.now())`（:973/982）
- Enter 提交前调 `shouldInsertNewlineInsteadOfSubmit(Date.now())`（:900）
- 满足条件则 `extendWindow(Date.now())` + 插入换行（:902）
- 非普通字符/非 Enter 时 `reset()`（:705/720/733）

pi 用 `Date.now()` 硬编码时间——xy 用 c405 的 `Clock` trait 注入，测试用 `MockClock` 确定性控制时间（c405 tt04 已验证此模式）。

## What Changes

1. **新建 `packages/xylitol-tui/src/paste_burst.rs`**：移植 pi `PasteBurst` 类为 `PasteBurst` struct。
   - 4 个方法：`on_plain_char(now: Instant)` / `should_insert_newline_instead_of_submit(now: Instant) -> bool` / `extend_window(now: Instant)` / `reset()`
   - 4 个常量（对齐 pi）：`MIN_CHARS=8` / `CHAR_INTERVAL=8ms` / `ACTIVE_IDLE_TIMEOUT=30ms` / `ENTER_SUPPRESS_WINDOW=120ms`
   - **关键设计**：方法接受 `Instant` 参数（而非内部调 `Instant::now()`），让时间源可注入。调用方（未来 editor）传 `clock.now()`。

2. **lib.rs 导出** `PasteBurst` + 4 个常量。

3. **测试（c405 第 1+3 层）**：
   - 第 1 层（纯函数单测）：状态机行为——快速 8 字符触发 burst、慢速不触发、Enter 抑制窗口、reset 清除
   - 第 3 层（时序）：窗口边界（7ms 内 vs 9ms 外）、用 MockClock 确定性推进时间（对应 c405 tt04 paste-burst-window-boundary scenario）

4. **对齐 pi 测试**：pi `editor.test.ts:42-60` 的 `PasteBurst` describe 块（6 个测试）逐一移植。

## Capabilities

- `paste-burst`（新建）

## Impact

- `packages/xylitol-tui/src/paste_burst.rs`（新建，~70 行）
- `packages/xylitol-tui/src/lib.rs`（导出）
- `packages/xylitol-tui/tests/paste_burst_test.rs`（新建，~10 测试）
- `llmanspec/specs/paste-burst/spec.toon`（新建）

## Non-goals

- **不集成进 editor.rs**——editor 还没移植 PasteBurst 的调用点（那是 c420 editor 移植的范围）。本变更只移植独立模块 + 测试。
- **不改 crossterm/terminal**——paste-burst 是纯逻辑，不碰 I/O。
- **不处理 bracketed paste**——那是 stdin_buffer/editor 的职责（bracketed paste 有明确标记，不需要 burst 检测）。
