---
depends_on: []
---

# 交互 bang 去流式：完成即返回

## Why

用户主动触发的交互 bang（`!` / `!!`）当前把「实时输出」建模为 `execute_bash(..., chunk_tx)` 的进程内 mpsc 通道（c669）：

- 该通道只在 in-process driver 生效；产品 TUI 默认 attach Host，`XyRemoteDriver` 明确忽略 `chunk_tx`（REST 请求/响应）——**产品面从来没有实时流式**，流式是未交付能力。
- 它让 `XyDriver` 在 c2710 塌缩后仍留 `execute_bash` 例外（trait 上的会话操作），同一份 TUI 代码在 in-process / remote 下行为分叉。
- bridge/host 为此维护一组流式 UI 状态（`append_bash_output` / `append_bash_chunk` / `bash_utf8_pending` / chunk 到达 dirty 标记）。

产品定调：**bang = 服务端执行、完成即返回结果**。Pre-0.0.1 卫生：删除未交付能力的通道与状态，`Command::Bash` 成为交互 bang 唯一入口。

**范围边界（重要）**：本 change 只动「用户主动触发的终端 bang」。agent 调用 bash tool 的链路（`XyBashExecutor` 的 `chunk_tx` 端口 → ReAct 实时输出 → `ToolExecutionUpdate`）**保持不变**，infra-bash 的流式执行器 spec 照旧。

## What Changes

- `XyDriver` 删除 `execute_bash`：trait 不再含任何会话操作。
- in-process 的 bash 执行转为 inherent（`pub(crate)`，无 `chunk_tx` 参数），仅供 `SessionCommandExecutor` 的 `Command::Bash` 分支调用（内部传 `None`）。
- TUI `run_interactive_bang` 改为 `dispatch(Command::Bash)` 等待结果；保留 Esc abort、busy 硬拒绝、pending/终态渲染。
- bridge/host 删除增量接口：`append_bash_output`、`append_bash_chunk`、`bash_utf8_pending`、chunk 到达的 dirty 分支。
- live specs 改写（去流式条款，保留块语义/终态/取消）：`app-tui-bridge` atb9、`app-tui-transcript` 交互 bang 条款、`app-tui-host` 主环与 Tick 条款（去 bang chunk 臂）。

非目标：

- 不改 agent bash tool 路径（`infra-bash` 的 `XyBashExecutor` chunk 端口与 ReAct 实时输出保留）。
- 不改 bang 的取消语义（Esc → abort / cancelled 终态）与硬拒绝（第二次 bang）。
- 不改 `Command::Bash` 的 wire 形状与 host unary；不改 journal/事件面。

## Capabilities

start 后改 live spec：

- `app-tui-bridge`：atb9「bash-stream-append」→ 去增量 API，保留 pending/终态收口。
- `app-tui-transcript`：交互 bang 条款去「输出 chunk 到达时增量刷新」。
- `app-tui-host`：主环与 Tick 条款去「bang chunk」臂与 dirty 来源。

`infra-bash` **不改**（agent 工具链路保留流式端口）。

## Impact

- 嵌入模式（in-process）交互 bang 不再实时输出，完成时一次性呈现（与产品 remote 行为对齐）。
- `XyDriver` trait 无会话操作方法；c2710 的 execute_bash 例外关闭。
- 删除若干 bridge/UI 状态与 harness 流式测试。
