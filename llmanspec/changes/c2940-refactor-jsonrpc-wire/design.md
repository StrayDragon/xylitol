# 设计：产品线协议 JSON-RPC 2.0（JSON-RPC 2.0，线协议）

## 1. 决策总览

- 线形状：JSON-RPC 2.0 文本（request / notification / response / batch）。载体仍为 salvo：`POST /rpc`（unary）+ `WS /rpc`（下行与双向 text 帧）+ `GET /healthz`。
- 唯一映射：`XyEvent ↔ protocol::Event` 只在新 crate `xylitol-wire` 实现；`serve` 与 `app-tui` 不再各持一份。
- 方法寻址：按方法名（string）寻址，删除数字 `method_id`、`Method` 枚举、`METHOD_NAMES`、`method_name()`。
- 版本标识：`PROTOCOL_VERSION: u32 = 4`（仅用于 `host.describe` 协商）；`WIRE_FORMAT_JSONRPC: &str = "jsonrpc"`。无 `V4`/`v3` 类型名。
- 一步到位：所有端同批切换；无双轨、无兼容 shim、无别名（如 `new_v3`）。

## 2. 消息形状

### 2.1 request（有 id，有应答）

```json
{ "jsonrpc": "2.0", "id": "1", "method": "host.describe", "params": {} }
```

- `id`：JSON string 或 integer（i64）；服务端**原样回显**到 response。`null`、小数、对象、数组 `id` 判 `-32600`。
- 内部幂等键（idempotency key）与 pending 表键统一用 `id` 的字符串形式（`Integer(n)` → `n.to_string()`）。删除 `stable_rpc_id` 哈希。
- `params`：object 或 array；缺省视为 `{}`；标量判 `-32600`。**产品方法只接受 object**：array 判 `-32602`。
- `writer_token`：放 `params.writer_token`（可选，服务端忽略，与现状一致：服务端从不签发）。不作为顶层字段。`x-writer-token` header 本次不做。
- `seq`：只出现在 notification 的 `params.seq`（journal 序号），不是 JSON-RPC `id`。

### 2.2 notification（无 id，无应答）

下行（服务端 → 客户端）：`{ "jsonrpc": "2.0", "method": "session/event", "params": { "session_id": "s0", "seq": 7, "event": { "type": "text_delta", "text": "hi" } } }`

上行（客户端 → 服务端）：产品方法均为 request；客户端发来的 notification **不执行、不应答**（服务端忽略）。

### 2.3 response

```json
{ "jsonrpc": "2.0", "id": "1", "result": { "protocol": 4, "formats": ["jsonrpc"] } }
{ "jsonrpc": "2.0", "id": "1", "error": { "code": -32601, "message": "Method not found", "data": { "code": "unregistered_method" } } }
```

- 解析失败、无法取得 `id` 的错误应答，`id` 为 `null`。

### 2.4 batch

- 顶层数组：逐条处理（**按序执行**），应答数组中只包含 request 的 response，notification 不产生应答。
- 空数组 `[]` 判 `-32600`（HTTP 400）。
- 全部为 notification 的 batch：HTTP 200，无 body。

## 3. 错误码与 HTTP 状态

### 3.1 JSON-RPC 标准码

| 场景 | `error.code` | `error.data.code` | HTTP |
|---|---|---|---|
| 非法 JSON / 深度超 2048 / 非 UTF-8 | `-32700` | `parse_error` | 400 |
| 缺 `jsonrpc:"2.0"`、缺 `method`、`id` 类型非法、标量顶层、空 batch | `-32600` | `invalid_request` | 400 |
| body 超过 4 MiB | `-32600` | `payload_too_large` | 413 |
| 未登记方法（含 `plugin/*` 预留命名空间） | `-32601` | `unregistered_method` | 200（batch 内） / 200 |
| `params` 形状或字段错误（如 array params、缺 `model_id`） | `-32602` | `invalid_input` | 200 |
| 服务端内部错误 | `-32603` | `internal_error` | 200 |

