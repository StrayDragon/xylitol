# 主流 RPC 替换（相对「四象限字段打补丁」）

> c2825 探索。不计成本、为可维护性。ACP 仍后置。

## 为什么现在的 design 是补丁

把 `RpcMessage` 的字段改名塞进 JSON-RPC（`params.payload`、`params.writerToken`、mux 上假 request、respond 另走 POST）会得到：

- **两套信封**：IR 四象限 + 线上 JSON-RPC。每加横切能力都要在两层各开一个洞。
- **方法表在 URL、方法名又在 JSON**：`POST /api/{method}` 不是 JSON-RPC 的主流形状（主流是单一 `POST /` 或一条 WS，method 只在 JSON）。
- **反向 RPC 不是 JSON-RPC**：标准没有「server 在 WS 上下 request、client 用另一条 HTTP 回 result」。通用 jsonrpsee/client **不会**帮你做 `/api/respond`。
- **租约/幂等进 params**：污染每个方法的业务 params，无法对未知方法统一中间件。

这能 parse，但扩展点全是特例。

## 三层（可替换的才叫框架）

```text
① 词表     Command / Event / REGISTRY（Auth/Idem/Resp/Exec）
② RPC 运行时  方法注册、id、error、notification、subscription、中间件
③ 传输     HTTP 和/或 WebSocket（由 ② 提供，而不是自研 envelope.rs）
```

横切（写者租约、rpcId 幂等、协议版本）走 **② 的 middleware / Extensions / HTTP headers**，不进 ① 的 params。

## 候选运行时

| | jsonrpsee（Parity，0.26） | tonic + protobuf | 自研 codec 打补丁 |
|---|---|---|---|
| 信封 | 真 JSON-RPC 2.0 | gRPC | 看起来像 JSON-RPC |
| 流 | `register_subscription` → notification | proto stream | 自研 mux |
| 客户端 | 官方 Http/Ws client，WASM 有 | 任意 grpc | 手写 HttpWsClient |
| 中间件 | `RpcServiceBuilder`；HTTP `set_headers` | interceptor | 每个 handler 复制 |
| 调试 | 文本 JSON；可再接 OpenRPC | grpcurl / proto | Scalar 信封级 |
| 审批 | **notification + 客户端 unary `approve`**（主流） | 同左或 bidi | 假 reverse RPC |
| 与 ACP | 同族信封，词表仍不同 | 不同族 | 同族但方言 |
| 替换 Salvo 路由 | 是（jsonrpsee 自带 HTTP/WS server） | 是 | 否 |

Rust 生态里 JSON-RPC 的事实标准是 **jsonrpsee**（polkadot-sdk / subxt / zkSync 在用），不是再写 `codec.rs` 映射表。

gRPC 更「RPC 框架」，但本票目标是 JSON 可调试 + 与业界 agent 线同族；gRPC 适合另开程序脸，不适合替换 TUI 真源。

## 若采用 jsonrpsee：形状（终态）

- **一个 RPC 入口**（HTTP 与 WS 同一 `RpcModule`），不再 `POST /api/{method}` 当产品真源（调试别名可留）。
- `params` = 该方法的 **Command 字段**，没有 `payload` 袋子。
- `id` 由 jsonrpsee 管；幂等 middleware 认这个 id。
- `writerToken`：连接级 Extensions 或 header（如 `X-Writer-Token`），方法 params 无此字段。
- 下行事件：`session_subscribe` → JSON-RPC **notification**（无 id，不必 respond）。
- 审批：下行 **notification** `approval/requested`；客户端调 unary `approve_tool`（今日禁止把审批登记为 unary 的条款要改——这是可维护性换来的）。
- 握手：普通 `initialize`/`host.describe` 方法，不必 `ServerHello` 特例帧。
- 业务错误：jsonrpsee `ErrorObject`；`data.code` 仍是 xylitol 字符串。

通道纪律从「WS 禁业务上行」变为「WS 上只有 JSON-RPC 帧（含客户端 unary）」——这是用主流框架的代价，比假四象限更干净。

## A vs B（前瞻 / 可维护）

| 五年后要加的东西 | A jsonrpsee | B 字段补丁 |
|---|---|---|
| 新 unary | `REGISTRY` 一行 → `register_async_method` | REGISTRY + URL 路由 + codec 映射 + 四象限 IR |
| 新横切（鉴权、超时、trace） | 一层 `RpcServiceBuilder` | envelope 新字段 + 每个 handler 复制 |
| 新订阅流 | `register_subscription` | 再发明一种下行 tag |
| 非 Rust 客户端 | 官方 HTTP/WS client；curl 一行 JSON | 手写四象限改名规则，文档即方言 |
| 日后 ACP 脸 | 同族信封，只译词表 | 先要再做一次「真 JSON-RPC」迁移 |
| 调试 | 标准 JSON-RPC；未知方法 `-32601` | URL 404 与信封码两套 |

B 的唯一优点是 **本票 diff 小**：通道、`HostClient::respond`、`POST /api/respond`、mux 禁上行都可以不动。代价是把自研运行时再锁一轮；ACP 来时还是要拆 `/api/respond`。

不计成本、要可维护：**A**。B 不是过渡，是把补丁写进合约。

「只换字节、IR 仍四象限、通道不变」视为 **被否决的过渡幻想**。可维护路径是 **换 ②**。codec 抽取仍有用（双读窗），但不是终态架构。ACP 仍后置。

Salvo 不必一次拆光：jsonrpsee 可作 tower service 挂 RPC 入口，调试 `GET /openapi.json` / Scalar 继续走现有 HTTP 栈。日后 Web 端也走这条切分——页面/CORS/OAuth 用 HTTP 框架，agent 仍用同一 `RpcModule`，不要为浏览器再发明 REST 词表。
