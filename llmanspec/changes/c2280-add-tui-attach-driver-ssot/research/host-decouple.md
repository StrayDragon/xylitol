# 架构 E：操作器在 server，TUI 管显示

> 吞吐优先。长期不做网页。工作区可以进 docker。tokio 留下。允许以后开 nightly，但默认还是 stable。

## wire protocol（线协议）是什么

TUI 进程和 server 进程之间约定的 JSON 形状，代码在 `src/protocol/wire/`。本票用 **tagged JSON**（带 `type` 字段），不换成 JSON-RPC envelope。词表：[`glossary.md`](./glossary.md)。

TUI 发 **Command**，例如 `Prompt`：用户打了这段话，开跑。
Server 推 **Event**，例如 `TextDelta`：模型又吐了几个字；`ToolEnd`：这个工具跑完了；reverse RPC：问用户选 A 还是 B。

现在这些消息是 JSON 文本，经 HTTP 和 WebSocket 寄。人能看懂，每条 `TextDelta` 都要 serde 一遍。

换 encoding 只是换字节格式：还是同一组 Command/Event，可以改成更短的二进制（postcard）。种类不用改。

`tonic` 会换成 protobuf schema。背压、身份校验现成，但现有 JSON 枚举要对一遍。本票不走这条。


## 术语校准

「TUI 仅视图」= 不在终端进程里跑工具 / MCP / 改仓库。键位、画界面、粘贴、你输入的句子，还在 TUI。句子变成请求发给 server。

| 留在 TUI 进程 | 必须在 server / sandbox |
|---|---|
| 键位、paint、TTY、剪贴板、`$EDITOR` | ReAct、工具、MCP、对工作区的读写、sandbox、**bang（`!` 在工作区跑 shell）** |
| 把用户意图打成 Command | 执行并把 Event 推回来 |

输入（prompt / steer / 审批）仍从 TUI 出发，不是无头。解耦的是 **操作器**，不是把 TUI 做成纯录像机。

## 吞吐分层（换 HTTP 框架几乎碰不到）

从大到小：

