---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
  - c2290-update-standalone-host
branch: sdd/c2302-update-host-multi-session
base_sha: 833ac1a1ea01777c5c4182c4d18c87efd5a8a219
checkpointed: false
---

# host 多会话与 salvo 载体（POST + WS 下行）

N 客户端 ↔ 1 个 HTTP 监听器。**产品信封由 c2290 钉死**（四象限：unary HTTP POST + 下行 WebSocket `ServerRequest`）。本票只换载体与多 session 组合根。栈：salvo 0.94。

> **作废旧 landing。** `sdd/c2302-update-host-multi-session` 曾落地「产品语义只走全双工 WS Command/Event」——与 c2290 / DSH 最新冲突，**禁止** `attach` 该分支。

c2301 归档交接里「c2302 必须走 WS 承载 Command/Event」作废：WS 只承载 host→client 的 `ServerRequest`；Command 闭集走 POST payload。

## Why

现状一把 `Mutex` + 锁文件 + port+1 + REST 产品面。要的是可连上的监听器，绑定占用即失败，多 session 各有 journal 与写者，并且载 c2290 信封而不是再发明一条 WS 词表。

## What Changes

- 监听 `127.0.0.1:18790`（`ServerConfig` / 现有 `server run --port`）。`--host` 可显式 `0.0.0.0`（本票配置字段即可；CLI 旗标形状归 c2303）。占用同一 `{addr,port}` → EADDRINUSE，不 port+1，无整机锁文件。
- 多 session：每 session **槽**独立 journal / seq / 写者 / turn 引擎。进程共享一份 `RuntimePorts`；槽在首次非只读 unary 时 lazy 物化。只读 attach 不物化写者。一写者；第二连接只读，写入回产品错误。
- 资源按 **本监听器进程** 装配。reload 作用于该进程、所有连接共享。
- 传输（salvo）：**只实现 c2290 方法表已登记的 unary**，禁止另开 REST 动词或第二套 method 字符串。
  - `POST /api/{method}`：ClientRequest → 同一 dispatch → ServerResponse
  - `POST /api/respond`：ClientResponse（回填 host 审批/问卷的 `rpcId`）
  - `GET /api/events.mux`（及可选 `events.host`）：`WebSocketUpgrade`，只推方法表里的下行 `ServerRequest`，不收业务上行
  - `GET /healthz` 可留；MUST NOT 再提供 run/abort/tree 等产品 REST
- 产品 TUI 已由 c2290 接到 `HttpWsClient`；本票把监听器接上，使 attach 从「未在听失败」变成可会话。不再 POST 产品 REST，也不再全双工 WS Command。
- 优雅停机：`ServerHandle::stop_graceful`；停则连上的客户端断开。
- 保留现有 `server run/install/stop` 动词（c2303 再改 `serve`/`--attach`）。`stop` 不再靠锁文件。
- 观测：不启用 salvo `logging` feature 的 `tracing` Logger。

## 非目标

进线 CLI（`--attach` / `serve` 改名）、符合性闸、ACP、二进制编码、UDS 默认发现、Docker 粗沙盒、MCP 池 key 落地、Web UI、salvo oapi（c2320）、embed 的 in-process 信封 carrier（print 仍走现有 InProcessDriver；`InProcessClient` 已在 c2290）。

## Open Questions

无。Driver 基数已钉：占用 = session 槽；进程共享 `RuntimePorts`；每槽 lazy 物化隔离 Driver。见 `design.md` §10。

## Further Notes

c2290 verify 未修项（真 Host / ReverseRpc / `sr-env1` 假绿升级）：[`research/c2290-verify-handoff.md`](./research/c2290-verify-handoff.md)

已锁定细节：`design.md` §9（不实现 `events.host`、删 REST、停机不读锁、审批走 respond、`subscribe` 进 `run()` 等）。
