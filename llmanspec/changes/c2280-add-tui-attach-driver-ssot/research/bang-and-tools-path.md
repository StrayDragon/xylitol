# bang、agent bash、工具审批：三条路径

> 对照代码。工作区在 server 上跑。剪贴板仍在 TUI。英文专名见 [`glossary.md`](./glossary.md)。

## 先分清：两种 bash，现在却共用一个 Driver 方法

人打的 `!ls` 和模型调用的 `bash` 工具，**产品不是一回事**。今天却都可能挤进 `XyDriver::execute_bash`（`proto.rs`）。拆到 CS 时要拆成两条 API，执行地点都是 **server 的工作区**。

| | 你打的 `!` / `!!` | 模型的 bash 工具 |
|---|---|---|
| 谁发起 | 人，输入框 `!ls` | 模型在一轮里决定跑命令 |
| 现在入口 | TUI `run_interactive_bang` → `driver.execute_bash(..., Some(chunk_tx))`（`effects/bang.rs`） | ReAct → `infra/tools/bash.rs` → `ToolStart` / `ToolEnd` |
| 屏幕 | 一块「正在跑命令」的 bash 区，字节往里追加 | 一条工具卡片，有开始/结束/流式输出 |
| Esc | `driver.abort()`，文案是 bang 取消，**不是** agent Aborted（`bang.rs` + TUI AGENTS） | busy 无 overlay 才 abort **这一轮 agent** |
| 记会话 | `BangExecHandler` 写成 `bash_execution` 条目；`!!` 的 `exclude_from_context` 不进模型上下文 | 工具调用记在 assistant/tool 消息里 |
| 审批 | 人已经敲了命令，一般不再问一次 | 可能先 reverse RPC 问你同不同意 |
| Remote 现状 | `XyRemoteDriver::execute_bash` **丢掉** `chunk_tx`，REST 等跑完才返回（`remote.rs` 注释） | 走 Event 流 `ToolExecutionUpdate` |

`dispatch` 里的 `Command::Bash` 也调 `execute_bash(..., None)`（没有直播通道）。Print / 远程 REST 走这条。

**结论：** 两条 API。共用一个 `XyBashExecutor` 端口去真正 `spawn` 可以；**不要**再让 TUI bang 和 LLM 工具共用同一个 Driver 方法语义。

## 路径 1：你打字，屏幕多几个字（模型回合）

你输入 `fix the bug`，回车。TUI 不调模型。

1. TUI 发出一条 **Command**：`{"type":"prompt","message":"fix the bug"}`（`Command::Prompt`）。
2. Server 开一轮。模型先吐 `Hel`：server 推一条 **Event**：`{"type":"text_delta","text":"Hel"}`。TUI 追加到助手气泡。再推 `"lo"`。画面上就像字往外蹦。
3. 模型要改文件。server 在工作区跑工具，推 `tool_start` / `tool_execution_update` / `tool_end`。TUI 画工具块。
4. 一轮结束：`agent_end` / `turn_end`。你又可以打字。

这些 JSON 就是 **wire protocol**（线协议）。换 HTTP 栈只换路由和 listener，不换 `Command` / `Event`。

## 路径 2：你打 `!ls`（bang，工作区里的 shell）

1. 你敲 `!ls`。TUI 认出这是 bang，不是 prompt（`run_interactive_bang`）。
2. TUI 发出 **Command::Bash**（或等价的 `execute_bash`）：在工作区跑 `ls`。
3. **Server** 在它的 cwd / docker 里 spawn。stdout 应作为 **Event** 一块块推回来。TUI `append_bash_chunk`。你看见的是工作区目录，不是笔记本家目录。
4. 进程结束，TUI `push_bash_result`（对应 `Event::BashResult`）。`!!` 则写盘但不喂给下一轮模型。
5. 跑的时候按 Esc：TUI 发 `Abort`，server 杀掉 **这棵 bash**，agent 回合（如果有）另算。

### attach 必做：bang 直播（live chunks）

进程内：`chunk_tx: mpsc::Sender<Vec<u8>>`，TUI 边收边画（c669）。

`XyRemoteDriver::execute_bash` 把 `chunk_tx` 标成 `_`，REST 一问一答。产品 `--attach` **MUST NOT** 停留在这种语义：长命令会整段卡住，Esc 取消也和直播环对不上。

实现票最小形状（仍用 tagged JSON，不引入 JSON-RPC）：

- 增加 wire **Event**，专给 bang，不要复用 `ToolExecutionUpdate`（UI / Esc / 会话条目都不同）。**已钉：`BashDelta`**（增量字节或文本）+ 已有 `BashResult`。
- 走 **同一条 WebSocket 订阅**（journal 或旁路同一连接），TUI 才能在 `select!` 里一边收 chunk 一边收键。
- `Abort` 在 bang 进行中必须能打到 server 那棵进程树（已有 `BangExecHandler` 的 cancel token，Remote 要接到这条 Command）。

这和选 axum / poem **无关**。是 Remote Driver 的缺口。

## 路径 3：模型要跑工具，还要你点头（审批）

1. 模型发出工具调用。server 认为要人批。
2. server 不直接跑。它登记一个 `call_id`，经 WebSocket 做 **reverse RPC**（`ReverseRpcGateway::register`，`ws.rs`）。
3. TUI 弹出选项。你选是/否。TUI 发 `{"type":"approve_tool","call_id":"...","approved":true}`（`Command::ApproveTool`）。
4. 第一个回答算数；晚到的丢掉。server 才 spawn 工具。然后仍走 `tool_start` → 输出 → `tool_end`。
5. **卡住：** 默认等 **60 秒**（`REVERSE_RPC_TIMEOUT`）。超时当拒绝/超时结果，agent 那一轮会看见失败而不是永久挂起。

问卷（`Command::AnswerQuestion`）同一套网关，回的是字符串不是 bool。

「拖死」还有别的：工具自己的超时（bash 工具注释里默认可 unlimited、上限 120s）、你 abort 整轮、journal 满了要求 `ResyncRequired`。那些不是 HTTP 栈问题。

## tagged JSON vs JSON-RPC（本票：前者）

路径 1 现在每蹦几个字就一条小 JSON Event。JSON-RPC 会再包 `method` / `id` envelope。本票 **保持 tagged JSON**。jsonrpsee 不够 4000 星，见 [`http-json-tokio-top6.md`](./http-json-tokio-top6.md)。
