# Design: 产品 RPC 运行时（终态）

**已决（2026-09-26）**：产品 RPC = jsonrpsee，入口 **`POST /rpc` + `WS /rpc`**；写者租约 HTTP 走 header **`X-Writer-Token`**（请求与响应），WS 应答把同一令牌放在 JSON-RPC **对象顶层** `writerToken`（extra member），**不**进 `params` / `result`；Salvo 留下 HTTP 周边；ACP 不做。

ACP **不在本票**（`c2305` 保持搁置）。词表仍是 Command / Event / `REGISTRY`。

被否决：把 `RpcMessage` 改名塞进 JSON-RPC（`params.payload`、`params.writerToken`、mux 上假 request、`POST /api/respond`）。那是两套信封，横切能力每加一次就在两层各开一个洞。证据：[research/mainstream-rpc.md](./research/mainstream-rpc.md)。

## 三层（可替换的才叫框架）

```text
① 词表     Command / Event / REGISTRY（Auth / Idem / Resp / Exec）
② RPC 运行时  方法注册、id、error、notification、subscription、中间件
③ 传输     HTTP 与 WebSocket（由 ② 提供；调试 GET 可仍走现有 HTTP 栈）
```

① 是产品闭集，不随 RPC 库换。② 今日是自研四象限 + Salvo 路由；终态换成 **jsonrpsee `RpcModule`**（Parity JSON-RPC 2.0，HTTP + WS，官方 client，`RpcServiceBuilder` 中间件）。③ 不再自研 `type: client-request` 与 `/api/respond`。

gRPC / tonic 可另开程序脸，**不是** TUI 真源：调试与 agent 生态是 JSON-RPC 族。

## 扩展点：横切不进业务 params

| 能力 | 放哪 | 不放哪 |
|---|---|---|
| 写者租约 | HTTP：请求与响应 header `X-Writer-Token`。WS：升级请求可带同一 header；每条 unary 应答把令牌放在 JSON-RPC **对象顶层** `writerToken`（规范允许的 extra member）；同一条连接内后续 unary 用连接本地租约。`REGISTRY.auth` 决定要不要；CORS 日后 `Access-Control-Expose-Headers` | `params` / `result` 里的 `writerToken` |
| 幂等 | middleware 认 JSON-RPC `id`（已有 `Idem::PerRpc`） | 自研 `rpcId` 字段平行于 `id` |
| 协议版本 | `host.describe` / `initialize` 的 `result` | 特例 `ServerHello` 首帧 |
| 事件流 | jsonrpsee **subscription** → 下行 **notification**（无 id，不必 respond） | mux 上带 `id` 的假 server-request |
| 审批 / 问卷 | 下行 notification 告知；客户端 **unary** `approve_tool` / `answer_question` | `POST /api/respond` 假 reverse RPC |
| 未知方法 | JSON-RPC `-32601` | URL 404 与信封数字码两套语义 |

`params` 只承载该方法的 Command 字段。没有 `payload` 袋子。

