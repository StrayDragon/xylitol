# Design：salvo 传输面

一手 API 以 `~/.config/llman/skills/__submodules__/salvo-skills/` 为准（0.94）。Rust 1.97 满足 salvo 0.94 的 1.94+ 要求。

## 1. 依赖与特征

```toml
salvo = { version = "0.94", features = ["websocket"] }
```

主仓 `server` feature：由 `axum` + `tower-http` 改为 `salvo`。WS **客户端**仍可用 `tokio-tungstenite`（RemoteDriver）。

不用：`concurrency-limiter`（1–3 连接）、`cors`（Web 未开闸）、WS `protocols()`（版本走 `ServerHello`，c2301）。

## 2. 监听与停机

- `TcpListener::new("127.0.0.1:18790").bind().await`（salvo-basic-app）。host/port 来自 `ServerConfig`。
- 绑定失败（含 EADDRINUSE）→ 失败返回，**禁止** port+1。
- 删单实例锁文件路径；`server stop` 不再读 `/tmp/xylitol-server.lock`，改为提示对进程发 SIGTERM。
- 停机：`let handle = server.handle();` + Unix `SIGTERM`/`SIGINT` → `handle.stop_graceful(Duration::from_secs(30))`（salvo-graceful-shutdown）。WS 回调是 spawn 任务，订阅者 map **必须自收**（从 per-client Tx 摘掉）。

## 3. 路由

```text
GET  /healthz     非产品；进程活着即 200（停机中可 503）
GET  /ws          WebSocketUpgrade → Command/Event 帧
```

不挂 `/api/v1/session/{id}/run` 等产品 REST。session 由首帧 `Subscribe { session_id, last_seq }` 选择（已有 ClientFrame）。

Origin：TUI 等原生客户端常 **省略** Origin。用 `check_origin(|o| o.is_none() || /* 未来浏览器白名单 */)`，**不要** `allowed_origins`（会 403 缺 Origin）（salvo-websocket）。

升级前可读 query；认证若有也在 upgrade 前。本票无 cookie 会话。

## 4. 连接内循环（salvo-websocket + realtime）

- `WebSocketUpgrade::new().max_message_size(..).upgrade(req, res, handle)`。
- `handle` 内 `ws.split()`；sink 项类型 `Result<Message, salvo::Error>`。
- **每连接 bounded `mpsc`**（realtime：慢客户端背压；不要抄 skill 里的 unbounded 聊天示例）。队列满 → 对该连接 `ResyncRequired` 或断开（与 journal 满策略一致，实现时二选一写进注释）。
- 文本帧 `serde_json` 解 `ClientFrame` / 产品 `Command`；发出 `ServerFrame` tagged JSON（`#[serde(tag="type")]`，与 skill JSON 节同构）。
- 心跳：`select!` 里 `Message::ping`（auto-pong）；原生客户端省略 Origin 已在升级策略处理。
- 不把秘密放进 `Sec-WebSocket-Protocol`。

## 5. 多 session 与写者

```text
HostState
  sessions: HashMap<session_id, SessionSlot>
    journal, seq, writer_conn, subscribers: HashMap<conn_id, Tx>
```

- 第一个对该 session 发非只读 Command 的连接成为写者。
- 已有写者时，新连接只收 Event；再写 → `Event::Error` 说明已有其它客户端以写者连接（c2301 缺口变体未到前用 Error，不新开 REST）。
- journal / reverse RPC 保留现有语义（rr1–rr4、w4–w7）；从 axum 任务迁到 salvo 回调。

## 6. 与 embed

InProcessDriver **不**占用 TCP。产品语义仍是 Command/Event → 同一 dispatch（c2300）。本票只加监听器这条 carrier。禁止「只有 WS 才进 dispatch」。

## 7. 测试边界

| 测什么 | harness |
|---|---|
| healthz / 绑定占用 | salvo `TestClient`（无 TCP）+ 真 bind 的 EADDRINUSE 测 |
| WS 帧 / journal / 反向 RPC | 现有 `server-ws` / `server-reverse-rpc` 场景，迁到 salvo 升级路径 |
| 产品不再走 REST | 改写 `server-runtime.feature`：run/tree REST 场景改为「HTTP 产品路由不存在或非产品」；RemoteDriver 走 WS |
| BDD | `cargo test --test bdd`（server-core 各 `@req`） |

## 8. 旧 MUST 收口

本票 **替换**（不再并存）：`server-core` sr2/sr3/sr7/sr8/sr9/sr-st1 的 REST/锁/port+1；`layer-architecture` la6 单实例锁+REST 产品面；`w3` 的 URL 绑 session。`ip3` REST 信封类型可留在 protocol 作历史 DTO，但 server **不得**再用它承载产品动词。
