# Rust 服务端框架：从零对照（忽略早先 C2 /「已有 axum」）

> 不把「本仓已经 axum」和 proposal 里早年的 C2 预置当结论。axum 只作为**现状事实**出现在迁移成本列。
> 负载：个人 coding agent host——连接少、**单连接 token 级小 JSON 很密**、serde 往返多、要双向流（含 server→client 反向 RPC）、本机 UDS、未来浏览器还要能连。
> 前端壳（React+Vite）见 `web-framework.md`，本文只评 **Rust 服务端**。

## 1. 热路径（先于任何框架）

```text
XyEvent → to_wire_event → serde_* → 套接字帧 → N 个订阅者
```

REST `Json` extractor **不在**这条路上。HTTP 路由器的 RPS 榜对 token 流几乎无意义。框架只决定：套接字怎么来、帧模型是 WS / gRPC / 自写、背压旋钮在哪、runtime 是否是 tokio。

## 2. 四条架构（先选这条，再选 crate）

| 架构 | 流模型 | 序列化 | 浏览器 | 反向 RPC | 适合何时 |
|---|---|---|---|---|---|
| **A. HTTP 升级 → 双向 JSON 帧** | WS text；自己 loop 推 Event | 现有 `#[serde(tag)]` | 原生 `WebSocket` | 同一连接上回帧 | **要保留 Command/Event + 浏览器** |
| **B. 每连接 Service/Actor** | mailbox / service factory 收发 Frame | 仍可以 JSON | 原生 WS（经 HTTP 升级） | mailbox 天然对 ask | 把「审批/问卷」做成连接对象，而不是全局 oneshot map |
| **C. 类型化 bidi RPC** | gRPC stream / tarpc | protobuf 或另 schem | tonic 要 grpc-web | 一流二向最干净 | **允许换协议** |
| **D. 裸 UDS 帧** | 长度前缀或 tungstenite 无 HTTP | JSON 或 bincode | **没有**，Web 必须第二套 | 自写 | 只优化本机 attach |

产品文档（同源、远程薄端、Cloud-Agent TS 壳）在 **未改产品决策** 时把你钉在 A 或 B。C/D 是「换协议 / 本机与 Web 分载体」的显式决策，不是框架快慢问题。

## 3. 架构内的框架（不含「因为已经在用」）

### A — HTTP/WS JSON（tokio 家族）

热路径同构：upgrade 后自己 `serde` + `send`。差异在 **UDS、双 listener、WS 背压 API、JSON 实现**。