| 层 | 杠杆 | 换 Poem/Salvo/axum？ |
|---|---|---|
| 0. 进程切分 | 工具 IO / MCP / 模型等待不再和 TUI 抢 RSS 与启动税 | 无关 |
| 1. **编码** | 无浏览器 → 不必 JSON。每枚 `TextDelta` 少分配 | 无关 |
| 2. **成帧** | 本地不必 WebSocket 头/mask | 无关 |
| 3. 传输 | UDS / vsock（docker）/ 少 copy | 有 UDS 即可 |
| 4. runtime | 连接数 ≈ 1–3。io_uring 的 60% 故事来自 **高连接 TCP proxy**（[tokio-uring 公告](https://tokio.rs/blog/2021-07-tokio-uring)），不是「一条 UDS + LLM」 | 过早 |

结论：E 的框架问题是 **「选一条本地 RPC/流栈」**，不是「选网站框架」。

## 编码（无 Web 之后才允许动）

| 方案 | 一手定位 | 对 token 流 | 改协议成本 |
|---|---|---|---|
| 继续 serde JSON | 现状 | 每事件一次 tagged 文本；热 | 零 |
| **postcard**（仍 serde） | 紧凑二进制、serde 兼容 | 改 `to_vec` 即可；enum tag 仍在 | 低（词汇不变） |
| **prost / tonic** | schema、bidi stream | 类型化流；codegen | 高（废 Command/Event JSON） |
| **rkyv** | [零拷贝访问](https://rkyv.org/zero-copy-deserialization.html)：字节布局即内存 | TUI **读** journal/回放可不分配成 `String`；写侧仍要 archive | 中（自己的 Archive trait，不是 serde） |

token 洪水时，比「换 axum」更狠的是：**Delta 不要每次带完整 tag 对象**（专用 `u8 kind + u32 len + utf8`），那是 codec 设计，任何框架都能做。

nightly：`portable_simd` 可加速自定义 codec；**rkyv / postcard / tonic / ntex-uring 都不要求 nightly**。切 nightly 是许可，不是选 crate 的前提。

## 本地栈候选（全看）

### 1. ntex（网络服务，不是网站）

- 官方：runtime 必须选 feature：`tokio` / `compio` / `neon` / **`neon-uring`**（[ntex README](https://github.com/ntex-rs/ntex/)）。
- `bind_uds`（feature `uds`）；`ntex::ws` + `web::ws::start`；底层 **codec / io pipeline**。
- `ntex::rt`：current-thread；`rt::uring` 模块存在（[docs.rs ntex::rt](https://docs.rs/ntex/latest/ntex/rt/)）。
- **E 下为什么高**：本机帧 + 可选 io_uring，不背 HTTP 网站心智；docker 里可单线程泵 Event。
- **税**：主仓 agent 仍是 multi-thread tokio。要么 host 整段 ntex、agent 用 `ntex` feature `tokio` 共用，要么两 runtime。选 `neon-uring` 时 TUI 客户端也要能连（对端不必 uring）。

### 2. tonic（类型化 bidi，无浏览器包袱后变强）

- `serve_with_incoming(UnixListenerStream)`；`UdsConnectInfo.peer_cred` 可做 **sandbox 外 TUI 的 uid 校验**（[unix.rs](https://docs.rs/tonic/latest/src/tonic/transport/server/unix.rs.html)）。
- `streaming` bidi：Command 一流、Event 一流，反向 RPC = 同一 stream 上的消息，不必 HashMap oneshot。
- HTTP/2 窗口 = 协议级背压（慢 TUI 不会撑爆 server）。
- **税**：prost、codegen；与现有 serde enum 双写直到切完。无 Web 则 grpc-web 可推迟到「真有浏览器」那天。

### 3. 自写帧（tokio + LengthDelimitedCodec / ntex codec）+ postcard

- 词汇仍是 Command/Event，只换二进制。
- 本仓已有 tungstenite（可不用：本地无 WS 更瘦）。
- **税**：journal、重连 `last_seq`、反 RPC 继续自己写（今天已在 `server/ws.rs`）。
- 吞吐上限由 codec 决定，框架为零。

### 4. Poem / Salvo / Rocket_ws / warp / axum

在 **E + 长期无 Web** 下：HTTP 升级、CORS、REST 都是死重量。它们的 UDS/WS 能力 **用得上的部分**（双听、max_send_queue）可被 1–3 覆盖。
仅当还想留 `curl` 健康检查 / 调试 REST 时，用 **极薄 HTTP 旁路**，不要当 Event 泵。

Rocket `max_send_queue`、Poem `sonic-rs` 仍可参考为 **旋钮形态**，但不必为旋钮买整个网站框架。

### 5. tokio-uring / compio（runtime，不是 web 框架）

- tokio-uring：Linux 5.10+，**current-thread + !Sync 资源**；项目自我定位仍年轻（[docs.rs](https://docs.rs/tokio-uring/latest/tokio_uring/)）。
- 官方故事：epoll TCP proxy 内核占比高时 uring 才亮。xylitol 线协议连接极少；**工具在 sandbox 里狂读盘** 才可能吃到 uring。
- 更干净的接法：ntex `neon-uring` / `compio` feature，而不是在 axum 下塞 tokio-uring。

## docker / sandbox

框架不管隔离。要管的是：

- 容器内 server 听 **UDS（volume）或 vsock 或发到 host 的 TCP**。tonic/ntex/自写帧都能绑 UDS。
- `peer_cred`（tonic 现成）或显式 token：防止同机别的进程乱 attach。
- TUI 在 host，操作器在容器 → 传输是 **跨网络命名空间的本地 RPC**，不是浏览器。

## 从零排序（E 专用，可证伪）

1. **先切进程 + 二进制编码**（postcard 保词汇，或 tonic 重词汇）。JSON+WS 留作调试旁路。
2. **传输**：UDS 上 tonic **或** ntex codec **或** LengthDelimited+postcard。三者都能做反向流；tonic 背压与 peer_cred 最省事；ntex 在「自定义超瘦 delta codec + 可选 uring」上最开。
3. **不要**为吞吐去 Poem/Salvo/Rocket/axum。
4. **io_uring / nightly simd**：有 journal/serde 火焰图再开；默认 stable + ntex `tokio` 或 tonic 即可。
5. 现状 axum+JSON+WS 是 **可跑的缝**，不是 E 的目标态。

## 建议的下一刀（实现票前）

钉三个产品决策即可选 crate：

1. 词汇：保 `Command`/`Event`（postcard）还是 prost（tonic）？
2. docker：UDS volume vs vsock vs 发布端口？
3. host runtime：继续 tokio，还是允许 ntex current-thread（含 uring）？
