# 拓扑：默认本机 serve，Docker 是可选沙盒

> 本票不实现编排。用词见 [`glossary.md`](./glossary.md)（**UDS** 那节）。连法见 [`docker-connect.md`](./docker-connect.md)。

## 产品默认（和 proposal P4 对齐）

- **默认**：TUI 进程内跑操作器（今天这样）。要 attach 时，本机 `xylitol serve`，不必进 Docker。本机 Linux 可以听 UDS。
- **可选沙盒**：把 **整个 serve** 放进 Docker。TUI 仍在宿主机，走 published port。工具 / bang / MCP 都在容器里。

两条都是「N 个 TUI ↔ 1 个 server」。Docker 不是新协议，只是 serve 的运行位置。


## 你说的那条：server 部署进 Docker

可以。而且这就是 **粗粒度 sandbox（沙盒）**：

| 进容器的 | 仍在宿主机 TUI 进程的 |
|---|---|
| 操作器：ReAct、工具、MCP、工作区文件系统、bang 的 shell | 键位、paint、剪贴板、TTY、`$EDITOR` |

模型调用的 `rm`、MCP、`!ls` 都在容器的根文件系统 / 挂进去的工作区里跑，碰不到你笔记本上随便一个目录——前提是你 **不要** 把整个 `$HOME` bind-mount 进去。

这和「每个工具再套一层 seccomp / landlock」不是同一层。Docker 管的是 **进程树 + 文件系统 + 网络命名空间**。xylitol 以后若还有细粒度工具沙盒，是容器 **里面** 的事，不替代「server 整进程进容器」。

TUI 怎么连进去：宿主机进程 → 容器里的 axum。Docker Desktop 只能走 **published port**。Linux docker-ce 才可以额外 UDS 目录挂载。见 docker-connect。

## 和 c2280 已经对齐的：N 个 TUI ↔ 1 个 server

```text
TUI (窗 A) ──┐
TUI (窗 B) ──┼──►  一个 xylitol serve（可在 Docker 里）
TUI (窗 C) ──┘         │
                       ├─ 工作区 / 工具 / MCP
                       └─ journal + reverse RPC
```

关一扇 TUI 不得杀 server（proposal P5）。这就是 attach 的收益：第二扇窗不付 MCP 启动税。

HTTP 栈不负责「多窗」——多窗是同一 WebSocket / HTTP 上的多个订阅者。journal 已经按 session 缓冲 Event。

## 以后再做、本票不做：1 个管理面 → N 个 server

```text
若干 TUI ──► serve-A（仓 / 容器 A）
若干 TUI ──► serve-B（仓 / 容器 B）
                 ▲
                 └── 以后的编排器（启停、路由、锁）  ← 不是 c2280
```

多 server 才需要：工作区写锁、按 cwd 的 MCP 池、session 落在哪台、TUI 连哪把锁。现在的单实例 `xylitol serve` 锁（proposal P3）是 **一台 host 一把**，不是编排器。

路线可以记在 change 调研里，**不要**在本票把 HTTP 栈做成「将来能管一千个容器的控制面」。一个 tagged JSON 的 attach 通道够用；编排是另一张图。

## 对 HTTP 栈意味着什么

server **经常**在 Docker 里、TUI 在外面时：

- 产品 attach 路径是 **TCP + published port + WebSocket 上的 tagged JSON**
- Unix domain socket 的 `chmod` / `combine` 再漂亮，也帮不了 Docker Desktop 上的 TUI
- 因此 **不要用 UDS API 的优劣来换 HTTP 栈**（见 [`http-stack-revisit.md`](./http-stack-revisit.md)）

本机不经过 Docker 的 `xylitol serve` 仍可以听 UDS；那是第二条拓扑，不是 Docker 沙盒的主路。