| crate | UDS | 双听（本机 sock + loopback） | WS / 背压 | 序列化相关 | 优劣（对我们） |
|---|---|---|---|---|---|
| **Poem 3** | 官方 `UnixListener::bind`；`with_permissions` / `with_owner` | 官方 `combine` | feature `websocket` | README：**sonic-rs 替代 JSON**（REST；WS 帧仍可自选用 sonic） | tokio 纯；双听与 sock 权限是本机 host 真需求；sonic 是少数**碰到 serde 热路径**的官方开关 |
| **Salvo** | 官方 `conn::unix::UnixListener`（feature `unix`） | 官方 `JoinedListener` | `WebSocketUpgrade` + `Sink`/`Stream` | serde | 同一进程绑 UDS+TCP 写得很死；还可选 quinn HTTP/3（远程后置，不是现在的闸） |
| **Rocket 0.5 + rocket_ws** | `unix:/path` listener（[UnixListener](https://api.rocket.rs/master/rocket/listener/unix/struct.UnixListener.html)） | 配两套 endpoint | **`Stream!` / `channel`**；`Config.max_send_queue` | serde | **推流 API 最贴 token 泵**；队列上限是背压，不是再调 tungstenite 私有字段 |
| **warp 0.4** | README 无一等 UDS | 无 | 官方 `warp::ws()`，底层 tungstenite | serde | Filter 组合漂亮；本机 sock 要自接 hyper——相对 Poem/Salvo 缺一块 |
| **actix-web 4** | 官方 `bind_uds` | 多次 bind | WS **默认 actor** | serde | 见 B；当 A 用则 runtime 税不值 |

**A 从零倾向**：要 tokio + UDS + 一进程两 listener → **Poem 或 Salvo**（能力接近，Poem 多 sock 权限与 sonic-rs 开关；Salvo 多 JoinedListener / HTTP/3）。要「推 Event 像写 generator、带发送队列上限」→ **Rocket + rocket_ws**。

### B — 连接对象化

| crate | runtime | UDS | WS | 对我们 |
|---|---|---|---|---|
| **ntex 3** | 自带 [`ntex::rt`](https://docs.rs/ntex/latest/ntex/rt/)：「**runs everything on the current thread**」；入口 `#[ntex::main]` | `bind_uds`（feature `uds`） | `ntex::web::ws::start` + 自有 `ntex::ws` 编解码 | **最像「网络服务框架」**（codec / io / service pipeline），不是网站路由器。税：与主仓 multi-thread tokio 世界并排 |
| **actix-web + actix-web-actors** | HTTP 可用 tokio；**WS actor 仍要 `actix_web::main` System**（[MIGRATION-4.0](https://github.com/actix/actix-web/blob/main/actix-web/MIGRATION-4.0.md)） | `bind_uds` | actor mailbox | 反向 RPC = 给该连接 actor 发消息，模型好看；System 与现有 runtime 打架 |

**B 从零倾向**：若接受「host 用 ntex 单线程 IO runtime、agent 仍 tokio」的双 runtime，ntex 比 actix 更贴高频帧。若不愿双 runtime，不要选 B。

### C — 换协议换流模型

| crate | 流 | 序列化 | 浏览器 | 对我们 |
|---|---|---|---|---|
| **tonic** | 官方 bidi `streaming` | prost | grpc-web | **流式语义第一**；UDS 官方 `Connected for UnixStream`。废 tag JSON |
| **jsonrpsee** | HTTP/WS + subscription | JSON-RPC 2.0 方法表 | WASM client | 词汇不是 Command 闭集；ask 方向别扭 |
| **tarpc** | tokio bincode/json | serde | 无浏览器一等 | 本机 typed RPC 漂亮，Web 仍要适配器 |

**C 从零倾向**：只有产品允许「线协议重写」时 tonic 才赢。否则是假对比。

### D — 裸帧

`tokio-tungstenite::from_raw_socket`（任意 `AsyncRead+Write`）或 `tokio_util::codec::LengthDelimitedCodec`。本机最瘦；浏览器必须另开 A。这是**载体分裂**，不是「更快的 serde」。

## 4. 和「高频 / 序列化」真正相关的旋钮

| 旋钮 | 谁提供 | 谁不提供 |
|---|---|---|
| 慢客户端发送队列上限 | Rocket `max_send_queue`；tungstenite write buffer（Salvo/Poem/warp 都能摸到） | 换路由器本身 |
| JSON 实现 | Poem feature sonic-rs；或 **WS 帧上自己** `sonic_rs`/`serde_json` | 任何 HTTP 框架的 RPS |
| 少 clone 到 N 窗 | 应用层 journal（Arc 字节 / 广播） | 框架 |
| HTTP/2 流控窗口 | tonic；actix `h2_initial_window_size` | 对 **WS/1.1 文本帧** 帮不上 |

## 5. 从零结论（可被产品决策推翻）

1. **先钉架构**：保留 Command/Event + 浏览器 → 只在 **A**（或可接受双 runtime 的 **B=ntex**）里挑。不要为「更流式」去 tonic，除非同时接受换协议。
2. **A 里不靠 axum 惯性**：Poem / Salvo / Rocket_ws 都能 UDS + WS + tokio。分位：Poem/Salvo 双听与 sock 元数据；Rocket 推流与队列背压。
3. **ntex** 是「除开网站框架」后最值得认真看的：**codec + ws + bind_uds**，但 `ntex::rt` 是当前线程 runtime。
4. 吞吐优化落在 **serde 实现与 journal**，写在实现票里，不要写成「换 crate 就快」。

现状：代码在 axum。那是迁移成本，不是能力证明。若选 Poem/Salvo/Rocket/ntex，要另估重写 `server/{rest,ws,runtime}` 的量。
