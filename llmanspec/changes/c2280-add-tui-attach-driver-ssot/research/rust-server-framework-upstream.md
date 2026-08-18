# 上游摘录：Rust CS host 框架（非 xylitol 结论）

> 本文是 **一手资料摘录与能力对照**，不是 xylitol 产品决策。禁止当 live spec。
> 负载假定：token 级 TextDelta 小 JSON 帧、连接少但单连接高频、serde tagged `Command`/`Event`、需 WS 双向（含 server→client 反向调用）、UDS 和/或 `127.0.0.1`、未来浏览器原生 WS、graceful shutdown、可选 feature。不是 C10k、不是 CRUD REST 为主。
> 版本锚：axum 0.8.8 docs.rs；actix-web 4.14；warp latest=0.4.3（UDS 字样见 0.3.7）；jsonrpsee 0.26；tonic 0.14.6；tokio-tungstenite 0.29。体积数字一律 **未测**。

反向 RPC：下列 HTTP/WS 框架均无「server 调 client 方法」一等 API；同一连接上双向文本帧可承载自研 ask/问卷。JSON-RPC 一等的是 client→server method + server→client **subscription**；gRPC 一等的是 stream RPC。

---

## 1. axum 0.8（本仓已依赖 `axum = 0.8` + `features = ["ws"]`）

