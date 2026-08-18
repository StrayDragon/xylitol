# HTTP 栈再比：axum vs poem vs salvo（对着 Docker 拓扑）

> 用户否决「已经在用 axum 所以继续」。本篇只比 **UDS / 双听 / 和「server 在 Docker、TUI 在宿主机」是否相关**。
> 一手： [poem UnixListener](https://docs.rs/poem/latest/poem/listener/struct.UnixListener.html) · [poem Listener::combine](https://docs.rs/poem/latest/poem/listener/trait.Listener.html) · [salvo UnixListener](https://docs.rs/salvo/latest/salvo/conn/unix/struct.UnixListener.html) · [salvo Listener::join](https://docs.rs/salvo_core/latest/salvo_core/conn/trait.Listener.html) · axum [unix-domain-socket 示例](https://github.com/tokio-rs/axum/blob/main/examples/unix-domain-socket/src/main.rs)
> 拓扑：[`topology-sandbox.md`](./topology-sandbox.md)

## UDS / 双听：poem 和 salvo 确实赢

| | axum 0.8 | poem 3.1 | salvo 0.95 |
|---|---|---|---|
| UnixListener | `tokio::net::UnixListener` 交给 `axum::serve` | 一等 `UnixListener::bind` | 一等 `UnixListener::new`（feature `unix`） |
| bind 时 chmod / chown | 自己 `std::fs::set_permissions` / libc | **`with_permissions` / `with_owner`** | **`permissions` / `owner`** |
| UDS + TCP 同一进程 | 两个 `axum::serve` 任务，同一 `Router` | **`listener.combine(other)` → Combined** | **`listener.join(other)` → JoinedListener** |
| WebSocket tagged JSON | 已有 `rest.rs` / `ws.rs` | 要重写 handler | 要重写 handler |
| Docker Desktop 上 TUI attach | TCP published port | 同左 | 同左 |

poem 和 salvo 在 **本机 Linux、chmod socket、一行双听** 上就是更完整。这点成立，不因为本仓已经 axum 就假装没有。

## 但这条优势对「server 进 Docker」几乎用不上

TUI 在宿主机、server 在容器里：

- Docker Desktop：UDS **跨不了 VM 内核**。attach = TCP + published port。
- 容器内的 `UnixListener` 只给 **同一容器 / 同内核 bind-mount 目录** 用。宿主机 TUI 不是那个客户端（Desktop 上尤其不是）。

所以：用 poem 的 `combine(Unix, Tcp)` 换掉 axum，**不会**让 Docker 沙盒更好连，也 **不会**让 bang 的 `BashDelta` 出现。双听 API 优化的是「本机不经过 Docker 的 serve」。

## 修订后的钉法（不是惯性句）

1. **主 attach 路径**（Docker 沙盒 + 多 TUI）：TCP loopback + published port + WebSocket + tagged JSON。三个 crate 在这条路上同构。
2. **因此不换 poem/salvo**：换栈的成本是重写 `src/app/server/{rest,ws,runtime}.rs` 的 HTTP 皮，收益落在本机 UDS chmod/双听——那不是 Docker 主路。
3. **本机 Linux 无 Docker 的 UDS**：axum 官方示例已经能 `serve(UnixListener)`。chmod 自己做十行。需要双听时两个 serve 任务，丑，但可测。
4. **若以后产品改成「默认永不 Docker、只 UDS」**：再评估迁 poem 或 salvo。现在不要为那张图付迁移税。

HTTP 栈 **仍是 axum**。理由改成：主拓扑是 TCP attach；UDS 文档赢家（poem/salvo）对这条拓扑不对齐。不是「Cargo.toml 里已经有了」。

warp / actix / Rocket 仍否决（无一等 UDS chmod、或 worker/actor 税），见 [`http-json-tokio-top6.md`](./http-json-tokio-top6.md)。
