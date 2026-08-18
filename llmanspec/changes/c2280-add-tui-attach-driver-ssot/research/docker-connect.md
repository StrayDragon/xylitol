# Server 在 Docker 里时，TUI 怎么连

> 三种常见说法：UDS volume、vsock、发布端口。对照 Docker 官方网络文档和 Unix 套接字跨内核的限制。用词见 [`glossary.md`](./glossary.md)。

TUI 在你键盘前面的机器上。server（操作器、工作区、工具）在容器里。中间必须有一条 **TCP** 或 **同一内核上的 UDS**。没有第三条「Docker 自动把套接字变出来」的魔法。

## 1. published port（发布端口）— 默认该走这条

容器内 axum 听 `0.0.0.0:PORT`（或只听容器 loopback 再映射）。宿主机：

```text
docker run -p 127.0.0.1:<host-port>:<container-port> …
```

TUI attach：`http://127.0.0.1:<host-port>`，HTTP 上的 tagged JSON + WebSocket。

[Docker published ports](https://docs.docker.com/engine/network/#published-ports)：`-p` 把容器端口映射到 **宿主机**，这样宿主机上的进程（TUI）才能连。绑定 `127.0.0.1` 而不是 `0.0.0.0`，避免 LAN 上别人连你的 agent。

**能用的环境：** Docker Engine on Linux、**Docker Desktop（Mac/Windows，底下是 Linux VM）**。Desktop 会把发布端口从 VM 转到你的 loopback。这是跨 VM 唯一不折腾的一等路径。

**短处：** 本机多一个 TCP 端口；要管冲突和只绑 loopback。对「个人 coding agent」够用。

## 2. UDS + bind-mount 目录 — 仅同一 Linux 内核

Unix domain socket 是内核把两个进程接在一起，**套接字文件出现在 NFS / 另一台 VM 的文件系统上也不等于能通信**。

实务：

- 挂载 **目录**，不要只挂 `.sock` 这一个文件。监听进程会删掉再建 socket；挂文件会拿到旧 inode。[moby#21109](https://github.com/moby/moby/issues/21109)
- 容器内 `UnixListener::bind(/run/xylitol/host.sock)`，目录 `/run/xylitol` bind-mount 到宿主机同一路径。TUI `connect` 那个路径。
- 权限：socket 文件的 uid/gid 要让宿主机上的你能连。

**能用：** 宿主机就是 Linux，且用 **docker-ce / 同内核** 跑容器（不是 Desktop 那个隐藏 VM）。

**不能用：** Docker Desktop（Mac/Windows，以及 Linux 上的 Desktop）：容器在 VM 内核里，TUI 在宿主机内核里。套接字文件你甚至能 `ls` 到，`connect` 会失败。这和「NFS 上看到 .sock 但不能用」是同一类限制。

## 3. vsock（`AF_VSOCK`）— 不进 v1

vsock 是 **宿主机 ↔ 虚拟机** 的通道（Firecracker、Kata、libvirt 常见）。它 **不是** `docker run -v` 能挂进去的那种文件。

Docker 默认 seccomp 还拦过 `AF_VSOCK`（社区讨论：[forums.docker.com vsock unprivileged](https://forums.docker.com/t/how-to-use-vsock-in-unprivileged-containers/146310)）。要 privileged、自定义 seccomp、或 `/dev/vhost-vsock`。Docker Desktop 的 TUI 在 macOS/Windows 内核上，更对不上容器 VM 的 CID。

以后若 sandbox 是 **microVM**（Firecracker 一类），再单独做 vsock。现在「整个 serve 进 docker」用发布端口。

## 4. 整个 server 进 Docker = 粗粒度沙盒

可以把 **xylitol serve**（操作器 + 工作区）整进程放进容器。TUI 留在宿主机 attach。工具、MCP、bang 都在容器里跑。

这隔离的是容器的文件系统 / 网络 / 进程树，**不是**每个工具再套一层 seccomp。不要把整个 `$HOME` bind-mount 进去，否则沙盒是空的。

多扇 TUI 连这一个容器 = 本票 attach。以后「一个编排器管多个 server 容器」见 [`topology-sandbox.md`](./topology-sandbox.md)，不是本票。


## 怎么和 axum 叠

同一 `Router`：

| 你怎么跑 server | listener |
|---|---|
| `xylitol serve` 本机 Linux | `UnixListener`（可选再加 `127.0.0.1` TCP 给调试） |
| `docker run` + Desktop 或「TUI 在 VM 外」 | `TcpListener` + `-p 127.0.0.1:…` |
| Linux docker-ce，TUI 也在宿主机 | 可以 UDS 目录挂载，**或** 仍然用发布端口（简单、一种代码路径） |

实现票建议：**一条 TCP loopback 代码路径先打通 attach + bang 直播**。UDS 当 Linux 本机加分项，不要当 Docker 的唯一连法。

## 不要和这些搞混

| 说法 | 实际 |
|---|---|
| `-v /var/run/docker.sock` | 让容器指挥 Docker daemon，不是 xylitol 的 Command/Event 通道 |
| `--network host` | 容器和宿主机共享 netns（Linux Engine）。Desktop 上行为不同，且把 agent 端口直接铺在宿主机上 |
| 把 `.sock` 当普通文件 copy 出容器 | 没有同一内核，copy 无意义 |
