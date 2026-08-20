# Design：salvo 载体（四象限）

一手 API 以 `~/.config/llman/skills/__submodules__/salvo-skills/` 为准（0.94）。Rust 1.97 满足 salvo 0.94 的 1.94+ 要求。框架选型结论仍见 `research/framework-pick.md`（salvo）；该文「WS 全双工承载 Command/Event」行作废，改为「WS 只做下行」。

## 1. 依赖与特征

```toml
salvo = { version = "0.94", features = ["websocket"] }
```

主仓 `server` feature：由 `axum` + `tower-http` 改为 `salvo`。TUI/Remote 客户端仍可用 `reqwest`（unary）+ `tokio-tungstenite`（下行）。

不用：`concurrency-limiter`（1–3 连接）、`cors`（Web 未开闸）、WS `protocols()`（版本走握手字段，c2301/c2303）、`logging`（会拉 `tracing`，与 fastrace 双栈冲突）。

`oapi` **不**在本票启用。OpenAPI 不是类型 SSOT；specta 闸在 c2290。salvo oapi 仅 unary 调试文档另草案。

## 2. 监听与停机

- `TcpListener::new("127.0.0.1:18790").bind().await`。host/port 来自 `ServerConfig`。
- 绑定失败（含 EADDRINUSE）→ 失败返回，**禁止** port+1。
- 删单实例锁文件路径；`server stop` 不再读 `/tmp/xylitol-server.lock`，改为提示对进程发 SIGTERM。
- 停机：`let handle = server.handle();` + Unix `SIGTERM`/`SIGINT` → `handle.stop_graceful(Duration::from_secs(30))`。WS 回调是 spawn 任务，订阅者 map **必须自收**。

## 3. 路由

```text
GET  /healthz              非产品；进程活着即 200（停机中可 503）
POST /api/respond          ClientResponse
POST /api/{method}         ClientRequest（method 与路径一致，handler 校验 path==method）
GET  /api/events.mux       WebSocketUpgrade → 只下行 MuxFrame/ServerRequest
GET  /api/events.host      可选；host 级帧。TUI v1 可只订 mux
```

`{method}` 集合 = c2290 `research/method-table.md` 的 unary 列（含 `host.describe`）。未知 path → 信封解析失败 / 404，不默默加 REST 别名。

不挂 `/api/v1/session/{id}/run` 等产品 REST。session 由 unary `subscribe` / 首个 prompt 的 payload 选择，不靠「连上哪根 `/ws`」。

Origin：TUI 等原生客户端常 **省略** Origin。下行 upgrade 用 `check_origin(|o| o.is_none() || /* 未来浏览器白名单 */)`，**不要** `allowed_origins`（会 403 缺 Origin）。

升级前可读 query；认证若有也在 upgrade 前。本票无 cookie 会话。WS **拒绝** 应用层客户端消息（text/binary 业务帧 → 关闭或忽略并记日志）。

## 4. 连接内循环（salvo-websocket + realtime）

- `WebSocketUpgrade::new().max_message_size(..).upgrade(req, res, handle)`。
- `handle` 内 `ws.split()`；sink 项类型 `Result<Message, salvo::Error>`。
- **每连接 bounded `mpsc`**。队列满 → 对该连接 `ResyncRequired` 或断开。
- 发出的文本帧：完整 `ServerRequest` JSON（含 `type`、`rpcId`、`method`、`payload`），不是旧 `ServerFrame` tagged Command/Event 外层。
- 心跳：`select!` 里 `Message::ping`（auto-pong）。
- 不把秘密放进 `Sec-WebSocket-Protocol`。

Unary handler：`JsonBody<ClientRequest>` → dispatch → `Json<ServerResponse>`。业务错误走 `result.ok=false`，HTTP 仍 200（对齐 DSH：HTTP 只表示载体）。非法信封 → 400/`bad-request`。

## 5. 多 session 与写者

```text
HostState
  sessions: HashMap<session_id, SessionSlot>
    journal, seq, writer_conn, subscribers: HashMap<conn_id, Tx>
```

- 第一个对该 session 发非只读方法的连接成为写者。
- 已有写者时，新连接只收 `ServerRequest`；再写 → 业务 error，说明已有其它客户端以写者连接。
- journal / reverse RPC 保留现有语义；从 axum 任务迁到 salvo 回调。反向 RPC：host 在 mux 上下发可应答 `ServerRequest`，client `POST /api/respond`。

## 6. 与 embed

InProcessDriver **不**占用 TCP。产品语义仍进同一 dispatch（c2300）。本票只加监听器这条 carrier。禁止「只有套接字才进 dispatch」。

## 7. 测试边界

| 测什么 | harness |
|---|---|
| healthz / 绑定占用 | salvo `TestClient`（无 TCP）+ 真 bind 的 EADDRINUSE 测 |
| POST unary / respond / WS 下行 / journal / 反向 RPC | 改写 `server-ws` / `server-reverse-rpc` / `server-runtime` 场景 |
| 产品不再走 REST | run/tree REST 场景改为「HTTP 产品路由不存在或非产品」 |
| BDD | `cargo test --test bdd`（server-core 各 `@req`） |

## 8. 旧 MUST 收口

本票 **替换**（不再并存）：`server-core` sr2/sr3/sr7/sr8/sr9/sr-st1 的 REST/锁/port+1；`layer-architecture` la6 单实例锁+REST 产品面；`w3` 的 URL 绑 session。`ip3` REST 信封类型可留在 protocol 作历史 DTO，但 server **不得**再用它承载产品动词。
