# HTTP/WS 框架选型：needs 覆盖 vs axum / poem（结论：salvo）

> **2026-08-20 补记：** 产品信封已改到 c2290 四象限。Salvo **结论仍锁**。下表「WS 全双工承载 tagged JSON Command/Event」作废——网络路径是 POST unary + **WS 只下行** `ServerRequest`。ACP 仍不走产品信封。
>
> c2302「契约传输面」的框架决策。**本文只对比特性覆盖，不做可行性验证**（按用户要求）。一手来源：本地 `salvo-skills` 全套（websocket / realtime / graceful-shutdown / ...）、docs.rs 官方、c2280 research `01` 既有对照。版本锚：salvo 0.94/0.95（2026），axum 0.8。
> 需求来自 c2300（A 统一 CS、本机 UDS 一级拓扑）、c2290（四象限）、c2305（ACP provider 未来）、c2280 research（热路径 / 多订阅者广播 / 停服语义）。

## 需求 → 覆盖矩阵（salvo）

| 我们的需要 | salvo 对应 | status |
|---|---|---|
| WS 全双工承载 tagged JSON Command/Event（含反向 RPC 应答） | `WebSocketUpgrade` 全双工；`WebSocket` 是 `Stream+Sink`，`split()`/`recv()`/`send()`；`#[serde(tag="type")]` 直接可用 | ✅ 一等 |
| journal 广播到 N 个 TUI 连接，每连接独立队列 + 慢客户端背压 | realtime skill 明确推荐：**bounded `mpsc` per-client queue** 或 broadcast `RecvError::Lagged` 丢滞后订阅者；`max_message_size/max_frame_size/write_buffer_size` 旋钮；concurrency-limiter 可封顶订阅者 | ✅ 文档化正解（正是我们要的形态） |
| 优雅停机：等在途请求 + 收掉全部 attach，可带超时强停 | `ServerHandle::stop_graceful(Some(dur))` / `stop_forceful()` / `Server::max_connections` | ✅（WS 回调与 axum 同为 spawn 任务，订阅者收摊仍需自管理） |
| 本地 UDS + 远程 TCP 双听 | `salvo_core::conn::unix::UnixListener` + `Listener::join` | ✅（axum 是两个 serve 任务）。产品默认是 HTTP 端口，UDS 非必须 |
| UDS sock 权限（uid/gid/chmod 防同机他用户） | bind 时 `permissions`/`owner` 一等 API | ✅（axum 要自己 `set_permissions`/libc） |
| 升级前身份/授权 + Origin 校验（浏览器未来） | hoop 中间件 + depot `jwt_auth_data`；`allowed_origins`/`check_origin`；query 在 upgrade 前可读 | ✅ |
| 心跳 / 断线检测 / 连接计数 | `Message::ping`（auto-pong）+ `select!` 心跳；`AtomicUsize` 计数模式 | ✅ 文档化 |
| 未来：浏览器同源挂静态 + WS + CORS | `salvo-static-files`、`salvo-cors`（permissive 可配）、WS 原生可升级 | ✅ |
| 未来：ACP provider `/acp` 单端点（WS-only 起步；streamable HTTP 可后加） | 同一 Router/listener 挂 `/acp` + `WebSocketUpgrade`；HTTP/2 支持在（Streamable HTTP 用）；SSE 也有（`salvo-sse`） | ✅ |
| 明确不需要（关掉/不启用）：多租户 SaaS、serverless 负载均衡、数据库/缓存/ORM、代理网关 | 按需 feature 或不用；`salvo-openapi`/`salvo-proxy` 等为可白拿项 | ✅ 无负担 |

## 与 axum 0.8 的逐项对比

| 项 | axum 0.8 | salvo 0.94/0.95 | 裁判 |
|---|---|---|---|
| WS 全双工 + split + tagged JSON | ✅ `WebSocketUpgrade`+`on_upgrade` | ✅ `WebSocketUpgrade`+`upgrade` | 平（都是 Sink+Stream） |
| 多订阅者广播 / 慢客户端背压 | 无官方专章（自写 channel/clone） | realtime skill 给 bounded per-client queue + lagged 丢弃正解 | **salvo 略胜（文档即答案）** |
| 优雅停机 + 强停 + 连接数上限 | `with_graceful_shutdown`；不等 on_upgrade spawn 流（自收） | `stop_graceful(Some(dur))`/`stop_forceful`/`max_connections`；WS 任务同样自收 | **salvo 多 force/上限** |
| UDS 双听 | 两个 `serve` 任务（丑但可测） | `Listener::join` → JoinedAcceptor 一个上挂 | **salvo 胜** |
| UDS sock 权限 | 自写 `std::fs::set_permissions`/libc | bind 时 `permissions`/`owner` | **salvo 胜** |
| 生态 / 官方 / 社区 | tokio-rs 官方、最大 | 单团队、docs+skills 全、星 ~4.4k | axum 更"大牌"，salvo 够用 |
| HTTP/3 / WebTransport / OpenAPI | 无一等 | 自带（未来可白拿） | salvo（非必要） |
| REST 调试旁路类型 | `Json<T>` extractor | `JsonBody<T>` extractor | 平 |

## 与 poem 3 的对照（c2280 已比，二选一）

- poem 3：一等 `UnixListener` + `with_permissions/with_owner` + `Listener::combine` 一行双听 + `websocket` feature —— 与 salvo **能力同级**，c2280 结论"不必两个都引进"。
- salvo：星略高、`JoinedListener/JoinedAcceptor` 语义清晰、全套官方 skill（对 AI 开发导航好）、带 HTTP/3。
- 结论：两者都满足；考虑用户偏好 + 本仓库后续要用 skill 走 TUI 开发心智，**取 salvo**。

## 结论（锁）

1. **salvo 覆盖当前与未来可预见全部需求**（WS 一等、广播背压文档化、优雅停机、UDS+TCP 双听、sock 权限、auth/origin、静态/CORS、ACP 单端点、SSE/HTTP/2 可后加）。
2. 相对 axum 的差异点（双听、sock 权限、force 停机、背压文档）在本机 UDS + 远程/容器 TCP 上有用；REST 废弃重建后换栈税下降。
3. **默认取 salvo**（c2302 契约传输面）；**axum 为保守回退**记录在案。不做可行性验证（按用户要求；文档与 skill 覆盖已足够支撑决策，首次实现时若遇文档与代码不符再回退评估）。

## 一手来源

- 本地 salvo skills：`salvo-websocket`、`salvo-realtime`、`salvo-graceful-shutdown`、`salvo-auth`、`salvo-cors`、`salvo-static-files`、`salvo-sse`（`~/.config/llman/skills/__submodules__/salvo-skills/`）。
- docs.rs：salvo `conn` 模块与 [conn::unix::UnixListener](https://docs.rs/salvo_core/0.89.2/salvo_core/conn/) · [UnixListener struct](https://docs.rs/salvo_core/0.58.2/salvo_core/conn/unix/struct.UnixListener.html)
- [JoinedAcceptor / AcmeAcceptor public · salvo-rs/salvo #1146](https://github.com/salvo-rs/salvo/issues/1146)
- c2280 research `01`（axum/poem/salvo 能力对照、热路径、旧「钉 axum」premise）
