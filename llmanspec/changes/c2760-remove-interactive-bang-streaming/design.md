# Design: 交互 bang 去流式

> Designed / pre-start。无硬依赖（c2710 已归档，`Command::Bash` 执行器已存在）。

## 1. 目标

交互 bang 的执行路径 = `dispatch(Command::Bash)` → `SessionCommandExecutor`，完成即返回；删除进程内 `chunk_tx` 面通道与流式 UI 状态。`XyDriver` 不再含 `execute_bash`。

## 2. 代码事实

| 路径 | 现状 |
|---|---|
| `src/app/core/driver/proto.rs` | `execute_bash(..., chunk_tx)` trait 声明（c2710 注记的例外） |
| `src/app/core/driver/in_process/mod.rs` | trait impl 内实现（被 executor 的 `Command::Bash` 经 `self.execute_bash(..., None)` 调用） |
| `src/app/core/driver/remote.rs` | trait impl：忽略 `chunk_tx`，`unary_cmd(Command::Bash)` |
| `src/app/tui/effects/bang.rs` | `execute_bash(Some(chunk_tx))` + `select!` 收 chunk → `session.append_bash_chunk` |
| `src/app/tui/bridge/model.rs` | `append_bash_output` + `bash_utf8_pending`（增量追加，c669） |
| `src/app/tui/host/mod.rs` | `append_bash_chunk`（mark paint_dirty，ath24） |
| `src/app/tui/harness.rs` | trait impl 的 `execute_bash`；c669 流式测试（pending tint / chunks-before-done） |
| `src/infra/tools/bash.rs` | `XyBashExecutor` 的 `chunk_tx` 端口（agent bash tool 实时输出；**保留**） |
| `src/app/core/bang_exec.rs` | `XyBashRunner::execute(..., chunk_tx, ...)`（**保留**，agent 链路仍用） |

## 3. 目标形状

```text
交互 bang:
  begin_bash_exec → dispatch(Command::Bash) → 结果 → push_bash_result / note_bash_cancelled
（唯一入口 = Command；无本地 chunk 通道）

agent bash tool:
  BashTool → XyBashExecutor(chunk_tx) → ReAct 流 → ToolExecutionUpdate（不变）
```

### in-process 执行器

`execute_session_command` 的 `Command::Bash` 分支改为调 inherent：

```rust
pub(crate) async fn execute_bash_impl(
    &self,
    command: &str,
    exclude_from_context: bool,
) -> Result<XyBashResult, XyDriverError>
```

body 沿用现 trait 实现（script hook + `bang.execute(..., None, cwd)`），去掉 `chunk_tx` 参数。

### TUI bang 循环

`run_interactive_bang` 的 `select!` 保留 `bash_fut` / ticker / input 臂，去掉 `chunk_rx` 臂；`dispatch` 返回后照旧 `push_bash_result`（成功）或 `note_bash_cancelled`（abort 路径）。Esc / busy 硬拒绝语义不动。

## 4. Specs 改写（live，绑定分支）

- `app-tui-bridge` atb9：改为「交互 bang 块 MUST 以 pending 状态表示执行中；完成/取消时经既有 finish/cancelled 路径切换终态；MUST NOT 每块新建独立滚动行」——删除「增量追加 API」与「流式 chunk 追加期间」条款。
- `app-tui-transcript`：删除「输出 chunk 到达时 MUST 在同一块内增量刷新」；保留 pending→终态轨色、cancelled 呈现、exit_code error。
- `app-tui-host`：
  - 主环条款：`可选 bang 完成与 bang chunk` → `可选 bang 完成`；删除「chunk 到达 MUST 仅标记 dirty…」句（保留 Tick/BangDone 渲染与 agent 不饿死）。
  - Tick 条款：`（含 bang chunk 追加）` → `（含 bang 完成）`。
- `rules_edit_acked: true`（多条 @human 修改）。

## 5. 验证

- harness：删除 c669 流式断言（`c669_append_bash_keeps_pending_tint`、`c669_scripted_streams_chunks_before_done`），保留/平移非流式断言（bash_calls、第二 bang 硬拒绝、Esc cancelled）。
- `just test-tui` + `just qa`；BDD `app-tui-bridge` / `app-tui-host` 场景重跑。
- agent bash tool 链路测试（`steps_agent_tools::test_bash_cancel` 等）必须保持绿。
