# 类型共享：Eden 在 Rust 侧没有同构物

> 给 c2290 用。不是 live spec。
>
> **2026-08-20 锁定：** specta derive + `bindings.ts` + qa 闸 **进 c2290 硬任务**（无 Web UI）。薄 TS 客户端 / Vite SPA / salvo oapi 调试文档另草案。OpenAPI/AsyncAPI 不当 v1 SSOT。

Eden（Elysia）能做端到端类型安全，是因为 **server 与 client 都是 TypeScript**：`typeof app` 推断出 Treaty 客户端，零 codegen。[Eden overview](https://elysiajs.com/eden/overview)。砍掉 Elysia 之后，Host 是 Rust、Web 是 TS，这条路物理上不存在。

## 消费者分列

| 消费者 | 语言 | 要什么 |
|---|---|---|
| 产品 TUI / print / embed | Rust | 与 Host 同一套 Req/Resp/Frame |
| 未来 Web | TypeScript | 同一套形状的 TS 类型 + 薄客户端 |
| ACP | 外层翻译 | **禁止**共享产品信封；自己的 JSON-RPC |

TUI 与 Host 已在同一 crate：`protocol` 里的信封与 payload **直接 `use`**。这比 Eden 更强（同一编译单元，改了就红）。不需要、也不该为 TUI 走 OpenAPI。

Web 才需要跨语言导出。

## 候选（一手）

### 1. specta + specta-typescript（推荐给 Web）

- 源：[specta-rs/specta](https://github.com/specta-rs/specta/)
- `#[derive(Type)]` 后 `Types::register::<T>()` 会带上依赖图（这是相对 ts-rs 的明确优势，见 docs.rs `specta`「Why not ts-rs」）。
- TypeScript exporter：**Stable**。OpenAPI exporter（`specta-openapi`）：**Partial**（复杂类型未完成），不能当生产 OpenAPI 真源。
- 导出的是 **类型**，不是 Eden Treaty 那种 `client.session.prompt()` 对象。薄客户端仍要手写一层（对齐 DSH `AbstractApiClient`：铸 `rpcId`、POST、校验回显）。

### 2. ts-rs

- 源：[aleph-alpha/ts-rs](https://github.com/aleph-alpha/ts-rs)
- 按类型单个导出，不自动拉依赖图。serde tag 可对上四象限。
- 能用，但当方法表 + 嵌套 Event 闭集时不如 specta。

### 3. rspc（tRPC-like）

- 源：[specta-rs/rspc](https://github.com/specta-rs/rspc/)；停维护声明：[Discussion #351](https://github.com/specta-rs/rspc/discussions/351)（2025-03，作者不再维护；Tauri 集成已坏）。
- 会把 HTTP 栈换成 rspc/axum 路由，与「salvo 只做载体」冲突。
- **不采用。**

### 4. salvo oapi → OpenAPI 3.1 → 生成客户端

- 源：本地 skill `salvo-openapi`（0.94，`#[endpoint]` + `ToSchema`，OpenAPI 3.1）。
- 生成侧：TS 可用 openapi-typescript / hey-api；Rust 客户端可用 [progenitor](https://github.com/oxidecomputer/progenitor)（OpenAPI 3.0.x）或 [openapi-to-rust](https://openapi-to-rust.dev/)（3.0/3.1，含 SSE，但优化点是 axum）。
- **只覆盖 HTTP unary。** DSH/c2290 的下行是 WebSocket 上的 `ServerRequest`，OpenAPI 没有一等双向/server-push 帧模型。用 OpenAPI 当 SSOT 会迫使「事件走另一份 schema」——正是要避免的双轨。
- 适合：给外部调试挂 `/openapi.json`，或给只调用 unary 的脚本。不适合作为 TUI/Web 的类型真源。

### 5. AsyncAPI

- 源：[asyncapi-rust](https://docs.rs/asyncapi-rust)（code-first → AsyncAPI 3.0；示例绑 actix-ws / axum，**无 salvo**）；社区 [asyncapi-rust-ws-template](https://github.com/kanekoshoyu/asyncapi-rust-ws-template) 从 spec 生成 tungstenite 客户端。
- 官方 Generator 的 WS 模板以 Node 为主，且「spec 与代码双写易漂」是 AsyncAPI 自己承认的问题（[websocket-part3](https://www.asyncapi.com/blog/websocket-part3)）。
- v1 **不**上 AsyncAPI。WS 帧类型已经是四象限里的 `ServerRequest`；specta 导出那一个 union 即可。

## 锁定

1. **SSOT = Rust `protocol` 信封 + 方法表 payload**（serde）。TUI 零生成，直接 `use`。
2. **c2290 落地 specta 闸**：`#[derive(Type)]` + 检入 `bindings.ts` + `just qa` 重生 diff。覆盖信封、`RpcResult`、unary payload/value、`Event`、下行帧。**不是** Web UI。
   - specta-serde **unified** `Format` 盖不住 `skip_serializing_if`（要 PhasesFormat 才会拆 `*_Serialize`/`*_Deserialize`）。产品信封/`Event` 的 Option 字段改 `#[serde(default)]`、序列化显式 `null`，换单一 TS 类型。不要为可选字段再开双相导出。
   - JSON 数字：`u64`/`usize` 用 `#[specta(type = specta_typescript::Number)]`（JS number，会话 seq/token 可接受精度）；`serde_json::Value` 用 `Any`。禁止为 specta 把线类型改成 String bigint。
3. **薄 TS 客户端**（铸 `rpcId`、POST、校验回显、订 mux）→ `c2310-add-web-ts-client`；Vite SPA 更后。
4. salvo oapi / OpenAPI → `c2320-add-salvo-oapi-docs`；仅 unary **调试文档**；MUST NOT 成为第二套产品词表。WS 下行不进 OpenAPI。
5. 不引入 rspc、不为 TUI 引入 ts-rs、v1 不上 AsyncAPI。

## 与 DSH 的对应

DSH 用 TS 接口 + zod 当签名 SSOT，浏览器 `import type` 同一套。我们没有「全 TS 同仓」；用 Rust 类型当 SSOT、specta 补 Web，是同一分层（契约一层、载体一层），只是语言切在 Host 这边。
