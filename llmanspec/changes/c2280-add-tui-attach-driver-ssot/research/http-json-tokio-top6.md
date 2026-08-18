# HTTP JSON / JSON-RPC：GitHub ≥4000 star 闸 + tokio 最熟的 6 个

> 星标以 `gh api` / `gh search` 在 **2026-08-18** 拉的为准。只评 **服务端能收发 JSON** 的 crate。不评 WASM 前端（Yew / Dioxus / Leptos）。不写实现。

## 先回答你的问题

只认 **JSON-RPC 2.0** 或 **HTTP JSON**（含 WebSocket 文本帧）时，候选面 **比 tonic / postcard / 裸长度前缀更宽**。

原因很简单：几乎每个 Rust HTTP 框架都能 `POST` 一段 JSON、再 upgrade 成 WebSocket 推 JSON。你们现在的 `Command` / `Event` 已经是 `serde` 打了 `type` 标签的 JSON（tagged JSON）。换 HTTP 栈 = 换路由和 listener，**wire protocol 可以不动**。

JSON-RPC 是另一种信封：

```json
{"jsonrpc":"2.0","method":"prompt","params":{"message":"fix the bug"},"id":"1"}
```

对照现在：

```json
{"type":"prompt","message":"fix the bug"}
```

信封换了，消息种类还是那些。**没有**一个 ≥4000 star 的 Rust 库专门做 JSON-RPC 服务端：[`paritytech/jsonrpsee`](https://github.com/paritytech/jsonrpsee) 只有 **851** 星。要 JSON-RPC，等于在 axum/actix 上自己包一层，或接受这个小 crate。

tonic / protobuf **不进**本闸（那是另一套说明书）。ntex **2528** 星，不够 4000。

## 闸怎么滤的

GitHub `language:Rust` `stars:>=4000` 有上百个仓，绝大多数是编辑器、CLI、runtime，不是 HTTP 框架。

下面是 **明确在做 HTTP 服务、或就是 HTTP 库**、且 ≥4000 的仓（同日快照）：

| 仓 | 星 | 算框架吗 | tokio | 对本题 |
|---|---:|---|---|---|
| [tokio-rs/axum](https://github.com/tokio-rs/axum) | 26900 | 是（路由层） | 官方，tokio-rs 自家 | HTTP JSON / WS |
| [rwf2/Rocket](https://github.com/rwf2/Rocket) | 25774 | 是 | 0.5 起 tokio | HTTP JSON / `rocket_ws` |
| [actix/actix-web](https://github.com/actix/actix-web) | 24784 | 是 | README：Full Tokio compatibility | HTTP JSON / WS / `bind_uds` |
| [hyperium/hyper](https://github.com/hyperium/hyper) | 16276 | **库**，不是应用框架 | 是 | axum/warp/salvo 坐在它上面 |
| [seanmonstar/warp](https://github.com/seanmonstar/warp) | 10360 | 是 | 是 | HTTP JSON / `warp::ws()` |
| [http-rs/tide](https://github.com/http-rs/tide) | 5094 | 是 | 历史主线 async-std | **最后 push 2024-01-05**，当休眠 |
| [poem-web/poem](https://github.com/poem-web/poem) | 4433 | 是 | 是 | HTTP JSON / WS / UnixListener |
| [salvo-rs/salvo](https://github.com/salvo-rs/salvo) | 4422 | 是 | 是（Hyper + Tokio） | HTTP JSON / WS / UnixListener |
| [tower-rs/tower](https://github.com/tower-rs/tower) | 4280 | 中间件抽象 | 是 | 不是路由器 |

**≥4000 但不进「CS host 框架」名单：**

| 仓 | 星 | 为什么剔掉 |
|---|---:|---|
| [cloudflare/pingora](https://github.com/cloudflare/pingora) | 27236 | 反向代理 / 网关，不是 agent 的 Command 泵 |
| [DioxusLabs/dioxus](https://github.com/DioxusLabs/dioxus) | 38788 | 前端 / 全栈 UI |
| [yewstack/yew](https://github.com/yewstack/yew) | 32783 | WASM UI |
| [leptos-rs/leptos](https://github.com/leptos-rs/leptos) | 21211 | WASM UI |
| [grpc/grpc-rust](https://github.com/grpc/grpc-rust)（tonic） | 12432 | gRPC + protobuf，不是 HTTP JSON |
| [actix/actix](https://github.com/actix/actix) | 9239 | actor runtime，不是 HTTP 路由器 |
| [shuttle-hq/shuttle](https://github.com/shuttle-hq/shuttle) | 6927 | 部署平台，底下仍要一个 web crate |

**想进名单但不够 4000：** ntex 2528；jsonrpsee 851；tokio-tungstenite 2494。

## 「tokio 最熟」的 6 个怎么取的

熟悉 = tokio 教程 / tokio-rs org / 写 HTTP 服务的人张口就来的名字。star 只做门槛，不做排名。

入选：**axum、Rocket、actix-web、warp、poem、salvo**。

tide 星比 poem 多，但停更 + 不是 tokio 默认心智，不进这 6 个。hyper 人人都熟，但是库，放在对照列，不当第 7 个产品壳。

下面这 6 个对 xylitol 的差别，几乎全在门口（UDS、双听、WS 推流 API），**不在** token 序列化。热路径仍是 `XyEvent → serde → 帧`，见 [`transport.md`](./transport.md)。

### 1. axum（tokio-rs）

- **熟**：tokio 官方栈。本仓 `feature = "server"` 已经在用 0.8 + `ws`。
- **能**：`Json<T>`；`WebSocketUpgrade`；官方 [unix-domain-socket 示例](https://github.com/tokio-rs/axum/blob/main/examples/unix-domain-socket/src/main.rs)；`axum::serve(listener, app)` 的 listener 可以是 `tokio::net::UnixListener`。
- **对 CS agent**：REST 命令 + WS 日志/反向 RPC，和现在 `src/app/server/{rest,ws}.rs` 同构。换它 = 零协议税。
- **短处**：WS `on_upgrade` 会 `tokio::spawn` 出去，graceful shutdown **不等**这条流（[源码注释](https://github.com/tokio-rs/axum/blob/main/src/extract/ws.rs)）。停 host 时 attach 窗要自己收摊。没有 JSON-RPC 方法表。
- **优劣一句话**：门口我们已经会开；JSON 闸下它不输，也不因为 star 就该锁死。

### 2. Rocket 0.5

- **熟**：Rust web 课里出现最早的名字之一。0.5 才切到 tokio。
- **能**：JSON；[`rocket_ws`](https://api.rocket.rs/master/rocket_ws/) 的 `Stream!` / `channel`；Unix listener 文档：[UnixListener](https://api.rocket.rs/master/rocket/listener/unix/struct.UnixListener.html)；发送队列有 `max_send_queue`。
- **对 CS agent**：token 泵很像「往一个 channel 里塞 Event」。背压旋钮比 axum 直白。
- **短处**：fairing / 状态模型和现有 axum handler 对不上，迁移面大。宏多。最后一次 GitHub push 相对这 6 个里偏静（2025-12-28）。
- **优劣一句话**：推流 API 好看；为 attach 换全家桶不划算，除非产品就想「Event 像 generator」。

### 3. actix-web 4

- **熟**：性能榜常客。星 24784。
- **能**：官方 [`HttpServer::bind_uds`](https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.bind_uds) / `listen_uds`；JSON；WebSocket；HTTP/2 窗口可调；`shutdown_signal` 可接 `CancellationToken`。README 写 Full Tokio compatibility。
- **对 CS agent**：UDS 是一等 API，比 warp 省事。
- **短处**：worker 默认按核数拉线程，个人 host 1～3 条连接是浪费。WS 常用 actor（[`actix`](https://github.com/actix/actix)），和现有 `ReverseRpcGateway` 的 oneshot map 两套模型。`run()` 在没 tokio runtime 时会 panic。
- **优劣一句话**：UDS 文档最硬；runtime/actor 税对「一个本机 host」偏重。

### 4. warp

- **熟**：axum 之前 tokio 世界的默认路由器（seanmonstar，和 hyper/reqwest 一家）。
- **能**：Filter 组合；`warp::ws()`；JSON。
- **短处**：README **没有**一等 UDS。本机 sock 要自己把 `UnixListener` 接到 hyper。2026 还在推（2026-07-28），但新项目教程已经转向 axum。
- **优劣一句话**：WS 熟；本机 Unix 插座要手接线。JSON 闸下能用，不是 CS host 首选。

### 5. poem

- **熟**：tokio 圈第二梯队，OpenAPI / 全家桶宣传多。刚过 4000。
- **能**：UnixListener + 权限/owner；一进程 combine 双听；WS；REST 可切 sonic-rs（官网 README）。tokio。
- **对 CS agent**：本机 sock 权限、UDS+TCP 双听是真需求。sonic-rs 碰的是 REST JSON，WS 帧仍可自选 serde。
- **短处**：生态比 axum 小；本仓零使用。issue 数量相对 star 偏高（仓页 191 open，同日）。
- **优劣一句话**：从零做本机 host，UDS 故事完整；不是「更熟」而是「门口更贴 Unix」。

### 6. salvo

- **熟**：和 poem 几乎同星、同期。宣传 HTTP/3、WebTransport、OpenAPI。
- **能**：`conn::unix::UnixListener`；`JoinedListener` 双听；`WebSocketUpgrade`；建立在 Hyper + Tokio 上（[README](https://github.com/salvo-rs/salvo)）。
- **对 CS agent**：和 poem 同一档：双听写得死。HTTP/3 对「本机 attach、长期不做 Web」用不上。
- **短处**：同样本仓零使用。HTTP/3 会把心智拉去远程网关。
- **优劣一句话**：能力表漂亮；和 poem 二选一即可，不必两个都引进仓。

## JSON-RPC 会不会让名单更长？

**不会变长，只会换信封。**

≥4000 的 6 个都能扛 JSON-RPC：**你自己**把 `{"jsonrpc":"2.0",...}` 解析成现在的 `Command`。没有第六个「JSON-RPC 框架」跨过星标闸。

jsonrpsee（851）提供方法表、subscription、HTTP+WS 服务端。DSH 那边 SDK 更像这种。代价：词汇从「闭集 enum」变成「字符串 method」；server→client 的 `ApproveTool` 要扭曲成反向调用或 notification。本仓已经有 tagged JSON + 自写 `ReverseRpcGateway`（`src/app/server/ws.rs`）。

## 和 xylitol 热路径的关系（别被 RPS 骗）

模型吐字时走的是 WS/`Event` 帧，不是 HTTP `Json` extractor。6 个框架在这条路上几乎同构：upgrade 之后都是 `serde` + `send`。

换框架 **解决不了**：

- `XyRemoteDriver::execute_bash` 丢掉 `chunk_tx`（REST 一问一答，bang 没有直播输出）
- journal clone、`Mutex<XyInProcessDriver>`
- 每枚 `TextDelta` 的 JSON 分配

换框架 **能**解决的只有：听 Unix 插座好不好写、停进程时 WS 任务收不收得干净、以后要不要同一套 HTTP 给浏览器。

长期不做 Web 时，HTTP 本身是税。JSON 闸的意义是：**先把 wire protocol 定成 tagged JSON，HTTP 栈在这 6 个里挑**；不要为了「框架更快」去 tonic。


## 不在这 6 个里、但调研里常被点名的

| 名字 | 星 | 一句话 |
|---|---:|---|
| ntex | 2528 | 更像网络服务（codec/ws/`bind_uds`），当前线程 runtime，不够闸 |
| jsonrpsee | 851 | JSON-RPC 专家，不够闸 |
| tonic | 12432 | 流式最好看，不是 JSON |
| tide | 5094 | 过闸但休眠 |

## 本篇不钉死的选型

JSON 闸下，**没有**「必须换掉 axum」的一手证据。也没有「必须留 axum」的证据——poem/salvo/rocket_ws 的 UDS/推流在文档上更贴本机 host。

**本轮已钉**（见 [`http-stack-pick.md`](./http-stack-pick.md)）：仍用 tagged JSON；HTTP 栈选 **axum**；listener 按拓扑换 Unix / TCP。另五个不换的原因写在那篇，不是因为「已经在用」。
