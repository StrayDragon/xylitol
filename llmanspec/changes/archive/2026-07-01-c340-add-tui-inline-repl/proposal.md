---
change_id: c340-add-tui-inline-repl
title: 落地 TUI 应用面（inline 渲染 REPL，经 InProcessDriver）
status: proposed
priority: 340
depends_on:
  - c325-add-app-tui-spec
author: agent
---

# c340-add-tui-inline-repl

## Why

xylitol 的 TUI 应用面此前两次尝试落地均失败。本变更依据 c325 沉淀的架构决策，**第三次**落地 TUI，并把风险压到与已验证的 print 模式同级。

### 为什么这次能成（前两次失败的根因已被消解）

1. **inline 渲染的复杂度被 ratatui 消解**：前两次的雷区之一是「inline 要自写差分渲染器」。但 ratatui 0.30.2 的 `Terminal::insert_before` 是稳定公开 API（证据见 c325 证据 A），`Viewport::Inline(N)` + `draw()`(尾部) + `insert_before()`(scrollback commit) 三件套原生实现了 codex/pi 的「stable region + mutable tail」模型，**无需手搓 `\x1b[2K` 差分渲染**。最小 inline chat 渲染器约 100-200 行。
2. **TUI 与 print 同路径**：TUI 经 `InProcessDriver`（print 已验证在用的同一条 `Driver::run → XyEvent` 流），不骑任何未验证的 daemon/socket（证据见 c325 证据 B）。TUI 的真实风险面只剩「ratatui 事件循环 + XyEvent 渲染 + abort 接线」，全部基于已验证的 seam。
3. **不铺空骨架**：每个新建文件当本变更内即被 `run()` 真实驱动（write-surface 铁律），无 `#[allow(dead_code)]`。

### 调研证据（详见 c325）

- inline 渲染：ratatui 0.30.2 `insert_before` 稳定（`ratatui-core-0.1.2/src/terminal/inline.rs:109`）。
- in-process 连接：codex `AppServerTarget::Embedded` 默认、pi `InteractiveMode` 直接持 `AgentSessionRuntime`。
- 文件布局对标 kimi-code `apps/kimi-code/src/tui/`（mod/render/input/commands/theme/components）+ write-tui skill 目标布局。

## What Changes

### 前置：Cargo 依赖补全（一行）
`crossterm` 补 `event-stream` feature（异步键盘事件进 `tokio::select!`）。ratatui 保持现状（默认 features，**不**加 `scrolling-regions`/unstable —— `insert_before` 无需它们，有稳定 fallback）。

### Mode 分发接入（`src/app/cli/mod.rs`）
对齐 pi 的 `resolveAppMode`：无 prompt 且 stdin 是 TTY → TUI；有 prompt → print；`--rpc` → rpc。复用既有 config/registry/composition 流程，TUI 接收构造好的 `InProcessDriver`。

### 文件布局（`src/app/tui/`，按需逐个建，不铺空骨架）
```
src/app/tui/
├── AGENTS.md          # 更新：标记 TUI 已落地（现状从「未落地」改为 live）
├── mod.rs             # run() REPL：tokio::select! 事件循环
├── terminal.rs        # 终端生命周期：Viewport::Inline(N) + raw mode + Drop 保证恢复
├── render.rs          # XyEvent → ratatui：draw()(tail) + insert_before()(scrollback)
├── input.rs           # crossterm EventStream → 键位/单行输入（Enter 提交）
├── commands.rs        # slash 解析（/exit→Quit, /model→SetModel/CycleModel）
├── theme.rs           # 语义颜色 token 单一真值源（对标 kimi-code colors.ts，无硬编码）
└── diff_review/       # 保留不动（c350 的职责）
```

### 事件循环核心（mod.rs，对标 codex 简化版）
```rust
loop {
    tokio::select! {
        Some(ev) = event_rx.recv() => { app.handle_xy_event(ev); terminal.draw_tail(&app); }
        key = input.next() => { /* 解析 slash 或提交 prompt → driver.run */ }
        _ = cancel.cancelled() => { driver.abort(); break; }
    }
    // 流式行 finalize 时 → terminal.insert_before(commit_line)
}
```
MVP **不实现** codex 的 FrameScheduler（帧合并限速）和 EventBroker pause-resume（外部编辑器）——按 write-surface「不建抛型骨架」，等真正遇到性能/编辑器需求再加（届时作为独立变更）。

### 复用（内部小范围，不为抽象而抽象）
- **装配**：直接调用 `cli/mod.rs` 既有装配路径（build_agent + model 选择 + trust/resource 发现）。完整共享抽取是 c330 的职责；c340 落地时若 c330 未 full 化，临时最小调用，等 c330 full 化后零成本切换。
- **命令**：`commands.rs` 复用 `protocol::Command` 语义。完整共享 dispatch 是 c335 的职责；c340 落地时若 c335 未 full 化，TUI 内最小本地分派（仅 /exit /model 两条），等 c335 full 化后切换。

## Capabilities

- `app-tui`（修改/填充）：本变更把 c325 声明的架构约束**实体化**为可运行代码。

## Impact

- **受影响代码**：新增 `src/app/tui/{mod,terminal,render,input,commands,theme}.rs`；改 `src/app/cli/mod.rs`（mode 分发）、`Cargo.toml`（crossterm event-stream）、`src/app/tui/AGENTS.md`。
- **受影响规范**：`app-tui`。
- **风险**：低（与 print 同 Driver 路径）。主要不确定性在 ratatui inline 视觉效果，但有稳定 fallback。

## 反降级护栏（防止本变更被降级为 alt-screen 或半成品）

- [ ] TUI MUST 用 `Viewport::Inline`（非 `Fullscreen`/alt-screen）——违反则与 c325 的 MUST 冲突。
- [ ] TUI MUST 经 `InProcessDriver`，MUST NOT 默认骑 RemoteDriver/daemon。
- [ ] 每个新建文件（mod/terminal/render/input/commands/theme）MUST 当本变更内即被 `run()` 真实驱动，MUST NOT 出现 `#[allow(dead_code)]`。
- [ ] 端到端验收 MUST 通过：`cargo run --features tui --` 进入 REPL → 输入 prompt → 看到 `TextDelta` 流式追加 → `/model` 切换 → `/exit` 退出，历史留在 scrollback。
- [ ] `arch_guard` MUST 无 `agent::session`/`agent::runtime`/`infra` 违规 import。
- [ ] `just qa` MUST 绿。
- [ ] `render.rs`/`commands.rs` 的纯函数 MUST 有单测（XyEvent→渲染行 / slash 解析）；端到端用 `infra/provider/fake.rs` 的 `FakeModel` 喂确定性事件。
