# 上游：只允许 JSON-RPC 2.0 或 HTTP+JSON 时，哪些 Rust 服务端库够格

> 调研日：2026-08-18。星数来自 GitHub REST `stargazers_count`。版本来自 crates.io API / GitHub Releases。每条事实带 URL。不选赢家。
>
> **约束怎么读**：合格 = 能当 **JSON-RPC 2.0 服务端**，或能当 **HTTP 服务端并吃/吐 JSON**（含 WebSocket 文本帧上的 JSON）。gRPC/protobuf 默认出局，除非官方有 JSON 转码故事。

## 一句话地图

在「≥4000 星」门槛下：**有一堆 HTTP+JSON 框架**；**没有**达到门槛的专用 JSON-RPC 2.0 服务端。名单里唯一内置 JSON-RPC 2.0 的是 `jsonrpsee`（851 星，见跳过表）。`tonic` 是 gRPC，官方 `tonic-web` 只做 grpc-web，不是 JSON 转码，按本约束 **出局**。`tokio` 是运行时，`tower` 是中间件抽象，都不是 Web 框架。

「tokio 生态最熟」大致分成两拨：

- **坐在 tokio + hyper（常加 tower）上**：axum、warp、poem、salvo、Rocket 0.5、jsonrpsee、tonic、hyper 本身。
- **自带或可选别的运行时**：actix-web（官方写 Full Tokio 兼容，但仍是自己的 `HttpServer`/worker，不是 tower 路由栈）；ntex（自有 `ntex::rt`，可开 `tokio` feature）；tide（async-std）；iron（老 hyper、同步 `.http()`）。

## 门槛与跳过

