# 05 吞吐分层与工具链选项（能力对照，非裁决）

> 本篇是「吞吐怎么上台阶」与「换工具链要不要 nightly」的能力事实；默认工具链、哪些单独 spike、是否切 nightly 是提案裁决，见 `proposal.md`「研究结论」。

## 吞吐分层（层级杠杆与框架相关性）

| 层 | 杠杆 | 与 HTTP 框架相关？ |
|---|---|---|
| 0. 进程切分 | 工具 IO / MCP / 模型等待不再和 TUI 抢 RSS 与启动税 | 无关 |
| 1. 编码 | 无浏览器 → 不必 JSON；每枚 `TextDelta` 少分配 | 无关 |
| 2. 成帧 | 本地不必 WebSocket 头 / mask | 无关 |
| 3. 传输 | UDS / vsock（docker）/ 少 copy | 有 UDS 即可 |
| 4. runtime | 连接数 ≈ 1–3；io_uring 的「60%」来自高连接 TCP proxy | 过早 |

推论：若走「只做本机、无 Web」的路，问题本质是「选一条本地 RPC/流栈」，不是「选网站框架」。

## 编码选项（代价表）

| 方案 | 一手定位 | 对 token 流 | 改协议成本 |
|---|---|---|---|
| 继续 serde JSON | 现状 | 每事件一次 tagged 文本；热 | 零 |
| **postcard** | 紧凑二进制、serde 兼容 | 改 `to_vec` 即可；enum tag 仍在 | 低（词汇不变） |
| **rkyv** | 零拷贝访问（字节布局即内存） | TUI 读 journal/回放可不分配成 `String`；写侧仍要 archive | 中（自己的 Archive trait，不是 serde） |
| **prost / tonic** | schema、bidi stream | 类型化流 + codegen | 高（废 Command/Event JSON） |

token 洪水时比换框架更狠的一招：**Delta 不带完整 tag 对象**——专用 `u8 kind + u32 len + utf8` codec，任何框架都能做。

## 本地 RPC/流栈候选（强弱 + 税）

| 候选 | 强在哪 | 税 |
|---|---|---|
| **ntex**（网络服务，不是网站） | `codec` / io pipeline + `ws` + `bind_uds`；可选 io_uring（feature `neon-uring`，官方要新内核，crates.io 版 MSRV 已到 1.97） | `ntex::rt` 是 current-thread runtime；主仓 agent 是 multi-thread tokio——要么 host 整段 ntex + agent 用其 `tokio` feature 共用，要么双 runtime |
| **tonic** | 一流 bidi streaming（反向 RPC = 同一 stream，不必 oneshot map）；`serve_with_incoming(UnixListenerStream)`；`peer_cred` 做沙盒外 TUI 的 uid 校验；HTTP/2 窗口 = 协议级背压 | prost / codegen；与现有 serde enum 双写直到切完；无 Web 则 grpc-web 可推迟 |
| **自写帧**（`LengthDelimitedCodec` + postcard） | 最瘦；词汇仍是 Command/Event | journal、重连 `last_seq`、反向 RPC 继续自己写（今天已在 `server/ws.rs`）；吞吐上限由 codec 决定，框架为零 |

若只想留 `curl` 健康检查 / 调试 REST，可用**极薄 HTTP 旁路**，不必当 Event 泵。poem / salvo / rocket_ws 的 UDS 双听 / `max_send_queue` / sonic-rs 可当「旋钮形态」参考，不一定为其买整个网站框架（`01`）。

**io_uring**：帮的是「很多连接或很多磁盘读写」时少 syscall。TUI 到 server 通常一两条连接；沙盒里工具狂读仓库，才可能在 server 进程上看到 uring 的好处——那仍是 stable 的 ntex / tokio-uring，不是 `portable_simd`。

## nightly 议题：哪些能力是否依赖 nightly（事实）

本仓钉 Rust **1.97.1 stable**（`rust-toolchain.toml`）；下列按「编 xylitol 本体要不要 nightly」分列。tokio 继续用；io_uring / SIMD 是另开的门，不是换掉 tokio。

不需要 nightly：

| 想加快的 | 用什么 | 硬性要求 |
|---|---|---|
| 消息别再用 JSON 文本 | `postcard`（还是 serde，包装换二进制） | stable |
| TUI 读一长串历史少分配 | `rkyv` | stable |
| Linux 上文件/网络少进内核 | ntex `neon-uring`，或以后再看 `tokio-uring` | stable；要新内核（官方写 5.10+） |
| REST 调试口的 JSON | `sonic-rs`（Poem 有开关；自己 parse 也能调） | stable |

需要 nightly：

| feature / 工具 | 干什么 | 触发条件 |
|---|---|---|
| `portable_simd`（`std::simd`） | 手写 SIMD 编解码 | 已做「专用 delta 格式」且 CPU 剖面显示编解码占热才值得；Gigatoken 整 crate 强制它，被本仓 c1530 推迟 |
| Cranelift codegen | 编译变快，不是跑 agent 变快 | 本地开发可选；CI 仍需 LLVM stable。见 skill `rust-build-tune` |
| `#[feature(...)]` 实验 API | 各式各样 | 没有一项是 attach / 吞吐的前提 |

若切 nightly 的影响面（事实）：CI 矩阵翻倍、依赖更容易跟 nightly 日期绑死、与 1.97.1 钉版本打架。

## 一手来源

- ntex：[README](https://github.com/ntex-rs/ntex/) · [rt / uring](https://docs.rs/ntex/latest/ntex/rt/)
- tonic：[unix.rs（peer_cred）](https://docs.rs/tonic/latest/src/tonic/transport/server/unix.rs.html) · [uds/server.rs 示例](https://raw.githubusercontent.com/hyperium/tonic/v0.14.x/examples/src/uds/server.rs)
- tokio-uring（io_uring 的高连接故事）：[tokio.rs blog](https://tokio.rs/blog/2021-07-tokio-uring) · [docs.rs](https://docs.rs/tokio-uring/latest/tokio_uring/)
- rkyv（零拷贝反序列化）：[rkyv.org](https://rkyv.org/zero-copy-deserialization.html)
