# 04 运行拓扑与 Docker 连法

> 怎么跑、怎么连。进线产品在 c2300 / c2303；HTTP 栈在 c2302。vsock 不进本轮。

## 现状 vs 可连法

- **现状**：TUI 进程内跑操作器，没有真正的 attach。
- **本机**：host 可听 UDS（同内核）或 loopback TCP；另一进程 attach。
- **host 进 Docker = 粗粒度沙盒**：TUI 在宿主机，工具 / bang / MCP 在容器里。

| 进容器的 | 仍在宿主机 TUI 进程的 |
|---|---|
| 操作器：ReAct、工具、MCP、工作区文件系统、bang 的 shell | 键位、paint、剪贴板、TTY、`$EDITOR` |

粗粒度沙盒隔离的是**进程树 + 文件系统 + 网络命名空间**；与「每个工具再套一层 seccomp / landlock」是不同层——细粒度工具沙盒若要做，是容器**里面**的事，不替代「server 整进程进容器」。注意：不要整个 `$HOME` bind-mount 进去，否则沙盒是空的。

N 个 TUI ↔ 1 个 host：多窗 = 多条订阅，journal 按 session 缓冲 Event。

## server 在 Docker 里时的三种连法（能力对照）

### 1. published port（发布端口）

容器内 host 听 `0.0.0.0:PORT`，宿主机 `docker run -p 127.0.0.1:<host-port>:<container-port>`；TUI attach 该 loopback 端口（tagged JSON + WebSocket）。

- **能用**：Docker Engine on Linux、**Docker Desktop（Mac/Windows，底下是 Linux VM）**。Desktop 会把发布端口从 VM 转到你的 loopback——这是跨 VM 唯一不折腾的一等路径。
- **要点**：绑定 `127.0.0.1` 而不是 `0.0.0.0`，避免 LAN 上别人连你的 agent。
- **短处**：本机多一个 TCP 端口，要管冲突和只绑 loopback。

### 2. UDS + bind-mount 目录 — 仅同一 Linux 内核

UDS 是内核把两个进程接在一起；**套接字文件出现在 NFS / 另一台 VM 的文件系统上也不等于能通信**。

- 挂载**目录**，不要只挂 `.sock` 这一个文件（监听进程会删了重建 socket，挂文件会拿到旧 inode；moby#21109）。
- 容器内 `UnixListener::bind(/run/xylitol/host.sock)`，`/run/xylitol` bind-mount 到宿主机同一路径，TUI `connect` 该路径；socket 文件的 uid/gid 要让宿主机上的你能连。
- **能用**：宿主机就是 Linux，且用 docker-ce / 同内核跑容器。**不能用**：Docker Desktop（容器在 VM 内核、TUI 在宿主机内核——套接字文件你甚至能 `ls` 到，`connect` 会失败，和「NFS 上看到 `.sock` 但不能用」同一类限制）。

### 3. vsock（`AF_VSOCK`）

vsock 是**宿主机 ↔ 虚拟机**的通道（Firecracker、Kata、libvirt 常见），不是 `docker run -v` 能挂进去的文件。Docker 默认 seccomp 拦过 `AF_VSOCK`；要 privileged、自定义 seccomp 或 `/dev/vhost-vsock`。Docker Desktop 的 TUI 在 macOS/Windows 内核上更对不上容器 VM 的 CID。适用场景偏向 microVM 型沙盒。

## listener（事实）

同一套路由，只换 listener：

| 怎么跑 server | listener |
|---|---|
| `xylitol serve` 本机 Linux | `UnixListener`（可选再加 `127.0.0.1` TCP 给调试） |
| docker + Desktop / 「TUI 在 VM 外」 | `TcpListener` + `-p 127.0.0.1:…` |
| Linux docker-ce，TUI 也在宿主机 | 可以 UDS 目录挂载，或仍用发布端口（一种代码路径，更简单） |

## 容易混淆的项（事实）

| 说法 | 实际 |
|---|---|
| `-v /var/run/docker.sock` | 让容器指挥 Docker daemon，不是 xylitol 的 Command/Event 通道 |
| `--network host` | 容器与宿主机共享 netns（Linux Engine；Desktop 上行为不同），并把 agent 端口直接铺在宿主机上 |
| 把 `.sock` 当普通文件 copy 出容器 | 没有同一内核，copy 无意义 |

## 1 管理面 → N host

工作区写锁、MCP 池按哪台、session 落点、客户端连哪个 bind。本轮是 N 客户端 ↔ 1 个 HTTP host 进程。

## 用户拉起 serve 的习惯（背景，不实现）

用户画像：习惯用**窗口管理器**（i3 / sway / Hyprland，或 tmux 多窗格）、经常多开终端。常见习惯：

| 习惯 | 做什么 |
|---|---|
| 图形会话 autostart | 登录后起一次 serve（xdg autostart、systemd `--user`、WM 自己的 startup 列表） |
| 工作区第一条终端 | 进某 workspace 手动/脚本先 `xylitol serve`，再开若干 `xylitol tui --attach` |
| 终端复用器 | tmux/zellij 里一个 pane 跑 serve，其余 pane attach |

serve 进 Docker 时，autostart 的是 `docker run … xylitol serve`（或 compose），TUI 仍在宿主机连发布端口。

## 一手来源

- Docker published ports：[docs.docker.com/engine/network](https://docs.docker.com/engine/network/#published-ports)
- bind-mount 挂目录而非 .sock 单文件：[moby#21109](https://github.com/moby/moby/issues/21109)
- vsock 与 Docker seccomp：[forums.docker.com vsock unprivileged](https://forums.docker.com/t/how-to-use-vsock-in-unprivileged-containers/146310)
- 现状锁文件：`/tmp/xylitol-server.lock`（代码现状）。产品改为绑定占用（EADDRINUSE），不是整机互斥。
