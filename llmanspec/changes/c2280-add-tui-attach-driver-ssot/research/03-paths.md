# 03 消息路径与现状缺口（prompt / bang / 审批，非裁决）

> 本篇讲三种交互的协议流程、两种 bash 语义的差异、以及现状里与「同名方法但语义不同」相关的坑。信封选哪种、是否直播、Event 命名等裁决在提案（「研究结论」+「BashDelta 实现票种子」）；本篇只给流程与缺口事实。

## 一种协议的两种信封形状（事实）

当前线协议是 tagged JSON：

```json
{"type":"text_delta","text":"Hel"}
```

JSON-RPC 2.0 是另一种信封：

```json
{"jsonrpc":"2.0","method":"prompt","params":{"message":"fix the bug"},"id":"1"}
```

信封换了，消息种类还是那些。相关背景事实：「没有 ≥4000 星的专用 JSON-RPC 服务端（jsonrpsee 851）」；gRPC 需换 protobuf 词汇见 `01`。

## 两种 bash 语义的差异（人 `!` vs 模型工具）

| | 你打的 `!` / `!!` | 模型的 bash 工具 |
|---|---|---|
| 谁发起 | 人，输入框 `!ls` | 模型在一轮里决定跑命令 |
| 现在入口 | TUI `run_interactive_bang` → `driver.execute_bash(..., Some(chunk_tx))`（`effects/bang.rs`） | ReAct → `infra/tools/bash.rs` → `ToolStart` / `ToolEnd` |
| 屏幕 | 一块「正在跑命令」的 bash 区，字节往里追加 | 一条工具卡片（开始 / 结束 / 流式输出） |
| Esc | `driver.abort()`，文案是 bang 取消，不是 agent Aborted | busy 无 overlay 才 abort 这一轮 agent |
| 记会话 | `BangExecHandler` 写 `bash_execution` 条目；`!!` 的 `exclude_from_context` 不进模型上下文 | 记在 assistant/tool 消息里 |
| 审批节奏 | 人已敲了命令，一般不再问一次 | 可能先反向 RPC 问同意 |
| Remote 现状 | `XyRemoteDriver::execute_bash` 丢掉 `chunk_tx`，REST 等跑完才返回（`remote.rs` 注释） | 走 Event 流 `ToolExecutionUpdate` |

事实关联：`dispatch` 里的 `Command::Bash` 也调 `execute_bash(..., None)`（无直播通道）；Print / 远程 REST 走这条。两种 bash 现在**共用** `XyDriver::execute_bash`；若要共用同一个 `XyBashExecutor` 端口去真正 `spawn` 可以，但两条语义不同（发起方、屏幕、Esc、会话记录、审批都不同）。

## 路径 1：prompt（模型回合）

`Command::Prompt` → 模型吐 `TextDelta` → 工具 `ToolStart` / `ToolExecutionUpdate` / `ToolEnd` → `AgentEnd` / `TurnEnd`。TUI 只显示。这些 JSON 就是线协议；换 HTTP 栈只换路由和 listener，不换 Command / Event。

## 路径 2：bang（`!ls`，工作区里的 shell）

1. TUI 认出 bang（`run_interactive_bang`），发 `Command::Bash`。
2. server 在它的 cwd / docker 里 spawn；stdout 作为 Event 一块块推回，TUI `append_bash_chunk`（你看见的是工作区目录，不是笔记本家目录）。
3. 进程结束，TUI `push_bash_result`（对应 `Event::BashResult`）。`!!` 写盘但不喂给下一轮模型。
4. 跑的时候按 Esc：TUI 发 `Abort`，server 杀掉这棵 bash；agent 回合（如果有）另算。

### 现状缺口：bang 直播（live chunks）

进程内：`chunk_tx: mpsc::Sender<Vec<u8>>`，TUI 边收边画。`XyRemoteDriver::execute_bash` 把 `chunk_tx` 标成 `_`，REST 一问一答——长命令会整段卡住，Esc 取消和直播环对不上。**这是一个 Remote Driver 的缺口，与选哪个 HTTP 框架无关。**

若要让 attach 具备直播，可选方向（候选，采纳与否见提案）：
- 新增一个**专给 bang 的 wire Event**（UI / Esc / 会话条目都与工具不同，不建议复用 `ToolExecutionUpdate`）——方案名意向 `BashDelta` + 已有 `BashResult`；
- 走**同一条 WebSocket 订阅**（journal 或旁路同一连接），TUI 才能在 `select!` 里一边收 chunk 一边收键；
- `Abort` 在 bang 进行中需要能打到 server 那棵进程树（已有 `BangExecHandler` 的 cancel token）——这是该方向的一条硬性要求。

## 路径 3：审批（模型跑工具要你点头）

1. 模型发工具调用，server 认为要人批。
2. server 不直接跑，登记一个 `call_id`，经 WebSocket 做反向 RPC（`ReverseRpcGateway::register`，`ws.rs`）。
3. TUI 弹选项；你选是/否。TUI 发 `{"type":"approve_tool","call_id":"...","approved":true}`（`Command::ApproveTool`）。
4. **第一个回答算数**，晚到的丢掉；server 才 spawn 工具，然后仍走 `tool_start` → 输出 → `tool_end`。
5. 卡住时的兜底：默认等 **60 秒**（`REVERSE_RPC_TIMEOUT`），超时当拒绝/超时结果，agent 那一轮看见失败而不是永久挂起。

问卷（`Command::AnswerQuestion`）同一套网关，回的是字符串不是 bool。

「拖死」还有别的原因，都不在 HTTP 栈层面：工具自己的超时（bash 工具注释里默认可 unlimited、上限 120s）、你 abort 整轮、journal 满了要求 `ResyncRequired`。