说明：「HTTP 200」指合法 JSON-RPC 的应答（含业务错误）。单个非法请求（非 batch）按 400；batch 内的单条非法项进入数组应答，整体 200。

### 3.2 业务错误

- 业务错误统一 `error.code = -32000`（`BUSINESS_ERROR`），`error.message = details`，`error.data.code = <稳定字符串>`。
- 稳定字符串沿用现有 `RpcResult` 业务码：`not_found`、`unavailable`、`idempotency_conflict`、`session_busy`、`writer_conflict` 等；`invalid_input`、`internal_error`、`unregistered_method` 由映射表转为 `-32602` / `-32603` / `-32601`。
- 映射函数只在 `xylitol-protocol::codec` 一处（`business_error(&RpcError) -> ErrorObject`）。

### 3.3 `GET /healthz`

保持现状（JSON：`pid` / `version` / `status`；starting / stopping 返回 503，starting 带 `retry-after`）。

## 4. 传输

- `POST /rpc`：请求体读取上限 `MAX_BODY_BYTES = 4 MiB`（`Request::payload_with_max_size`）；超限 413 + `payload_too_large`。
- 深度守卫：`json_nesting_exceeds_limit`（字符扫描，跳过字符串内字符）+ `serde_json` 的 `disable_recursion_limit`；超过 `JSON_DEPTH_LIMIT = 2048` 判 `-32700`。
- 响应 `Content-Type: application/json`；请求 Content-Type 不做强校验（body 即判定依据）。
- `WS /rpc`：仅接受 text 帧；text 帧解析失败回 `-32700` text 帧且连接保持；**binary 帧**以关闭帧 `1003`（unsupported data）断开，不执行业务；ping / pong / close 允许。WS `max_message_size` 设为 4 MiB。
- 不暴露 `/openapi.json`、`/docs`、`/api/*`、SSE。

## 5. 方法表

### 5.1 unary（request → result）

方法名与语义以 `crates/xylitol-protocol/src/registry.rs` 的 `REGISTRY` 为 SSOT（38 项）：

- 会话与模型：`prompt`、`abort`、`steer`、`follow_up`、`clear_queue`、`bash`、`compact`、`set_model`、`cycle_model`、`get_available_models`、`set_thinking_level`、`get_state`、`get_session_stats`、`get_messages`、`estimate_context`、`get_commands`、`queue_stats`。
- 会话生命周期：`new_session`、`switch_session`、`fork`、`list_sessions`、`load_session_entries`、`delete_session`、`get_session_name`、`set_session_name`、`set_session_name_for`、`export_html`、`export_jsonl`、`import_jsonl`、`session_tree`、`travel_session_tree`、`append_entry_label`。
- Host：`host.describe`（Bypass）、`reload`、`loaded_resources`、`subscribe`、`arm_tool_freeze`、`persist_trust`。

反向应答（不在 REGISTRY，由 `rpc_module` 直接处理，返回 `result: {}`）：`approve_tool`（`{call_id, approved}`）、`answer_question`（`{call_id, answer}`）。首次应答生效（r1706）。

面本地（不进协议）：`quit`（与 `quit_is_not_unary` 一致，r1702）。

命名空间预留：`plugin/*`、`plugin/call` 只作为预留前缀，**当前未登记**，调用返回 `-32601`。插件 ABI 不在本次范围，不为插件扩 `Event` 闭集。

### 5.2 下行 notification

