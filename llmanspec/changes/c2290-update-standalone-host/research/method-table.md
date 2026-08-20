# 方法表草案（Command / Event → 四象限）

> c2290 规划壳。不是 live spec。落地时方法名以此为准；缺口标「保留」者不进 v1 map（未知方法信封解析失败，对齐 DSH reserved-method 纪律）。

原则：

1. **不另写 DTO**。unary payload = 现行 `Command` 变体字段（去掉线层 `id`，相关改走信封 `rpcId`）。
2. **下行不按 Event 变体爆炸成方法**。多数生命周期进一帧 `session/event`，payload 仍是 `protocol::Event`。
3. **审批/问卷不是 unary**。host 下行可应答 `ServerRequest`；client `POST /api/respond`。
4. **`Quit` 不进 Host**。关 TUI 不杀 Host。
5. 方法字符串用 **snake_case，对齐现行 Command serde tag**（Pre-0.0.1 一次切，不搞 JSON-RPC 2.0 / REST 双名）。

## Unary（ClientRequest → ServerResponse）

| method | 现行 Command | payload 要点 | 返回（result.value） |
|---|---|---|---|
| `prompt` | `Prompt` | `session_id?`, `message` | 入队回执（可空 object）；token 走下行 |
| `abort` | `Abort` | `session_id?` | `{}` |
| `get_state` | `GetState` | `session_id?` | 现行 state 快照 |
| `set_model` | `SetModel` | `provider`, `model_id` | 选中模型 |
| `cycle_model` | `CycleModel` | — | 选中模型 |
| `get_available_models` | `GetAvailableModels` | — | 列表 |
| `set_thinking_level` | `SetThinkingLevel` | `level` | `{}` |
| `bash` | `Bash` | `command`, `exclude_from_context` | 结束态可在 result；直播若未进 Event 则仍结束态（c2301 缺口） |
| `compact` | `Compact` | `instructions?` | `{}`；进度走下行 |
| `get_session_stats` | `GetSessionStats` | `session_id?` | stats |
| `export_html` | `ExportHtml` | **禁止只交本机路径当唯一结果**；host 序列化，client 写盘（c2300）。v1 可先回内容字节/字符串 | 导出内容 |
| `export_jsonl` | `ExportJsonl` | 同上 | 导出内容 |
| `import_jsonl` | `ImportJsonl` | 内容或 client 已读的文本；**不要**强迫 host 读 TUI 本机 path | `{}` / session id |
| `switch_session` | `SwitchSession` | `session_id` 或 path（收口为 id） | `{}` |
| `fork` | `Fork` | `entry_id`, `position?` | 新 session id |
| `get_messages` | `GetMessages` | `session_id?` | 已加载条目 |
| `get_commands` | `GetCommands` | `session_id?` | slash 表 |
| `steer` | `Steer` | `message` | `{}`；`QueueUpdate` 下行 |
| `follow_up` | `FollowUp` | `message` | `{}` |
| `clear_queue` | `ClearQueue` | `clear_steer`, `clear_follow_up` | `{}` |
| `subscribe` | `Subscribe` | `session_id`, `last_seq` | subscribed 快照（`seq`）；后续增量走 mux |
| `host.describe` | （握手，无 Command） | — | `{ protocol: u32 }`；对不上断开 |

`session_id` 缺省：该连接/进程的当前写者 session（产品 TUI 一窗一 session）。多 session 由 c2302 注册表约束。

## 非 unary

| 现行 | 四象限落点 |
|---|---|
| `ApproveTool` / `AnswerQuestion` | `ClientResponse`（`POST /api/respond`），回显 host 可应答帧的 `rpcId` |
| `Quit` | 面本地。MUST NOT 停 Host |
| WS `Ping` | 载体心跳（WS ping），不进方法表 |

## 下行 ServerRequest（mux）

| method | 可应答 | payload | 现行 |
|---|---|---|---|
| `session/event` | 否 | `{ session_id, seq, event: Event }` | `ServerFrame::Event` |
| `session/subscribed` | 否 | `{ session_id, seq }` | `Subscribed` + `Ack` |
| `session/resync_required` | 否 | `{ session_id }` | `ResyncRequired` |
| `approval/requested` | 是 | `{ call_id, … }` 稳定 rpcId | `ReverseRpc` 审批 |
| `question/requested` | 是 | `{ call_id, … }` 稳定 rpcId | `ReverseRpc` 问卷 |
| `host/hello` | 否 | `{ protocol }` | `ServerHello`（也可只放 `host.describe` unary） |

`Event` 变体（`text_delta` / `tool_start` / `queue_update` / …）**留在 payload 内**，specta 导出整个 `Event` union。不要为每个 delta 开一个 RPC method。

## 保留（v1 map 不登记；实现前当未知方法失败）

c2301 缺口：`reload`（MCP/prompt/技能）、trust 持久化、写者/只读状态帧、bash 直播增量。
**禁止**抄 DSH 全表（`workspace.*` / `settings.*` / `agentPreset.*` / `skill.list`）——产品面没有就不要空转。需要时新 change 往 map 加一行。

## 错误

`ServerResponse.result` = `{ ok: true, value }` \| `{ ok: false, error: { code, details } }`。
`code` 从 `XyDriverError.kind`（及既有 `ErrorCode`）映射。HTTP 200 = 信封合法。非法信封才 400。废弃 REST `Envelope` 作为产品路径。

## specta 登记集

必须 `Types::register`：四象限四成员、`RpcResult`/`RpcError`、上表每个 unary payload/value、`Event`、下行帧 payload。导出一个 `bindings.ts`。方法名表用 const + 类型别名导出，便于 TS 薄客户端。
