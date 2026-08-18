# 实现票草稿：bang 直播 Event `BashDelta`

> **不是**本票 apply，**不改** live `llmanspec/specs/**`。给后续实现 change 当任务切片。c2280 只调研。
> 已钉：tagged JSON；HTTP 栈 axum；`BashDelta` 专给 bang，不复用 `ToolExecutionUpdate`。

## 为什么

进程内 TUI：`execute_bash(..., Some(chunk_tx))`，`run_interactive_bang` 边收字节边 `append_bash_chunk`。

`XyRemoteDriver::execute_bash` 丢掉 `chunk_tx`，REST 等跑完才返回。`--attach` 下长命令会卡住，Esc 和直播环也对不上。

## 产品行为（以后 landing spec 时写 MUST）

- attach 下 `!` / `!!` 在 **server 工作区**执行。
- stdout/stderr 增量作为 Event 推到同一条 WebSocket 订阅，TUI 画法和进程内 bang 区一致。
- 结束仍是 `Event::BashResult`（已有）。
- Esc → `Command::Abort` 杀掉这棵 bang 进程树，不是 agent Aborted 文案。
- 模型 bash 工具继续走 `ToolStart` / `ToolExecutionUpdate` / `ToolEnd`。两条 API。

## 代码切口（实现时对着改）

1. `src/protocol/wire/event.rs`：增加 `BashDelta { text: String }` 或 `{ data: bytes 的 utf8 有损 }`。进程内 chunk 是 `Vec<u8>`，wire 先定 UTF-8 文本（非法字节用现有 bash 工具那套替换策略，实现时对照 `infra/tools/bash.rs`）。
2. `XyEvent`（lifecycle）加对应变体；`to_wire_event` / `try_from` 往返。
3. journal：bang 进行中 `append` 每条 `BashDelta`，重连可 replay（会很长——实现时考虑是否只 replay 未完成 bang，或容量内原样）。
4. server：`BangExecHandler::execute` 的 `chunk_tx` 接到「写 journal + 广播 WebSocket」，不要只走 REST 返回值。
5. `XyRemoteDriver`：attach TUI 不再用 REST bash 当产品路径；订阅 Event，本地 `append_bash_chunk`。REST bash 可留给 Print / 调试。
6. `effects/bang.rs`：Remote 时 `chunk_rx` 改从 Event 流来，或统一成「只听 Event」。
7. harness：现有 bang 测参数化 InProcess | 测试 host+Remote，至少一条「多 chunk 再结束」。

## 非目标

- 不换 HTTP 栈、不换 JSON-RPC。
- 不在本草稿设计多 server 编排。
- 不把 Docker 当成 `BashDelta` 的前置；本机 serve 也要直播。