| method | params | seq | 说明 |
|---|---|---|---|
| `session/event` | `{session_id, seq, event}` | 消耗 journal seq | 事件；每会话单调；journal 环形重放 |
| `session/subscribed` | `{session_id, seq, inflight}` | 不消耗 | 订阅确认（与 `subscribe` 结果同形） |
| `session/resync_required` | `{session_id}` | 不消耗 | journal 无法覆盖 `last_seq` |
| `session/resources` | `{session_id, snapshot}` | 不消耗、不写 journal | MCP / skills 固定区快照 |
| `approval/requested` | `{call_id}` | 不消耗 | 审批；客户端以 `approve_tool` unary 应答 |
| `question/requested` | `{call_id}` | 不消耗 | 问卷；客户端以 `answer_question` unary 应答 |
| `bash/output` | `{session_id, bash_id, seq, data}` | 不消耗 journal | bang 输出流（原 `session/bash_output`） |

下行 notification 无作答义务；客户端不得对事件 notification 回应。

## 6. 事件映射策略

### 6.1 唯一映射

- 位置：`crates/xylitol-wire/src/event.rs`。
- 接口：`event_to_wire(&XyEvent) -> Result<protocol::Event, WireEventError>` 与 `event_from_wire(&protocol::Event) -> Result<XyEvent, WireEventError>`。
- 实现：逐变体、逐字段显式映射（不用 serde 往返，不 `expect`）；`to_wire` 对 `XyEvent` 全部 23 个变体**无 catch-all**，编译器强制覆盖。
- 错误：仅动态载荷（`AgentMessage` 的 `serde_json` 序列化 / 反序列化）可能失败，返回 `WireEventError::Payload`；调用方记录 `log::error!` 后跳过该事件（不静默）。`from_wire` 遇到 `Event::Unknown` 返回 `WireEventError::Unknown`，调用方 `log::warn!` 并跳过（前向兼容降级）。

### 6.2 变体对照（23 项 + Unknown）

| `XyEvent` | wire `type` | 载荷字段 |
|---|---|---|
| `AgentStart { session_id, model }` | `agent_start` | `session_id`, `model` |
| `AgentEnd { messages }` | `agent_end` | 无字段（`messages` 线上不携带，见 6.3） |
| `TurnStart { turn_index }` | `turn_start` | `turn_index` |
| `TurnEnd { turn_index }` | `turn_end` | `turn_index` |
| `MessageStart { role, message }` | `message_start` | `role`, `message?`（动态 JSON） |
| `MessageUpdate { text, thinking, message }` | `message_update` | `text`, `thinking?`, `message?` |
| `MessageEnd { role, message }` | `message_end` | `role`, `message?` |
| `TextDelta(String)` | `text_delta` | `text` |
| `ThinkingDelta(String)` | `thinking_delta` | `text` |
| `ToolExecutionStart { id, name, args }` | `tool_execution_start` | `id`, `name`, `args`（动态 JSON 原文） |
| `ToolExecutionUpdate { id, output }` | `tool_execution_update` | `id`, `output` |
| `ToolExecutionEnd { id, name, result, is_error }` | `tool_execution_end` | `id`, `name`, `result`, `is_error` |
| `CompactionStart { reason }` | `compaction_start` | `reason` |
| `CompactionEnd { ×9 }` | `compaction_end` | `result?`, `aborted`, `reason`, `will_retry`, `error_message?`, `summary?`, `tokens_before?`, `tokens_after?`, `notice?` |
| `ContextTokenSettlement { estimate, reason, generation }` | `context_token_settlement` | `tokens`, `provenance`, `usage_tokens`, `trailing_tokens`, `last_usage_index?`, `reason`, `generation` |
| `ModelSelect { provider, model_id }` | `model_select` | `provider`, `model_id` |
| `ThinkingLevelChanged { level }` | `thinking_level_changed` | `level` |
| `QueueUpdate { steer_count, follow_up_count }` | `queue_update` | `steer_count`, `follow_up_count` |
| `TodoUpdated { list }` | `todo_updated` | `list`（`items[{id, content, status}]`，逐字段映射） |
| `AutoRetryStart { attempt, max_retries, delay_ms }` | `auto_retry_start` | `attempt`, `max_retries`, `delay_ms` |
| `AutoRetryEnd { success, attempt }` | `auto_retry_end` | `success`, `attempt` |
| `Error(XyEventError { kind, message })` | `error` | `kind`, `message` |
| `SessionInfoChanged { key, value }` | `session_info_changed` | `key`, `value`（动态 JSON） |
| — | `unknown` | wire 侧前向兼容 catch-all（`#[serde(other)]`） |

