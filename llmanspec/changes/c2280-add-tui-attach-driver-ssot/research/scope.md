# 固定深度调研：本机 host 传输 / 框架 + 双模 SSOT 可证伪点

> 本文件是 **调研范围**，不是结论。结论写入同目录后续 `transport.md` 等，并摘要回 `proposal.md` Further Notes。
> 一手资料 only；禁止博客转述当证据。目标：实现票开张前能 **选出或明确否决** 框架，并证伪/坐实 proposal 里的 SSOT 硬规则。

## 深度闸（做到这就停）

| 要做 | 不要做 |
|---|---|
| 读 **本仓** Server/Driver/wire 源码与 `server-core` / `protocol-app` 条款 | 实现 attach |
| 读 **axum / tokio-tungstenite / tokio::net::UnixListener** 官方文档与本仓已用 API | 写基准大战或自研 RPC |
| 读 **DSH** `packages/host/apiproxy` 分层笔记（载体 vs 协议）+ `dsh-lsp-stdio` 池 key | 把 cordis 引进 xylitol |
| 每个候选：本地 UDS、事件流、反向调用、重连、未来浏览器 各一行 **能/不能/要改协议** | 评 JSON-RPC vs gRPC 宗教 |
| 全文（不含引用块）建议 < 1500 词 / 篇 | 把选型笔记升格 `docs/research/` |

产出三篇即可：`transport.md`（框架）、`ssot-seams.md`（双模缝的代码证据）、`dsh-mapping.md`（对照表）。够否决就停，不够再加一篇。

## 问题（必须能证实或证伪）

### T1 我们是不是已经有协议，只缺载体？

- `protocol::Command` / `Event` / `dispatch` 是否覆盖产品 TUI 已用命令？（对照 `XyDriver` 方法集与 TUI effects）
- `XyRemoteDriver` 哪些方法是真 REST、哪些 stub/`session_tree_kind_unimplemented`？
- Server REST 是否忽略 `Path(session_id)`？（`src/app/server/rest.rs`）——多 session attach 的 blocker 是路由还是框架？

### T2 本机默认载体该是什么？

候选（只这些）：

| ID | 候选 | 为何在名单里 |
|---|---|---|
| C1 | **现有 axum HTTP+WS + 127.0.0.1** | 代码已在；Web 以后同构 |
| C2 | **axum 听 Unix socket**（HTTP/WS 仍跑在 UDS 上） | 本机无 TCP 端口；协议不变 |
| C3 | **UDS + 长度前缀 JSON `Command`/`Event`**（不经 HTTP） | 更瘦；要自写 journal/反 RPC |
| C4 | **jsonrpsee / JSON-RPC 2.0 over UDS** | 贴近 DSH SDK；可能 **换协议** |
| C5 | **tonic gRPC** | 强 schema；浏览器要 grpc-web |

对每个候选填：与现有 wire 兼容？反向 RPC（ask）怎么走？journal/重连？`feature = "server"` 体积？未来 Web 是否第二套？维护面（我们已依赖哪些 crate）？

**预置倾向（可被一手资料推翻）**：不换协议；C2 优先于 C1 做 **本机默认**；C1 留给 Web/远程；C4/C5 仅当 C2 在官方文档里走不通（无 UDS、无 WS upgrade、无反向流）。

### T3 锁与进程模型（serve-first 已钉）

- 现锁路径 `/tmp/xylitol-server.lock` 对多用户/多仓的实际语义
- 用户级 `$XDG_RUNTIME_DIR/xylitol/host.lock` + sock 是否够（POSIX `O_EXCL` 已有）
- axum `serve` + graceful shutdown 与「停 host → 所有 attach 断开」是否已够，还是要额外 connection tracker

### T4 双模 SSOT 能否被代码证伪？

对照 `in_process.rs`：哪些 `XyDriver` 方法其实在做 **面本地**（clipboard / TTY / `$EDITOR`）？哪些其实该在 **工作区**（bang、agent bash）？列出拆缝清单——这是 attach 前的切分，不是框架问题。

`packages/xylitol-tui` 是否已零引用主 crate（可保持薄）？产品 host 是否已只依赖 `XyDriver`？（应已是，用源码确认）

符合性闸最小切片：哪些 **现有** TUI harness / dispatch 测可以参数化 `InProcess | Remote`，不必重写场景？

### T5 MCP 池（只定 key，不实现）

DSH：`lazy single-flight per (server id, canonical workspace)`。xylitol `McpClientManager` 是否按进程一份、无 cwd key？attach 收益是否 **完全** 依赖把 manager 挪到 serve 进程？

## 一手资料清单（调研 agent 只准从这里长）

本仓：

- `src/app/core/driver/{mod,proto,in_process,remote}.rs`
- `src/app/server/{runtime,rest,ws,lock}.rs`
- `src/protocol/wire/{command,event,transport}.rs`
- `src/app/core/dispatch.rs`
- `llmanspec/specs/server-core/spec.toon`
- `llmanspec/specs/protocol-app/spec.toon`
- `docs/architecture/{库与多客户端,远程体验与线协议}.md`
- `Cargo.toml` `server` feature 依赖

上游：

- axum Unix domain：官方 docs / `axum::serve` + `tokio::net::UnixListener` 示例（当前 axum 主版本以 Cargo.lock 为准）
- tokio-tungstenite 是否能绑 UDS（或必须 http upgrade）
- 若碰 C4：jsonrpsee 官方 server UDS 文档首页，不深挖生态

对照仓（路径，非 URL）：

- `../deepseek-harness/.agents/notes/implemented/architecture/2026-07-19-gui-layering-and-rpc-protocol.md`
- `../deepseek-harness/packages/lsp/lsp-stdio/README.md`
- `../deepseek-harness/packages/sdk/{README,server/README,protocol/README}.md`

## 明确推迟（后续实现票 / 别的 research）

- systemd `--user` socket 激活
- 浏览器载体细节（CORS、origin）；本票只要求 **不选一个浏览器永远接不上的唯一传输**
- Gigatoken / TUI viewport 切片（另一条性能线）
- 同仓工具写锁

## 完成判据

1. T2 表每个候选有「采用 / 否决 / 仅 Web」+ 一条一手出处。
2. T4 给出「必须从 InProcessDriver 挪走的面本地方法」清单（可空，但必须搜过）。
3. 实现票能写死：本机默认载体 = {C?}，协议 = 现有 Command/Event 或否。