GitHub 搜 `topic:web-framework` / `topic:rpc` + language Rust（[web-framework](https://github.com/search?q=topic%3Aweb-framework+language%3ARust&type=repositories)、[rpc](https://github.com/search?q=topic%3Arpc+language%3ARust+stars%3A%3E4000&type=repositories)）。另核：`viz-rs/vidi` 379、`Xudong-Huang/may_minihttp` 923、`nickel-org/nickel.rs` 3051。`cloudflare/pingora` 27236 是网络服务库不是 HTTP 应用框架（[README](https://github.com/cloudflare/pingora)）。`yewstack/yew` 是 WASM 前端。额外列入且 ≥4000 的 HTTP 框架：**iron**。

| 仓库 | 星 | 为何跳过 / 仍记一行 |
|---|---|---|
| [ntex-rs/ntex](https://github.com/ntex-rs/ntex) | 2528 | <4000。自有 runtime，feature `tokio` 可选（[Cargo.toml](https://github.com/ntex-rs/ntex/blob/main/ntex/Cargo.toml)、[ntex::rt](https://docs.rs/ntex/latest/ntex/rt/index.html)）。crates.io `ntex` 3.12.3（2026-08-07），该版 `rust_version` **1.88**；git `ntex/Cargo.toml` 现为 **1.97**。 |
| [paritytech/jsonrpsee](https://github.com/paritytech/jsonrpsee) | 851 | <4000，但是名单里 **唯一内置 JSON-RPC 2.0** 的服务端。见下节短表。 |
| [viz-rs/vidi](https://github.com/viz-rs/vidi) | 379 | 原 viz HTTP 框架改名；<4000。crates.io 仍有 `viz` 0.11.0，仓库已不是框架。 |
| [Xudong-Huang/may_minihttp](https://github.com/Xudong-Huang/may_minihttp) | 923 | <4000。 |

## 总表（≥4000，加名单点名项）

列含义：tokio = 官方文档/README/Cargo.toml 写明用 tokio。UDS = 官方有 Unix domain socket **listen** API 或官方示例。WS = 官方 WebSocket **服务端**。JSON body = 官方提取/反序列化 JSON 请求体。JSON-RPC 2.0 = 框架自己实现该协议，不是「你自己解析 JSON」。

| Crate | 星 | 角色 | tokio | UDS listen | WS 服务端 | JSON body | JSON-RPC 2.0 内置 | 最近发布 | 声明的 MSRV |
|---|---|---|---|---|---|---|---|---|---|
| [tokio-rs/axum](https://github.com/tokio-rs/axum) | 26900 | HTTP 路由库 | 是（[docs Compatibility](https://docs.rs/axum/latest/axum/)） | 是（[unix 示例](https://github.com/tokio-rs/axum/blob/main/examples/unix-domain-socket/src/main.rs)、[`Listener` for UnixListener](https://github.com/tokio-rs/axum/blob/main/axum/src/serve/listener.rs)） | 是，feature `ws`（[extract::ws](https://docs.rs/axum/latest/axum/extract/ws/index.html)） | 是，`Json`（[crate 首页](https://docs.rs/axum/latest/axum/)） | 否 | crates.io **0.8.9** / GitHub `axum-v0.8.9` 2026-04-14（[crates.io](https://crates.io/crates/axum)、[release](https://github.com/tokio-rs/axum/releases/tag/axum-v0.8.9)） | **1.80**（[README](https://github.com/tokio-rs/axum/blob/main/README.md)、[workspace Cargo.toml](https://github.com/tokio-rs/axum/blob/main/Cargo.toml)） |
| [actix/actix-web](https://github.com/actix/actix-web) | 24784 | HTTP 框架 | 是，「Full Tokio compatibility」；`#[actix_web::main]` 或 `#[tokio::main]`（[docs.rs](https://docs.rs/actix-web/latest/actix_web/)、[README](https://github.com/actix/actix-web)） | 是，`HttpServer::bind_uds` / `listen_uds`（[HttpServer](https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html)） | 是（[README](https://github.com/actix/actix-web)、[指南](https://actix.rs/docs/websockets/)） | 是，`web::Json`（[Json](https://docs.rs/actix-web/latest/actix_web/web/struct.Json.html)） | 否 | crates.io **4.14.1** 2026-08-09（[crates.io](https://crates.io/crates/actix-web)、[tag](https://github.com/actix/actix-web/releases/tag/web-v4.14.1)） | **1.88+**（[docs Features](https://docs.rs/actix-web/latest/actix_web/)、[README badge](https://github.com/actix/actix-web)） |
| [rwf2/Rocket](https://github.com/rwf2/Rocket) | 25774 | HTTP 框架 | 是：`rocket::execute` 为 tokio runtime；依赖 tokio/hyper（[docs.rs](https://docs.rs/rocket/latest/rocket/)、[core Cargo.toml](https://github.com/rwf2/Rocket/blob/master/core/lib/Cargo.toml)） | 是：源码 `listener/unix.rs`「Unix domain sockets listener」（[unix.rs](https://github.com/rwf2/Rocket/blob/master/core/lib/src/listener/unix.rs)） | 核心 crate 无 WS；官方 contrib **`rocket_ws` 0.1.1**（[crates.io](https://crates.io/crates/rocket_ws)） | 是，feature `json`（[docs.rs Features](https://docs.rs/rocket/latest/rocket/)） | 否 | crates.io **0.5.1** 2024-05-23（[crates.io](https://crates.io/crates/rocket)、[v0.5.1](https://github.com/rwf2/Rocket/releases/tag/v0.5.1)） | 已发布 0.5.1：crates.io `rust_version` **1.64**；git `core/lib` 现为 **1.75**（[Cargo.toml](https://github.com/rwf2/Rocket/blob/master/core/lib/Cargo.toml)）。`rocket_ws` 也写 1.64 |
| [seanmonstar/warp](https://github.com/seanmonstar/warp) | 10360 | HTTP 框架 | 是（[README](https://raw.githubusercontent.com/seanmonstar/warp/master/README.md) 要求 tokio） | 是（[unix_socket.rs](https://github.com/seanmonstar/warp/blob/master/examples/unix_socket.rs)） | 是（[filters::ws](https://docs.rs/warp/latest/warp/filters/ws/index.html)） | 是，`body::json`（[json()](https://docs.rs/warp/latest/warp/filters/body/fn.json.html)） | 否 | crates.io **0.4.3** 2026-05-04（[crates.io](https://crates.io/crates/warp)）；GitHub tag `v0.4.3` | README/Cargo.toml **未写** rust-version |
| [poem-web/poem](https://github.com/poem-web/poem) | 4433 | HTTP 框架 | 是，`#[tokio::main]` 示例（[docs.rs](https://docs.rs/poem/latest/poem/)） | 是，`listener::UnixListener`（[UnixListener](https://docs.rs/poem/latest/poem/listener/struct.UnixListener.html)） | 是，feature `websocket`（[web::websocket](https://docs.rs/poem/latest/poem/web/websocket/index.html)） | 是，`web::Json`（[Json](https://docs.rs/poem/latest/poem/web/struct.Json.html)） | 否 | crates.io **3.1.12** 2025-07-28（[crates.io](https://crates.io/crates/poem)） | workspace **1.85**（[Cargo.toml](https://github.com/poem-web/poem/blob/master/Cargo.toml)） |
| [salvo-rs/salvo](https://github.com/salvo-rs/salvo) | 4422 | HTTP 框架 | 是，示例 `#[tokio::main]`（[websocket 模块](https://docs.rs/salvo/latest/salvo/websocket/index.html)） | 是，feature `unix`（默认关）（[feature 表](https://docs.rs/salvo/latest/salvo/)）；源码 `conn/unix.rs` | 是，feature `websocket`（默认关）（同上） | 是，`Request::parse_json`（[salvo_core Request](https://docs.rs/salvo_core/latest/salvo_core/http/request/struct.Request.html)） | 否 | crates.io **0.95.2** 2026-08-06（[crates.io](https://crates.io/crates/salvo)、[tag](https://github.com/salvo-rs/salvo/releases/tag/v0.95.2)） | workspace **1.94**（[Cargo.toml](https://github.com/salvo-rs/salvo/blob/main/Cargo.toml)） |
| [http-rs/tide](https://github.com/http-rs/tide) | 5094 | HTTP 框架 | **否**：官方示例 `async-std`（[docs.rs](https://docs.rs/tide/latest/tide/)） | 是：listener 文档写 unix sockets；源码 `http+unix://`（[listener](https://docs.rs/tide/latest/tide/listener/index.html)、[to_listener.rs](https://github.com/http-rs/tide/blob/main/src/listener/to_listener.rs)） | 核心 crate 模块列表 **无** websocket（[docs.rs](https://docs.rs/tide/latest/tide/)）；有 SSE | 是，`Request::body_json`（[首页示例](https://docs.rs/tide/latest/tide/)） | 否 | 稳定 **0.16.0**；newest `0.17.0-beta.1` 2021-12-06（[crates.io](https://crates.io/crates/tide)）。仓库 last push 2024-01-05 | 未声明；edition 2018（[Cargo.toml](https://github.com/http-rs/tide/blob/main/Cargo.toml)） |
| [hyperium/hyper](https://github.com/hyperium/hyper) | 16276 | **HTTP 库，不是框架** | 依赖 tokio（[Cargo.toml](https://github.com/hyperium/hyper/blob/master/Cargo.toml)）；文档强调 runtime 组件在 `hyper::rt`（[docs.rs](https://docs.rs/hyper/latest/hyper/)） | 库本身不 `bind`；可把 `tokio::net::UnixStream` 交给 hyper 当 IO（UnixListener：[tokio](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html)） | 无 WS 协议实现；有 HTTP Upgrade（[upgrade](https://docs.rs/hyper/latest/hyper/upgrade/index.html)） | 无 JSON extractor；自己读 body 再 `serde_json` | 否 | crates.io **1.11.0** 2026-07-20（[crates.io](https://crates.io/crates/hyper)、[v1.11.0](https://github.com/hyperium/hyper/releases/tag/v1.11.0)） | **1.63**（[Cargo.toml](https://github.com/hyperium/hyper/blob/master/Cargo.toml) `rust-version`） |
| [iron/iron](https://github.com/iron/iron) | 6112 | HTTP 框架（额外 ≥4000） | 否。Hello World 同步 `.http("localhost:3000")`（[docs.rs 0.6.1](https://docs.rs/iron/0.6.1/iron/)） | 文档未提供 UDS bind | 核心「不捆绑」插件；无 WS 模块（[docs.rs](https://docs.rs/iron/0.6.1/iron/)） | 核心无 JSON extractor（「No plugins … are bundled」） | 否 | crates.io **0.6.1** 2019-08-14（[crates.io](https://crates.io/crates/iron)） | 未声明 |
| [grpc/grpc-rust](https://github.com/grpc/grpc-rust)（tonic） | 12432 | **gRPC** | 是（[docs.rs tonic](https://docs.rs/tonic/latest/tonic/) transport = hyper+tower+tokio） | 有 UDS 示例（搜到 `examples/src/uds/server.rs` + `UnixListenerStream`） | 否（gRPC 流 ≠ JSON WS） | protobuf codec，不是 JSON body | **否。出局**（见 tonic 节） | crates.io **tonic 0.14.6** 2026-05-07（[crates.io](https://crates.io/crates/tonic)） | **1.88**（[README](https://github.com/grpc/grpc-rust/blob/master/README.md)、[workspace](https://github.com/grpc/grpc-rust/blob/master/Cargo.toml)） |
| [tokio-rs/tokio](https://github.com/tokio-rs/tokio) | 32944 | **运行时，不是 Web 框架** | — | `tokio::net::UnixListener` 有（[docs](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html)） | 无 | 无 | 无 | **1.53.1** 2026-07-20（[crates.io](https://crates.io/crates/tokio)） | **1.71**（[tokio/Cargo.toml](https://github.com/tokio-rs/tokio/blob/master/tokio/Cargo.toml)） |
| [tower-rs/tower](https://github.com/tower-rs/tower) | 4280 | 中间件：`Service`/`Layer` | 部分 feature 依赖 tokio（[tower/Cargo.toml](https://github.com/tower-rs/tower/blob/master/tower/Cargo.toml)） | 无 listen | 无 | 无 | 无 | **0.5.3** 2026-01-12（[crates.io](https://crates.io/crates/tower)） | **1.64.0**（[docs.rs](https://docs.rs/tower/latest/tower/)「current MSRV is 1.64.0」） |

### jsonrpsee（<4000，点名补全）

| 项 | 事实 |
|---|---|
| 仓库 / 星 | [paritytech/jsonrpsee](https://github.com/paritytech/jsonrpsee) **851**（仍是 Parity 组织，未见换主） |
| 是什么 | 「JSON-RPC library designed for async/await」；HTTP 与 WebSocket 传输（[README](https://github.com/paritytech/jsonrpsee/blob/master/README.md)） |
| JSON-RPC 2.0 | 请求类型字段 `jsonrpc: TwoPointZero`，「JSON-RPC request object as defined in the spec」（[Request](https://docs.rs/jsonrpsee-types/latest/jsonrpsee_types/request/struct.Request.html)）；协议规范 [jsonrpc.org](https://www.jsonrpc.org/specification) |
| tokio / tower / hyper | 是。`ServerBuilder::build` 示例 `#[tokio::main]`；`set_http_middleware` 吃 `tower::ServiceBuilder`；有 `TowerService`（[ServerBuilder](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/struct.ServerBuilder.html)） |
| UDS | **无一等 API**。`build(addrs: impl ToSocketAddrs)`、`build_from_tcp`；`serve*` 文档写 TCP（[jsonrpsee-server](https://docs.rs/jsonrpsee-server/latest/jsonrpsee_server/)）。`to_service_builder()` 可接到自写 accept 循环，官方示例是 `TcpListener` |
| WS | 是（README + server `ws` 模块） |
| 发布 / MSRV | GitHub `v0.26.0` 2025-08-11；crates.io `max_stable_version` 0.26.0，crate `updated_at` 2026-05-27（[crates.io](https://crates.io/crates/jsonrpsee)）。workspace `rust-version = "1.85.0"`（[Cargo.toml](https://github.com/paritytech/jsonrpsee/blob/master/Cargo.toml)） |

## tonic：为何出局

tonic 自称「gRPC over HTTP/2」（[docs.rs](https://docs.rs/tonic/latest/tonic/)、[README](https://github.com/grpc/grpc-rust/blob/master/README.md)）。GitHub 请求 `hyperium/tonic` 会转到 [`grpc/grpc-rust`](https://github.com/grpc/grpc-rust)；crates.io `repository` 仍写 `https://github.com/hyperium/tonic`。

官方和 JSON 最接近的是 **`tonic-web`**：做 **grpc-web 协议翻译**，并写明「not expected to handle arbitrary HTTP/x.x requests」；「There is no support for web socket transports」（[tonic-web](https://docs.rs/tonic-web/latest/tonic_web/)）。这不是 protobuf HTTP/JSON transcoding（那种是另一套 Google 网关故事，tonic 文档未提供）。按本文件约束：**OUT**。

## 按约束谁合格（不排名）

**合格（HTTP+JSON 服务端）**：axum、actix-web、Rocket、warp、poem、salvo、tide、iron（铁已多年不发稳定版）、hyper（自己拼 JSON）。WS 文本帧：axum/actix/warp/poem/salvo/Rocket(+`rocket_ws`) 有官方路径；tide 核心无 WS。

**合格（JSON-RPC 2.0）**：只有 jsonrpsee（星不够门槛）。上述 HTTP 框架都能 **自己** 解析 JSON-RPC 正文，但那不是「内置」。

**不合格**：tonic（gRPC）；tokio、tower（不是 HTTP/RPC 服务器）。

**本机 UDS 上跑 HTTP/WS（不换协议）**：官方有 listen 的 ≥4000 项包括 axum、actix-web、Rocket、warp、poem、salvo（feature）、tide。jsonrpsee 要自己把 `TowerService` 接到 Unix 流，文档没提供。

## tokio / tower / hyper 亲疏

| 坐在 tokio+hyper（多为 tower `Service`） | 说明 |
|---|---|
| axum | 「designed to work with tokio and hyper」；中间件用 `tower::Service`（[axum](https://docs.rs/axum/latest/axum/)） |
| warp | 「builds on top of hyper」（[docs.rs](https://docs.rs/warp/latest/warp/)）；README 加 tokio |
| poem / salvo | Cargo.toml / 文档：hyper + tokio；tower 为可选 compat feature |
| Rocket 0.5 | hyper 1 + tokio（[core Cargo.toml](https://github.com/rwf2/Rocket/blob/master/core/lib/Cargo.toml)） |
| jsonrpsee | tower HTTP 中间件 + hyper-util 示例 |
| tonic | 「built on top of tokio, hyper and tower」（[docs.rs](https://docs.rs/tonic/latest/tonic/)）— 但协议出局 |
| hyper / tower | 栈的底层，不是应用框架 |

| 自有或非该栈 | 说明 |
|---|---|
| actix-web | 现官方 Tokio 兼容，但应用 API 是 `App`/`HttpServer`，不是 axum 那种 tower 路由器。历史上 actix-rt 演员运行时。 |
| ntex（跳过） | `ntex::rt` + 可选 `rt_tokio`（[rt](https://docs.rs/ntex/latest/ntex/rt/index.html)） |
| tide | async-std |
| iron | 旧 hyper 同步服务器 |

## xylitol 还开着的问题

- 若本机默认载体是 **UDS 上的现有 HTTP+JSON/WS（不换 JSON-RPC）**：axum 官方 Unix 示例已经覆盖「听 UDS」；还要不要为了 JSON-RPC 2.0 形状去碰不够星的 jsonrpsee，还是继续自有 `Command`/`Event`？
- jsonrpsee 的 `TowerService` 能否干净接到 **已有 axum UDS 监听**（官方只示范 TCP）而不变成两套协议？
- 未来浏览器只要求「别选一个永远接不上的唯一传输」：tide/iron 缺一等 WS、tonic 出局，是否把「必须有官方 WS」当成 Web 同构的硬过滤器？