命名说明：旧 wire 名 `tool_start` / `tool_end` 统一为 `tool_execution_start` / `tool_execution_end`，与 `XyEvent` 对齐；一步到位，不保留旧名。

### 6.3 `AgentEnd.messages` 决策

线上 `agent_end` 不携带 `messages`：当前所有消费者只匹配 `AgentEnd { .. }`，transcript 通过 `get_messages` 重建。`from_wire` 得到 `messages: Vec::new()`。此语义写入 `protocol-app` spec（本地保留，显式记录），不是静默丢弃。

### 6.4 动态载荷

工具参数、工具结果、消息内容、`CustomEntry.data`、`Resources.snapshot`、`CompactionEnd` 的 details 等保持 JSON 原文（`serde_json::Value`），不做结构化 schema。

## 7. Crate 与模块布局

### 7.1 依赖图（新增 `xylitol-wire`）

- `xylitol-plugin-sdk`：无变化（词表 + 端口）。
- `xylitol-protocol`：删 `fory`；依赖保持 sdk 无关（不依赖 wire、runtime）。
- `xylitol-wire`（新）：依赖 `xylitol-protocol` + `xylitol-plugin-sdk` + `serde_json` + `thiserror`；不依赖 runtime / serve / app-tui。
- `xylitol-serve`：依赖 `xylitol-runtime` + `xylitol-protocol` + `xylitol-wire`。
- `xylitol-app-tui`：依赖 `xylitol-protocol` + `xylitol-wire` + `xylitol-tui` + sdk。
- `xylitol-kernel`：依赖 `xylitol-protocol`（`Command`）；**不**依赖 wire、serve 的映射。`command_exec.rs` 的 `SessionTreeKind` 转换以本地 `match` 完成。
- `xylitol-app-print`：不编线协议；description 去掉 `fory` 字样。

### 7.2 模块布局

| crate | 文件 | 职责 |
|---|---|---|
| protocol | `message.rs` | `RequestId`、`Request`、`Notification`、`Response`、`Outcome`、`ErrorObject`、`Message`、`RpcResult` / `RpcError`（业务）、`HostDescribeValue`、下行参数结构；`PROTOCOL_VERSION`、`WIRE_FORMAT_JSONRPC` |
| protocol | `codec.rs` | `parse_text` / `parse_payload`（batch）/ `decode_message`（客户端）/ `encode`；深度守卫；错误码与业务码映射 |
| protocol | `command.rs` | 不变：`Command` serde 类型 |
| protocol | `event.rs` | `Event` 24 变体（23 + `Unknown`） |
| protocol | `method.rs` | 下行方法名常量、`is_unary_method`、`is_downlink_method` |
| protocol | `registry.rs` | `REGISTRY`、`lookup`、`parse_command`；删除 `Method` 依赖 |
| protocol | `todo.rs` / `session_tree.rs` | 不变 |
| wire | `event.rs` | `event_to_wire` / `event_from_wire` 与 `WireEventError` |
| wire | `lib.rs` | 再导出 |
| serve | `wire.rs` | `CONTENT_TYPE`、`MAX_BODY_BYTES`、`Reply`、HTTP 状态映射 |
| serve | `listen.rs` | salvo handler：`POST /rpc`、WS、`healthz`、`OccupiedHost` 接口 |
| serve | `occupancy/envelope.rs` | 上行：`parse_payload` → 逐条 dispatch → `Reply`；下行：`downlink_notification` 与 `encode` |
| serve | `occupancy/rpc_module.rs` | `dispatch_request(host, id, method, params) -> Outcome`；反向方法；`handle_unary` 调用；恢复原 `#[cfg(any())]` 测试 |
| serve | `occupancy/ws.rs` | `EventJournal`、`ReverseRpcGateway`（不变）；删除 `ServerFrame` / `ClientFrame` 死枚举 |
| serve | `occupancy/host.rs` | `append_and_push` 经 `event_to_wire`；`push_resources`、`push_bash_output` 等改为 `Notification` |
| app-tui | `host_client/json_rpc.rs` | 客户端 request 文本、downlink 解码、describe 协商校验 |
| app-tui | `host_client/http_ws.rs` | `Outgoing::Text` 唯一；pending 按 id 字符串；`negotiate` 校验 `protocol == 4` 且 `formats ∋ jsonrpc` |
| app-tui | `host_client/in_process.rs` | `subscribe_downlink` 产出 `Notification` |
| app-tui | `driver/remote.rs` | `XyRemoteDriver::new`（删除 `new_v3`）；`event_from_wire` |

