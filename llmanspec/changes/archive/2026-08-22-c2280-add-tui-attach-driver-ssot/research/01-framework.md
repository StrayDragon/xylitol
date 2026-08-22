# 01 服务端框架与流模型候选（能力对照，非裁决）

> 能力对照。选型在 c2302（salvo），不在本篇。
> 负载：约 1–3 连接、token 级小 JSON、双向流、本机 UDS、浏览器可连。不是 C10k，不是 CRUD REST。

## 热路径（先于任何框架）

```text
XyEvent → to_wire_event → serde_* → 套接字帧 → N 个订阅者
```

REST 的 `Json` extractor **不在**这条路上，HTTP 路由器的 RPS 榜对 token 流几乎无意义。框架只决定四件事：套接字怎么来、帧模型是 WS / gRPC / 自写、背压旋钮在哪、runtime 是否是 tokio。

## 四条候选架构（先选这条，再选 crate；选谁是裁决）

| 架构 | 流模型 | 序列化 | 浏览器 | 反向 RPC | 对应何种取舍 |
|---|---|---|---|---|---|
| **A. HTTP 升级 → 双向 JSON 帧** | WS text；自己 loop 推 Event | 现有 tagged JSON | 原生 WebSocket | 同一连接回帧 | 保留 Command/Event 且浏览器要接 |
| **B. 每连接 Service/Actor** | mailbox / service factory 收发 Frame | 可仍 JSON | 原生 WS（经 HTTP 升级） | mailbox 天然对 ask | 把「审批/问卷」做成连接对象而非全局 oneshot map；要接受双 runtime |
| **C. 类型化 bidi RPC** | gRPC stream / tarpc | protobuf 或另 schema | tonic 要 grpc-web | 一流二向最干净 | 允许换协议 |
| **D. 裸 UDS 帧** | 长度前缀或 tungstenite 免 HTTP | JSON 或 bincode | 没有，Web 必须第二套 | 自写 | 只优化本机 attach |

存在两个显式产品决策维度，不是框架快慢问题：**换不换协议**（A/B 不换、C 换、D 换载体）、**本机与 Web 是否分载体**（D 会分，A/B/C 不分）。

## 候选 crate 能力对照

筛选口径：只评**服务端能收发 JSON** 的 crate；GitHub `language:Rust stars:>=4000` 里去掉编辑器 / CLI / WASM / 中间件 / 代理后，剩 6 个 tokio 圈常见名：**axum (26900)、Rocket (25774)、actix-web (24784)、warp (10360)、poem (4433)、salvo (4422)**。hyper (16276) 是人人都熟的底层库，坐这 6 个下面，不当产品壳。

对本题，6 个的差异几乎全在「门口」——UDS、双听、WS 推流 API——**不在** token 序列化。

| crate | UDS | 双听（本机 sock + loopback） | WS / 背压 | 对该负载的注意点 |
|---|---|---|---|---|
| **axum 0.8** | 官方支持 `axum::serve(UnixListener)` + 官方示例 | 两个 serve 任务 / 同一 Router | `WebSocketUpgrade`；`on_upgrade` 会 `tokio::spawn`，优雅停机**不等**该流 | 本仓已依赖；无官方 JSON-RPC 方法表 |
| **Rocket 0.5** | `rocket::listener::UnixListener` | 配两套 endpoint | `rocket_ws` 的 `Stream!` / `channel`，发送队列有 `max_send_queue` | 推 Event 像往 channel 塞、背压最直白；换它=全页重写，维护偏静 |
| **actix-web 4** | 官方 `HttpServer::bind_uds` | 多次 bind | WS 默认走 `actix-ws` actor | UDS 文档最硬；worker 按核数拉线程、WS actor 是两套模型，对本机 1～3 连接偏重 |
| **warp 0.4** | 0.4 无官方 UDS（0.3.7 的 `serve_incoming` 才写 Unix） | 无 | `warp::ws()` 官方 | Filter 组合漂亮；本机 sock 要自接 hyper |
| **poem 3** | 一等 `UnixListener::bind` + `with_permissions` / `with_owner` | 官方 `listener.combine` 一行双听 | feature `websocket` | UDS / 双听 / sock 权限最完整；REST 可切 sonic-rs；生态比 axum 小 |
| **salvo 0.95** | 一等 `UnixListener`（feature `unix`） | 官方 `listener.join` 一行双听 | `WebSocketUpgrade` + Sink/Stream | 与 poem 同档、同星同期；HTTP/3 属于远程后置心智能不能接受 |

常被提名但**不过闸**：**ntex**（2528，更像「网络服务」框架，自有 current-thread runtime）与 **jsonrpsee**（851，唯一内置 JSON-RPC 2.0 的服务端）。**tide**（5094）过闸但 2024 起休眠。

### poem / salvo 相对 axum 的 UDS 优势，落在哪条连法

