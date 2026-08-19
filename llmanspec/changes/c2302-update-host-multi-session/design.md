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

本票 **替换**（不再并存）：`server-core` sr2/sr3/sr7/sr8/sr9/sr-st1 的 REST/锁/port+1；`layer-architecture` la6 单实例锁+REST 产品面；`w3` 的 URL 绑 session。`ip3` REST 信封类型可留在 protocol 作历史 DTO，但 server **不得**再用它承载产品动词。`w1`/`w2`/`w3` 的 `ServerFrame`/`ClientFrame` 产品 MUST 废止（journal/seq/resync **语义**迁到四象限下行方法）。

## 9. 对照 c2290 / 现码后锁定（未决只留 §10）

- **不实现** `GET /api/events.host`。TUI v1 只订 `GET /api/events.mux`。
- mux 每连接 bounded mpsc 满：对该连接发 `session/resync_required`；已无法投递则断开。MUST NOT 静默丢事件。
- **写者身份是 `writerToken` 租约，不是 HTTP 连接**（每次 unary 都是新 TCP）。首次非只读 unary 颁发；后续写回显；否则 `writer_conflict`。只读 unary / subscribe 不占写者。`HttpWsClient` 实例（含 clone）共享令牌。
- WS 收到业务上行 text/binary：**关连接**。ping/pong/close 按载体处理。
- 产品 REST `/api/v1/...` **删除**，不 410 双路径。未知 unary path → 404 或非法信封 400，不发明 REST 别名。
- **不扩方法表**：`list_sessions` / `session_tree` / `queue_stats` 不登记。`XyRemoteDriver` 继续 unsupported/default；live spec 改掉 sr-st1 / 对 REST 树的 MUST，而不是把树塞回 HTTP。
- 绑定 `ServerConfig.host`+`port`（默认 `127.0.0.1:18790`）。EADDRINUSE → 失败。禁止 port+1。删产品路径上的锁文件（`lock.rs` / `port_retry` 的占用语义）。
- `server stop`：不再读 `/tmp/xylitol-server.lock`。本票只打印「对监听进程发 SIGTERM」；**不**做 PID 发现。`serve --stop` 归 c2303。
- `host.describe` 必须实现。产品 TUI attach 成功后 unary 一次；`protocol` 对不上 → 失败断开，不降级。版本不走 WS subprotocol。
- ReverseRpc：mux 下发可应答 `approval/requested` / `question/requested`；应答只 `POST /api/respond`。TUI `AskHostGateway` 本票改走 `HostClient::respond`，禁止再在附加模式下本地假审批。
- `XyRemoteDriver::run()`：先 `mux`，再 unary `subscribe`（session + last_seq），再 `prompt`。
- `sr-env1`：真 bind + POST unary + WS 下行往返，禁止只断言类型存在。
- unary 解 payload → 现有 `dispatch` / `XyDriver`。禁止第二套与 Driver 漂移的 handler 语义。
- 不挂 permissive CORS（TUI 常无 Origin）。WS `check_origin` 允许 `None`。
- salvo 不启用 `logging`（避免 `tracing`）。`oapi` 不进本票。
- `InProcessClient` Echo → c2304。print / embed 仍 InProcessDriver，不占 TCP。
- 组合根仍 `bootstrap`（ce9）。现码是一把 `Mutex<XyInProcessDriver>` + 一个 journal。Driver 基数见 §10（已钉）。

## 10. Driver 基数（已钉）

**占用粒度 = session 槽，不是 Host 进程。**

产品已经按 session 切占用：`la-cs5` 一 session 一写者、每 session 独立 journal/seq。若进程里只有一把 turn 引擎，就会出现两套占用模型——session 上说「可以有各自的写者」，进程上却只有一个 busy / 一条 steer 队列 / 一次 `switch_session`。两窗同时 `prompt` 会互相踩，journal 也无法诚实地按 session 追加。

因此：

- **Host 进程**共享一份 `RuntimePorts` 基线（store / MCP / 模型注册表 / 工具 / prompt）。reload 打在这份基线上，再按槽物化。
- **每个 session 槽**在首次需要写者（非只读 unary）时 lazy `materialize` 一把隔离的 `XyInProcessDriver`（新的队列、绑定、compaction、busy）。闲置槽可在无订阅者且无进行中回合后回收（本票可先不回收，只 lazy 创建）。
- **一把 Driver 不得**同时绑定两个 session 做两次根提交。这不是「进程里只能有一个 Runtime」——那是把 TUI 单窗习惯误当成 Host 拓扑。
- 只读 attach 只订 mux / 读方法，不物化写者引擎。

本票按此落地，不把 N 槽推迟到下一票（否则 salvo 换栈后还要再拆一次 AppState）。