删除：`protocol/src/v3/`（`mod.rs`、`generated.rs`、`envelope.fbs`）、`protocol/src/envelope.rs`（内容拆入 `message.rs`）、`serve/src/mapping.rs`、`serve/src/v3_mapping.rs`、`app-tui/src/v3_mapping.rs`、`app-tui/src/wire_event.rs`、`app-tui/src/host_client/envelope_client.rs`、`scripts/gen_envelope.py`。

### 7.3 Host 侧语义保持

- 写者 / 占用 / lease / seq / journal / resync / reverse-rpc 语义不变（只改载体）。
- `subscribe` 仍为 unary request；返回 `{session_id, seq, inflight}`；随后推送 `session/event` 与 `session/subscribed` 通知。
- `idempotency_conflict` 等业务码语义不变，映射为 `-32000` + `data.code`。

## 8. 握手与版本

- `host.describe` 结果：`{ "protocol": 4, "formats": ["jsonrpc"] }`。
- 客户端连接后先发 `host.describe`：`protocol` 不等于 `PROTOCOL_VERSION` → `ProtocolMismatch` 断开；`formats` 不含 `jsonrpc` → 断开。**不得降级**，不读取环境变量开关。
- 字段级演进：JSON 字段增删不 bump `PROTOCOL_VERSION`；未知字段忽略；未知事件 `type` 落 `Unknown`。只有载体或语义不兼容的变化才 bump。

## 9. 命名与门禁

### 9.1 禁止与允许

- 禁止：`fory`、`FORY`、`WIRE_FORMAT_FORY_V3`、`XYLITOL_ENVELOPE_V3`、`XYLITOL_WIRE_V3`、`application/x-fory-v3`、`xy.wire.v3`、`envelope.fbs`、`codegen-envelope`、`gen_envelope`、`foryc`；线协议相关标识符带 `v<digits>` 段（如 `new_v3`、`v3_mapping`、`V4`）。
- 允许：`PROTOCOL_VERSION`；`Message`、`Request`、`Notification`、`Response`、`Error`、`Command`、`Event`、`MethodName`、`Registry`、`Codec`、`Mapping`、`WireFormat`。

### 9.2 新门禁 `scripts/check_no_version_names.py`

