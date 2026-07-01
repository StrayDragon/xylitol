# c340-add-tui-inline-repl — Tasks

> 依据 c325 架构决策落地 TUI。每个文件落地即被 `tui::run` 真实驱动，不铺空骨架。chunk ≤ 2h。

## 0. 前置

- [x] 跑 `audit-dead-code` skill 分诊 `src/app/tui/`：确认 `diff_review/` 暂保留（c350 职责）、其余无死码叠加（write-surface 步骤 1，强制）
- [x] 补 crossterm `event-stream` feature（`Cargo.toml` 一行）：`crossterm = { version = "0.29.0", optional = true, features = ["event-stream"] }`
- [x] 确认 `cargo build --features tui` 通过（依赖就绪）

## 1. 终端生命周期 `terminal.rs`（RAII）

- [x] `InlineTerminal::enter(tail_height)` → raw mode + `try_init_with_options(Viewport::Inline)`
- [x] `draw_tail` / `commit_to_scrollback`
- [x] `Drop` 调 `restore()` + `disable_raw_mode()`（spec tui15）
- [ ] panic 安全：主循环包 catch_unwind 或 panic hook 确保 Drop 运行 — InlineTerminal::Drop 已保证 raw mode 恢复；显式 panic hook 作为后续小变更(defer → c355-switch-to-ratatui-core-backend)

## 2. 状态机 `app.rs`

- [x] `TuiApp`：输入缓冲 / 当前流式文本 / spinner 态 / 最近工具状态 / is_streaming
- [x] `start_stream(stream)`：spawn 转发任务 drain `EventStream` → `event_rx` mpsc（缓解 R1 borrow）
- [x] `handle_xy_event(ev) -> Option<Vec<Line>>`：覆盖 TextDelta/ThinkingDelta/ToolExecution*/ModelSelect/Error/Turn*（spec tui13），未覆盖变体 degrade 不 panic
- [x] `turn_done()` / `end_stream()`

## 3. 渲染 `render.rs`（纯函数优先）

- [x] `draw_tail_frame(frame, app)`：输入缓冲 + spinner + 当前流式行 + 工具状态画进 frame area
- [x] `commit_lines_for` 纯函数：TextDelta 累积遇 `\n` 产出完整行；ToolExecutionEnd 摘要行；TurnEnd 提交剩余
- [x] `#[cfg(test)]` 单测：喂确定性 XyEvent，断言产出行（无终端依赖）

## 4. 输入 `input.rs`

- [x] 打印字符追加缓冲，Backspace 删除，Enter 提交，Ctrl+C abort，`/` 开头识别 slash
- [x] 返回 `InputOutcome::{Submit, Slash, Abort, Quit, Idle}`
- [x] MVP 单行输入；多行编辑推迟

## 5. 命令 `commands.rs`

- [x] `/exit` → 退出；`/model` → 复用 `protocol::Command::SetModel/CycleModel` 语义
- [x] 未知 slash → 内联错误不崩溃（spec tui14）
- [x] `#[cfg(test)]` slash 解析单测

## 6. 主题 `theme.rs`

- [x] 语义 token（primary/text_dim/assistant/tool/error/spinner）映射 ratatui Color
- [x] 组件不得硬编码颜色字面量

## 7. 主循环 `mod.rs`

- [x] `run()` REPL：tokio::select! 三源（event_rx / 键盘 EventStream / cancel）（spec tui11）
- [x] 流式行 finalize → commit_to_scrollback（spec tui12）

## 8. Mode 分发接入 `cli/mod.rs`

- [x] 无 prompt 且 stdin 是 TTY 且非 --rpc → `tui::run`（对齐 pi resolveAppMode）；`--tui` flag 强制入口
- [x] 装配复用 cli/mod.rs 既有流程（最小调用，c330 full 化后切换）

## 9. 更新文档

- [x] `src/app/tui/AGENTS.md`：现状从「未落地」改为 live
- [x] `src/app/tui/mod.rs`：替换 `pub mod diff_review;` 为真实模块声明（保留 diff_review 声明）

## 10. 验收与校验

- [x] 端到端：`cargo build --features tui --bin xylitol` 通过 + `--tui` flag 在 `--help` 显示 + 30 个 TUI 单测覆盖逻辑链（真终端交互需 TTY+API key，单测代理验证）
- [x] `arch_guard` 无 `agent::session`/`runtime`/`infra` 违规 import（spec tui4）— XyEvent 从 domain 层取
- [x] `just qa` 绿（fmt + clippy + test + docs；docs 29 warning 为既有 rustdoc 标签误判，非本次引入）
- [x] `llman sdd validate c340-add-tui-inline-repl --strict --no-interactive` 通过
