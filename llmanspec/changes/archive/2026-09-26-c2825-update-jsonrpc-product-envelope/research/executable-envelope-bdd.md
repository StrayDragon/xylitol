# c2825 信封 BDD：少 @human、尽量 @executable

> 探索笔记。不是实现。ACP 不在范围。对准 [design.md](../design.md) 的 jsonrpsee 终态，不是四象限字段改名。

## 现状问题

已决的 @human 已改口对准 jsonrpsee 终态。**本拍已把信封相关 @executable 改成真实 HTTP 步骤**（POST /rpc、JSON-RPC 形状、`X-Writer-Token`、approve_tool unary）。仍走旧 mux IR 的订阅/hello 场景（ath44 `server_hello`）留到翻客户端。

| 场景 | 现在测的 | 与新 @human 冲突 |
|---|---|---|
| `four-quadrant-envelope-shape` | `type=client-request` / `rpcId` | r1709 仍写四向；终态应改口 |
| `unary-stable-error-envelope` | body **不含** `jsonrpc`，顶层 `ok` | r1696 要求 JSON-RPC `error` + `data.code` |
| `frame-serialize` | JSON 含 `type=server-request` | 终态是 notification / unary，不是 server-request tag |
| `run-endpoint-works` | `ServerResponse` 回显 `rpcId` | 应断言 `jsonrpc` + `id` |
| `product-path-four-quadrant` | 「经四象限 POST」 | 通道词错 |
| `reverse-rpc-first-answer-effective` | `POST /api/respond` | 终态是 `approve_tool` unary |

这些不是「缺 @human」，是 **验收步与规则对不上**。本 change 应补/改 **可执行场景**，不要再堆只读人工句。

`protocol-app` 里与信封无关的 @human（Command 闭集、AgentPart JSONL、面本地禁入协议）**本票不改**。

## 本票应有的 executable（jsonrpsee 终态）

接缝：真 HTTP/WS Host + 产品 encoder（与现 `steps_server` 同款）。每条都能 rust 跑。未知方法不再测「URL 404 vs 信封」两套——单一 RPC 入口下就是 `-32601`。

### A. unary 形状（r1696 / r1701）

1. **jsonrpc-unary-success-shape**：`host.describe` POST 合法 JSON-RPC request → HTTP 200；body 含 `"jsonrpc":"2.0"`、`id` 回显、`result` 存在、**无**顶层 `ok`、**无** `"type":"server-response"`。
2. **jsonrpc-unary-app-error-shape**：已登记方法、业务失败（未知 session / 租约冲突）→ HTTP 200；`error` 对象；`error.data.code` 为字符串；测试 **不**把数字 `error.code` 当产品码。
3. **jsonrpc-illegal-envelope**：body `{}` 或旧四象限信封 → parse 失败（JSON-RPC 信封错 `-32700`/`-32600`，或 HTTP 4xx）；不是带 `result` 的成功。
4. **jsonrpc-id-echo**：request `id` 为字符串 `"R"` → response `id` 同值。
5. **jsonrpc-unknown-method**：`method: "no_such_method"` → JSON-RPC `-32601`；`error.data` 若有产品码也不得冒充已登记方法。

删除「body.method 必须等于 URL `{method}`」——真源不再把方法放进路径。

### B. 写者租约在横切（r1793）

6. **writer-token-via-header**：首次 Writer 方法成功响应带 `X-Writer-Token`；第二次请求带同一 header 成功；缺 token 的并发写者 → HTTP 200 + `error.data.code` 为 `writer_conflict`（今日业务码）；body 的 `params`/`result` **不含** `writerToken`。只读方法响应 **不**带该 header。

现有 `writer-lease` 只断言「业务错误说明已有写者」，应扩一步盯 **请求/响应 header**，body 不得再出现 `writerToken`。

### C. 订阅下行（r1704）

7. **session-subscribe-notifications**：`session_subscribe` 成功后，事件帧为 JSON-RPC **notification**（有 `method`+`params`，**无** `id`）；`params` 含既有 Event；**无** `type=server-request`。
8. **hello-version-mismatch-fatal**：`host.describe` / `initialize` 的协议版本不等于客户端期望 → 断开且不重试（ath44 已有，改断言方法 result 而非 `ServerHello` tag）。

### D. 审批是 unary（改口 r1700 / r1706 / r1713）

9. **approve-tool-is-unary**：下行 `approval/requested` **notification**；客户端 `approve_tool` unary 后回合恢复；同一 call 第二次 unary 被忽略（r1706）。
10. **ws-jsonrpc-peer-allows-unary**：WS 上发合法 JSON-RPC unary **不得**仅因「业务上行」被丢掉——丢掉的是非 JSON-RPC 应用帧。

现有 `approval-roundtrip` 只测「respond 后回合恢复」，改形状断言即可，不必新能力。

### E. 调试文档（r1784）

11. **openapi-envelope-is-jsonrpc**：`GET /openapi.json` 描述 JSON-RPC 信封（`jsonrpc` const），**仍无** per-method payload schema。现有 oapi 单测可升格。

## 明确不要做成 @human 新句的

- 「内部 IR 仍是 RpcMessage」——代码组织。
- 「双读窗口」——进 `tasks.md`。
- 「用了 jsonrpsee crate」——代码组织；spec 只写线上 JSON-RPC 形状。
- ACP / Fory / gRPC。

## 须改口的既有 @human（产品 WHAT）

若采纳 jsonrpsee 终态，下列规则与验收要一起改，而不是再加一层人句：

- r1700 / r1713：审批问卷 **改为** 产品 unary，撤回「MUST NOT 当作 unary」。
- r1709：撤回「WS MUST NOT 收业务上行」；改为「WS 只承载 JSON-RPC 帧」。
- r1704：握手改为 `host.describe` / `initialize` result，不再依赖 `ServerHello` 特例帧。

## 仍可留 @human 的（本票）

- JSON-RPC 数字码「仅载体」：场景 2 锁定，人句留一句原则。
- Command/Event 闭集、面本地禁入协议：与信封无关。

## 已决（2026-09-26）

Live spec 只写终态。双读旧四象限只在 tasks，不写 `@executable`「必须同时接受两种信封」。