- 只读，支持 `--check`（`qa-check-scripts` 通过 glob 自动接线）。
- 扫描范围：`crates/`、`plugins/`、`tests/`、`scripts/`、`src/`、`llmanspec/specs/`、`AGENTS.md`（含子目录 `AGENTS.md`）、`justfile`、`rustfmt.toml`、`.uncomment.toml`、`Cargo.toml`。
- 排除：`llmanspec/changes/archive/`、`llmanspec/delayed-changes/`、`Cargo.lock`、`target/`、`.git/`。
- 规则 A（字面，全部扫描文件）：上列禁止字面量（`fory` 大小写不敏感）。
- 规则 B（标识符，仅 `.rs` / `.py`）：标识符中以下划线分隔的 `v<digits>` 或 CamelCase 的 `V<digits>` 段；白名单：`new_v4`、`new_v7`（`uuid` crate API）。
- 不扫描：外部模型名（如 `deepseek-v4-flash`）、URL 路径（`/v1`）、`uuid` 的 cargo feature 字面量、会话落盘格式版本（`v5`/`v6`/`v7`）、provider trace schema 名。这些不是线协议命名，改动会破坏外部契约或用户数据。

### 9.3 DoD 字面清扫的诚实说明

按 prompt 的 `rg` 字面清扫（含 `\bv[0-9]+\b`）会命中上述非线协议内容（约 2000 行，多数为会话版本、模型名与 URL）。门禁按 9.2 收窄；字面清扫的剩余命中在最终报告中逐类列出并说明保留理由。

## 10. 测试策略

- `xylitol-protocol`（单元）：`parse_payload` 的 request / notification / batch / 空 batch / 非法 id / 非法 method / 深度 2048 边界 / 非 UTF-8；`encode` 形状；`RequestId` 原样回显；`business_error` 码映射表全覆盖；`Event` 24 变体 serde 往返与 `Unknown` 降级；`REGISTRY` 保留现有测试。
- `xylitol-wire`（单元）：23 个 `XyEvent` 变体逐一往返（表驱动）；`AgentEnd` 的 `messages` 本地语义断言；`Unknown` → `WireEventError::Unknown`；`TodoUpdated` 逐字段。
- `xylitol-serve`：`rpc_module`（恢复 `#[cfg(any())]` 测试并改写）：未登记 `-32601`、参数形状 `-32602`、reverse 方法、`subscribe` 续传；`envelope`：batch 按序、notification 无应答、`[]` → 400；`listen`：`POST` 413 / 400 / 200、WS text 往返、WS binary 关闭 1003。
- `xylitol-app-tui`：`json_rpc.rs` 请求文本与 downlink 解码；describe 校验（protocol 与 formats 两条不匹配路径）；`http_ws.rs` 仅 text。
- BDD：`tests/bdd/bindings_jsonrpc.rs` + `steps_jsonrpc.rs` 覆盖 prompt 草稿中的 6.1–6.5 场景；`steps_server.rs` / `steps_protocol.rs` / `steps_remote_resilience.rs` / `steps_hooks.rs` / `bindings_workspace_assembly.rs` 等改为 JSON 文本断言。场景名与步骤文本逐字一致。
- 门禁：`just fmt`、`just lint`、`just test`、`just qa`；`llman-sdd validate c2940-refactor-jsonrpc-wire --strict`；`cargo tree -i fory` 为空。

## 11. 不做的事

- 不引入 `jsonrpsee`、`axum`、`tracing`（保持 fastrace + log）。
- 不实现插件 ABI；不为插件扩 `Event` 闭集（`plugin/*` 仅预留前缀）。
- 不做 OpenAPI / docs / SSE。
- 不改 `llmanspec/changes/archive/`、`llmanspec/delayed-changes/` 的历史记录。
- 不做 `x-writer-token` header 映射。
- 不改外部模型名、provider URL、会话落盘版本号、cargo feature 名。

## 12. 风险与缓解

- BDD 绑定漂移：场景名 / 步骤文本逐字一致；`llman-sdd validate --strict` 与 `just test` 双验证。
- 大爆炸切换：所有端同批改；`cargo check --workspace` 驱动的编译错误清单作为进度依据。
- `AgentEnd.messages` 丢失：spec 显式写明；`get_messages` 为 transcript 唯一重建来源。
- WS 与 HTTP 对同一 `id` 并发：pending 表按字符串 id 去重；重复 id 由客户端保证唯一（uuid）。