poem `/` salvo 在**本机 Linux、bind 时 chmod、一行双听**上确实更完整（这是能力事实，不因本仓已用 axum 而消失）。但「UDS + TCP 同一进程」服务的是**本机不经 Docker 的 serve**；对「宿主机的 TUI 连容器里的 server」这条连法没有帮助——那是 TCP + 发布端口，三个 crate 在这条路上同构。换栈的重写范围是 `src/app/server/{rest,ws,runtime}.rs` 的 HTTP 皮。

## 与高频 / 序列化真正相关的旋钮

| 旋钮 | 谁提供 |
|---|---|
| 慢客户端发送队列上限 | Rocket `max_send_queue`；tungstenite write buffer（Salvo / Poem / warp 都能摸到） |
| JSON 实现 | Poem feature sonic-rs；或 **WS 帧上自己** `sonic_rs` / `serde_json`（HTTP 框架的 RPS 与 token 流无关） |
| 少 clone 到 N 窗 | 应用层 journal（Arc 字节 / 广播），与框架无关 |
| HTTP/2 流控窗口 | tonic；actix `h2_initial_window_size`——对 WS/1.1 文本帧帮不上 |

## 与 HTTP 框架无关的已知瓶颈（换栈不影响这些）

- `XyRemoteDriver::execute_bash` 丢 `chunk_tx`（REST 一问一答，bang 无直播输出）→ 见 `03`
- journal clone / `Mutex<XyInProcessDriver>`
- 每枚 `TextDelta` 的 JSON 分配

## JSON-RPC / gRPC 两个信封类别的性质

- **JSON-RPC 2.0**：任何一个 HTTP 框架都能 POST JSON、再自解析信封——但**没有** ≥4000 星的专门服务端（jsonrpsee 只有 851）。词汇从「闭集 enum」变「字符串 method」；server→client 的 `ApproveTool` 要扭成反向调用或 notification。信封形状对比：
  ```json
  {"jsonrpc":"2.0","method":"prompt","params":{"message":"fix the bug"},"id":"1"}
  ```
  对照现在的 tagged JSON：
  ```json
  {"type":"prompt","message":"fix the bug"}
  ```
  （信封换了，消息种类还是那些。）
- **gRPC / protobuf**（tonic / prost）：流式语义第一、UDS 官方、HTTP/2 窗口 = 协议级背压，`UdsConnectInfo::peer_cred` 可做 uid 校验；代价是**放弃现有 tagged Command/Event**、浏览器要 grpc-web（官方只支持 unary + server-streaming，无 bidi）。

## 一手来源

- axum：`Listener` for `tokio::net::UnixListener`（[listener.rs](https://docs.rs/axum/0.8.8/src/axum/serve/listener.rs.html)）· [unix-domain-socket 示例](https://github.com/tokio-rs/axum/blob/main/examples/unix-domain-socket/src/main.rs) · [ws 模块](https://docs.rs/axum/0.8.8/axum/extract/ws/index.html) · [on_upgrade spawn 注释](https://github.com/tokio-rs/axum/blob/main/src/extract/ws.rs) · [Serve::with_graceful_shutdown](https://docs.rs/axum/0.8.8/axum/serve/struct.Serve.html)
- Rocket：[UnixListener](https://api.rocket.rs/master/rocket/listener/unix/struct.UnixListener.html) · [rocket_ws](https://api.rocket.rs/master/rocket_ws/)
- actix-web：[HttpServer::bind_uds](https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.bind_uds) · [actix-ws book](https://actix.rs/docs/websockets/) · [MIGRATION-4.0](https://github.com/actix/actix-web/blob/main/actix-web/MIGRATION-4.0.md)
- warp：[0.3.7 serve_incoming（Unix）](https://docs.rs/warp/0.3.7/warp/struct.Server.html#method.serve_incoming) · [0.4.3 Server（不再写 Unix）](https://docs.rs/warp/0.4.3/warp/struct.Server.html)
- poem：[UnixListener](https://docs.rs/poem/latest/poem/listener/struct.UnixListener.html) · [Listener::combine](https://docs.rs/poem/latest/poem/listener/trait.Listener.html)
- salvo：[UnixListener](https://docs.rs/salvo/latest/salvo/conn/unix/struct.UnixListener.html) · [Listener::join](https://docs.rs/salvo_core/latest/salvo_core/conn/trait.Listener.html)
- tonic：[unix.rs（peer_cred）](https://docs.rs/tonic/latest/src/tonic/transport/server/unix.rs.html) · [tonic-web（无 WS / 无 bidi）](https://docs.rs/tonic-web/latest/tonic_web/)
- jsonrpsee：[README](https://github.com/paritytech/jsonrpsee/blob/master/README.md) · [SubscriptionSink 背压](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/struct.SubscriptionSink.html)
- 星标闸（2026-08-18 gh api / crates.io）：[axum](https://github.com/tokio-rs/axum) · [Rocket](https://github.com/rwf2/Rocket) · [actix-web](https://github.com/actix/actix-web) · [warp](https://github.com/seanmonstar/warp) · [poem](https://github.com/poem-web/poem) · [salvo](https://github.com/salvo-rs/salvo) · [ntex](https://github.com/ntex-rs/ntex/) · [jsonrpsee](https://github.com/paritytech/jsonrpsee)
