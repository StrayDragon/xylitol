# Design — c660 abort → process tree

## Gap（代码审计）

| 路径 | 现状 | 缺口 |
|---|---|---|
| ReAct 工具 bash | `XyToolCtx.cancel` = run token；`RealBashOperations` select cancel → `kill_process_tree` | 无（保持） |
| 交互 `!`/`!!` | `BashExecHandler` 自有 token；`abort_bash()` 可 cancel | `AgentRuntime::abort` / `Driver::abort` **未**调 `abort_bash` |
| Server | `DELETE …/session` → `driver.abort()` | 随 InProcess 修复自动继承 |

对齐 pi：`interactive-mode` 在 streaming 调 `agent.abort()`，在 bash 跑着调 `session.abortBash()`；xylitol 统一入口 `abort()` 应两者都做。

## 决策

1. **`abort()` 同时取消 run token + 交互 bash**（不必让 TUI 分支两套 API）。
2. **`BashExecHandler::abort` 改为 `&self`**：`Mutex<Option<CancellationToken>>`，与 `clear_steer_queue` 一致，满足 `Driver::abort(&self)`。
3. **不**在 `xylitol-tui` / 产品 TUI 改文案（c665）。

## OS / 失败模式

- Unix：`kill_process_tree` SIGTERM→SIGKILL（infra-process）；交互路径经 `kill_tree`。
- Windows：进程树杀为最佳努力；无法保证时仍标记 `cancelled`，用户可见警告留给 c665。
- `pid == 0`：noop（既有）。

## 产品 host 已知限制（本 change 不扩）

`run_host_loop` 在 `drain_pending` 内 **await** `Command::Bash`，期间不读键盘；Esc 要等 bang 结束后才进 `take_abort`。内核 `abort`→`abort_bash` 已闭环，但 **Esc 打断进行中的 `!` 命令** 还需 host 把 bash 抬进 `select!`（或 `AbortToken` 克隆）——记入后续（可挂 c665 或独立 change），本 change 不改 TUI 键位/文案。

## 测试

- `BashExecHandler`：并发 `sleep` + `abort` → `cancelled == true`。
- `AgentRuntime`：`Arc` 上并发 `execute_bash` + `abort` → `cancelled`。
- 既有：`infra/bash_exec` `cancellation_kills_process`；工具 bash cancel 测保持。