- **运行时**：官方「designed to work with tokio and hyper」；`#[tokio::main]` + `axum::serve`。[docs.rs/axum/0.8.8](https://docs.rs/axum/0.8.8/axum/)
- **WebSocket**：`extract::ws`（feature `ws`）：`WebSocketUpgrade` → `on_upgrade`；`WebSocket::{recv,send}`；`Message::Text`；`split` 并发读写。官方示例是 `while let Some(msg) = socket.recv()`。`Sink::poll_ready` 存在，但 **无** 高频小帧背压专章。[ws 模块](https://docs.rs/axum/0.8.8/axum/extract/ws/index.html) · [WebSocket](https://docs.rs/axum/0.8.8/axum/extract/ws/struct.WebSocket.html) · [Message](https://docs.rs/axum/0.8.8/axum/extract/ws/enum.Message.html)
- **Unix domain**：`Listener` 在 unix 上 **impl for `tokio::net::UnixListener`**；官方 example `UnixListener::bind` + `axum::serve(uds, app)`。[listener.rs](https://docs.rs/axum/0.8.8/src/axum/serve/listener.rs.html) · [example](https://raw.githubusercontent.com/tokio-rs/axum/main/examples/unix-domain-socket/src/main.rs) · [UnixListener](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html)
- **序列化**：HTTP `Json<T>` 要求 `serde::{Serialize,Deserialize}`，与 `#[serde(tag="type")]` 兼容。WS 帧是文本/二进制，JSON 自 `serde_json`。[Json](https://docs.rs/axum/0.8.8/axum/struct.Json.html)
- **流式**：SSE 一等（`response::sse`）；WS 一等；高频小消息要自己写 recv/send loop。[SSE](https://docs.rs/axum/0.8.8/axum/response/sse/index.html)
- **浏览器**：HTTP Upgrade → 101（`WebSocketUpgrade`），浏览器原生 `WebSocket` 可连。
- **与现有协议**：换框架不必换 `Command`/`Event` 词汇；只换 listener（TCP↔UDS）。
- **feature 体积**：`ws` 可选；默认含 `json`/`http1`/`tokio`。未测。[crate features](https://docs.rs/axum/0.8.8/axum/) · [crates.io/axum](https://crates.io/crates/axum)
- **graceful shutdown**：`Serve::with_graceful_shutdown`。[Serve](https://docs.rs/axum/0.8.8/axum/serve/struct.Serve.html)
- **一句话：采用** — 官方已覆盖 tokio 共用、UDS listener、双向文本 WS、serde JSON、浏览器 upgrade。[serve](https://docs.rs/axum/0.8.8/axum/serve/fn.serve.html)

---

## 2. actix-web 4

- **运行时**：「Full Tokio compatibility」；`#[actix_web::main] // or #[tokio::main]`。`HttpServer::run` 会 **另起 worker 线程**（默认 `available_parallelism`）。[crate](https://docs.rs/actix-web/4/actix_web/) · [HttpServer::run](https://docs.rs/actix-web/4/actix_web/struct.HttpServer.html)
- **WebSocket**：官方 book 走独立 crate **`actix-ws`**：`actix_ws::handle` → `session.text` + `stream.next()` 循环；双向文本有。无高频背压专章。[actix.rs/docs/websockets](https://actix.rs/docs/websockets/)
- **Unix domain**：`HttpServer::bind_uds` / `listen_uds`（Unix only）。[HttpServer](https://docs.rs/actix-web/4/actix_web/struct.HttpServer.html)
- **序列化**：`web::Json<T>` 要 serde Deserialize/Serialize；WS 文本自管 JSON。[web::Json](https://docs.rs/actix-web/4/actix_web/web/struct.Json.html)
- **流式**：文档列 Streaming/pipelining；WS 靠 `actix-ws` loop；SSE 非本页一等。
- **浏览器**：标准 HTTP WS upgrade（book 示例 `/echo`）。
- **与现有协议**：不必换词汇；多一个 actix-ws + worker 模型。
- **feature 体积**：默认开 cookies/macros/三套 compress。未测。[crate features](https://docs.rs/actix-web/4/actix_web/)
- **graceful**：`shutdown_signal` / `shutdown_timeout`。
- **一句话：否决** — 能打 UDS+WS，但相对已在树的 axum 增加 worker 线程与 `actix-ws` 依赖，官方无本负载增量。[websockets book](https://actix.rs/docs/websockets/)

---

## 3. warp

- **运行时**：建在 hyper 上；`tokio::task::spawn(server)` 示例。[Server 0.3.7](https://docs.rs/warp/0.3.7/warp/struct.Server.html)
- **WebSocket**：feature `websocket`；`warp::ws()` → 101 Switching Protocols；`WebSocket` 实现 `Stream`+`Sink`；ping 内部处理。无高频背压专章。[ws()](https://docs.rs/warp/0.4.3/warp/filters/ws/fn.ws.html) · [ws 模块](https://docs.rs/warp/latest/warp/filters/ws/index.html)
- **Unix domain**：**0.3.7** `serve_incoming` 原文「This can be used for Unix Domain Sockets」。**0.4.3** 改为 `incoming(acceptor)`，docs.rs **不再写 Unix**。[0.3.7 serve_incoming](https://docs.rs/warp/0.3.7/warp/struct.Server.html#method.serve_incoming) · [0.4.3 Server](https://docs.rs/warp/0.4.3/warp/struct.Server.html)
- **序列化**：`warp::body::json()` 要 `DeserializeOwned`。[json](https://docs.rs/warp/0.4.3/warp/filters/body/fn.json.html)
- **流式**：WS 一等；小消息自己 loop。
- **浏览器**：101 upgrade，原生 WS 可连。
- **与现有协议**：不必换词汇。
- **feature 体积**：未测。
- **graceful**：0.4 `graceful`；0.3 `bind_with_graceful_shutdown` / `serve_incoming_with_graceful_shutdown`。
- **一句话：否决** — WS 可用，但当前 0.4 无官方 UDS 示例，且与 axum 重叠。[crate](https://docs.rs/warp/latest/warp/)

---

## 4. hyper + tokio-tungstenite（无框架；本仓已有 tungstenite）

- **运行时**：hyper 是「lower-level HTTP」；I/O 由调用方接 tokio。[hyper](https://docs.rs/hyper/latest/hyper/)
- **WebSocket**：hyper **无** WS；`hyper::upgrade` 做 HTTP Upgrade，再 `tokio_tungstenite::accept_async`（`S: AsyncRead+AsyncWrite`）。`WebSocketStream` = `Stream`+`Sink`。官方 `accept_async` 文案「typically after TcpListener」，类型本身不限 TCP。[upgrade](https://docs.rs/hyper/latest/hyper/upgrade/index.html) · [accept_async](https://docs.rs/tokio-tungstenite/0.29.0/tokio_tungstenite/fn.accept_async.html) · [WebSocketStream](https://docs.rs/tokio-tungstenite/0.29.0/tokio_tungstenite/struct.WebSocketStream.html)
- **Unix domain**：无框架 listener；自接 `tokio::net::UnixListener`。
- **序列化**：无默认 JSON；自 `serde_json` 编 `Message`。兼容 tagged enum。
- **流式**：全自写 loop；无 SSE/gRPC 一等。
- **浏览器**：自己发 101 + `Upgrade: websocket` 即可被原生 WS 连。
- **与现有协议**：不必换词汇；要自写 HTTP/WS/UDS/shutdown 胶水。
- **feature 体积**：hyper 默认无 feature；tungstenite 已在 `server` feature。未测。
- **graceful**：hyper 无 axum 级 `with_graceful_shutdown`；自管连接。
- **一句话：本机可采用、Web 另开成本高** — 能绑 UDS+双向帧，但 HTTP 升级/路由/shutdown 都要自写；与已用 axum（内部已依赖 tokio-tungstenite，见 [crates.io axum `ws` feature](https://crates.io/crates/axum)）重复劳动。

---

## 5. jsonrpsee（JSON-RPC 2.0 over HTTP/WS）

- **运行时**：README「async/await」；server 示例 `tokio::net::TcpListener` + `tokio::spawn`。[README](https://raw.githubusercontent.com/paritytech/jsonrpsee/master/README.md) · [ServerBuilder](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/struct.ServerBuilder.html)
- **WebSocket**：crate/README 一等「Client/server WebSocket」。`TowerService` 可 upgrade。`SubscriptionSink::{send, send_timeout, try_send}` **写明** wait-until-capacity / channel full（官方背压）。[jsonrpsee-server](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/) · [SubscriptionSink](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/struct.SubscriptionSink.html)
- **Unix domain**：`ServerBuilder::build(ToSocketAddrs)` / `build_from_tcp`；**无** `bind_uds`。低层 `serve`/`serve_with_graceful_shutdown` 的 `I: AsyncRead+AsyncWrite` 可塞 `UnixStream`，但 README **无 UDS 示例**。[build](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/struct.ServerBuilder.html) · [serve](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/fn.serve.html)
- **序列化**：JSON-RPC 2.0 对象（`jsonrpsee-types` request/response/params），**不是** 自由 tagged `Command`/`Event`。[types](https://docs.rs/jsonrpsee-types/latest/jsonrpsee_types/)
- **流式**：subscription 一等；不是裸 TextDelta 帧。
- **浏览器**：服务端是 WS，浏览器原生 WS **能连**，但载荷必须是 JSON-RPC。另有 `wasm-client`（web-sys）走客户端。[crate features](https://docs.rs/jsonrpsee/latest/jsonrpsee/)
- **与现有协议**：**必须换词汇**（method/id/jsonrpc vs `type` tag）。反向 ask 不是 RpcModule 一等（那是 server methods）；订阅是 server→client 通知。
- **feature 体积**：无 default features；`server`/`ws-client` 等需显式开。未测。
- **一句话：否决（换协议）** — 除非放弃现有 Command/Event。[README](https://raw.githubusercontent.com/paritytech/jsonrpsee/master/README.md)

---

## 6. tonic / prost（gRPC + streaming）

- **运行时**：「based on tokio, hyper and tower」。[tonic crate](https://docs.rs/tonic/latest/tonic/) · [README](https://raw.githubusercontent.com/hyperium/tonic/master/README.md)
- **WebSocket**：gRPC over HTTP/2，**不是** 浏览器 WS。`tonic-web`：**「There is no support for web socket transports」**；grpc-web 客户端目前仅 unary + server-streaming，**无** client/bidi streaming。[tonic-web](https://docs.rs/tonic-web/latest/tonic_web/)
- **Unix domain**：官方 example `UnixListener` + `UnixListenerStream` + `serve_with_incoming`。[uds/server.rs v0.14.x](https://raw.githubusercontent.com/hyperium/tonic/v0.14.x/examples/src/uds/server.rs) · [serve_with_incoming](https://docs.rs/tonic/latest/tonic/transport/server/struct.Server.html)
- **序列化**：codegen + prost/protobuf；`Codec` 可换，默认不是 serde tagged JSON。[codec](https://docs.rs/tonic/latest/tonic/codec/index.html)
- **流式**：`Streaming<T>` 与 README「Bi-directional streaming」一等；HTTP/2 window 有官方流控项（`initial_stream_window_size` 等）。
- **浏览器**：不能原生 WS；要 grpc-web + `accept_http1(true)`，且无 bidi。[accept_http1](https://docs.rs/tonic/latest/tonic/transport/server/struct.Server.html)
- **与现有协议**：**必须换** protobuf RPC 词汇。
- **feature 体积**：`transport`/`router`/`codegen` 默认开。未测。
- **graceful**：`serve_with_shutdown` / `serve_with_incoming_shutdown`。
- **一句话：否决** — schema/浏览器/WS 三条都不贴本负载。[README](https://raw.githubusercontent.com/hyperium/tonic/master/README.md)

---

## 7. poem / salvo（仅当官方 WS+UDS）

二者官方都同时有 **websocket feature** 与 **UnixListener**：

- **poem 3.1**：feature `websocket`；`poem::listener::UnixListener`；`web::websocket` 双向 `Stream`+`Sink`。tokio。[UnixListener](https://docs.rs/poem/latest/poem/listener/struct.UnixListener.html) · [websocket](https://docs.rs/poem/latest/poem/web/websocket/index.html) · [features](https://docs.rs/poem/latest/poem/)
- **salvo 0.95**：features `unix` + `websocket`（均非 default）；示例 `new WebSocket(...)`（浏览器）+ `UnixListener`。[salvo features](https://docs.rs/salvo/latest/salvo/) · [websocket](https://docs.rs/salvo/latest/salvo/websocket/index.html) · [UnixListener](https://docs.rs/salvo/latest/salvo/conn/unix/struct.UnixListener.html)

相对 axum 无本仓已依赖优势。**一句话：不扩用（能打但不取代 axum）。**

---

## 对照（上游能力，非产品拍板）

| 需求 | axum | actix | warp 0.4 | hyper+tt | jsonrpsee | tonic |
|---|---|---|---|---|---|---|
| 共用 tokio | 是 | 是（另有 worker 线程） | 是 | 是 | 是 | 是 |
| 官方 UDS listener | 是 | `bind_uds` | 0.3 有字 / 0.4 无 | 自接 | 无一等 | `serve_with_incoming` |
| 浏览器原生 WS | 是 | 是 | 是 | 自写 upgrade | 是（载荷变） | 否（grpc-web；无 WS） |
| 保留 tagged JSON | 是 | 是 | 是 | 是 | 否 | 否 |
| 官方背压说法 | Sink only | 无 | Sink only | Sink | subscription channel | HTTP/2 window |

**本机默认载体**：上游允许 **axum::serve(`UnixListener`) 上跑现有 HTTP+WS**，协议不变。TCP `127.0.0.1` 与 UDS 是同一 Router 换 listener。jsonrpsee/tonic 要换 envelope，不满足「只缺载体」。