jsonrpsee 已支持：HTTP headers → 每连接 `RpcServiceBuilder` → `Request::extensions`（见 [paritytech/jsonrpsee#1370](https://github.com/paritytech/jsonrpsee/issues/1370) 与 `jsonrpsee_as_service` 例）。租约中间件按 `REGISTRY.auth` 拒绝或放行，业务 handler 看不到 token 字段。

## 线上形状（产品可观察）

同一 `RpcModule` 挂 **`POST /rpc` 与 `WS /rpc`**。`/healthz`、`/docs`、日后 `/` 静态页走 Salvo，不抢 RPC。`POST /api/{method}` 不再是真源（人手调试别名未决，默认不留）。

**Unary（HTTP 或 WS）：**

```json
{"jsonrpc":"2.0","id":"R1","method":"prompt","params":{"sessionId":"s","text":"hi"}}
{"jsonrpc":"2.0","id":"R1","result":{"ok":true}}
{"jsonrpc":"2.0","id":"R1","error":{"code":-32000,"message":"…","data":{"code":"writer_taken"}}}
```

WS 没有逐帧 HTTP 响应头，成功/失败 unary 的 JSON-RPC **对象**上额外带顶层 `writerToken`（与 `result` 平级，不是 result 字段）：

```json
{"jsonrpc":"2.0","id":"R1","result":{"ok":true},"writerToken":"…"}
```

`error.data.code` 仍是产品字符串；JSON-RPC 数字码仅载体。

**订阅（替代今日 unary `subscribe` + 只下行 mux）：**

```json
{"jsonrpc":"2.0","id":"S1","method":"session_subscribe","params":{"sessionId":"s","lastSeq":12}}
{"jsonrpc":"2.0","method":"session/event","params":{ /* Event */ }}
```

notification 无 `id`。客户端不必、也不得对事件帧 `respond`。

**审批（撤回「禁止登记为 unary」）：**

```json
{"jsonrpc":"2.0","method":"approval/requested","params":{ /* ApprovalRequired */ }}
{"jsonrpc":"2.0","id":"A1","method":"approve_tool","params":{"id":"call-1","decision":"allow"}}
```

同一 call 的第一 unary 生效、后续忽略（今日 r1706 语义保留，通道换成普通方法）。

**握手：** 普通 `host.describe`（或 `initialize`）返回协议版本；对不上则客户端断开且不重试（ath44）。不必 `ServerHello` 特例帧。

## 通道纪律（相对今日的取舍）

今日：「WS 禁业务上行 + 审批走另一条 HTTP」是为了不发明全双工应用协议。换成 jsonrpsee 后，WS 上只有 **标准 JSON-RPC 帧**（含客户端 unary）。这不是 REST 全双工，是主流 JSON-RPC peer。

代价：撤回 r1700 / r1713「审批不得 unary」、r1709「mux 禁业务上行」。换来的是：通用 client 能连、横切进中间件、不再维护 `/api/respond`。

Salvo 可继续托管 `GET /openapi.json` / `GET /docs`；jsonrpsee 以 tower service 挂在 RPC 入口（官方 `jsonrpsee_as_service` 路径），不必一次性拆掉调试 HTTP 栈。

### Salvo 还干什么（不是被消灭）

jsonrpsee 吃掉的是 **产品 RPC**（今日 `POST /api/{method}`、`/api/respond`、`/api/events.mux`）。Salvo（或日后 axum）留下的是 **HTTP 周边**：

| 今日 / 日后 | 谁管 |
|---|---|
| 产品 unary / 订阅 / 审批 | jsonrpsee `RpcModule` |
| `GET /healthz`、就绪探针 | HTTP 应用栈 |
| `GET /openapi.json` / Scalar `/docs` | HTTP 应用栈（信封级调试，不是客户端真源） |
| 未来 Web 端：静态资源、HTML 壳、CORS、cookie、OAuth 回调、导出文件下载 | HTTP 应用栈 |
| 未来 Web 端：跟 TUI 同一套 Command/Event | **还是 jsonrpsee**（浏览器走 HTTP/WS JSON-RPC，不必再开 REST 词表） |

Web 端若出现，正确切法是「页面用 HTTP 框架、agent 用同一 RPC 模块」，不是把 Command 再包一层 REST。Salvo 有场景，但场景不是产品协议。

## HostClient 端口（行为网，不钉类型名进 spec）

产品客户端仍是「发方法、收结果、订事件」：

| 今日端口 | 终态 |
|---|---|
| `unary(method, payload)` | jsonrpsee `request(method, params)`；HTTP 或 WS 同一模块 |
| `unary_with_id` | 同一 `id`（幂等键） |
| `respond(rpc_id, payload)` | **删除**；改为 `approve_tool` / `answer_question` unary |
| `mux()` 只下行 | `session_subscribe`；事件为 notification |

进程内客户端实现同一方法表，不走字节。

## 明确不做

- 不解冻 ACP / `c2305`
- 不引入 Fory / MessagePack；gRPC 不当 TUI 真源
- 不改 Command / Event 变体集合（审批从「假 reverse」改为登记 unary 是方法表行，不是新词表）
- 不把 `RpcMessage` 四象限钉进 live spec（代码组织）

## 迁移（只在 tasks，不进 MUST）

双读窗：codec 暂时仍能读旧四象限，出站按请求方言。翻产品客户端 → bump `PROTOCOL_VERSION` → 删旧解析。Salvo `POST /api/{method}` 可并行到 jsonrpsee 入口稳定。

## BDD

终态 executable 对准 **jsonrpsee 线上形状**（单一 RPC 入口、subscription notification、approve unary、未知方法 `-32601`），不是四象限改名。清单：[research/executable-envelope-bdd.md](./research/executable-envelope-bdd.md)。
