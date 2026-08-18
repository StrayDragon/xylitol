# 用词（英文专名 + 中文）

调研里不再用「纸条」「管子」这类比喻。对照代码时用下面这些词。

| 英文 | 中文 | 在本仓里是什么 |
|---|---|---|
| **wire protocol** | 线协议 | TUI 进程和 server 进程约定的 JSON 形状。类型在 `src/protocol/wire/`。 |
| **tagged JSON** | 带 `type` 标签的 JSON | `#[serde(tag = "type")]`。例：`{"type":"prompt","message":"..."}`。**本票钉死用这个**，不换成 JSON-RPC 信封。 |
| **Command** | 命令 | 客户端 → server。`protocol::Command`（`Prompt` / `Abort` / `Bash` / `ApproveTool` …）。 |
| **Event** | 事件 | server → 客户端。`protocol::Event`（`TextDelta` / `ToolStart` / `BashResult` …）。 |
| **encoding** | 编码 | JSON 文本 vs postcard 二进制 vs protobuf。种类可以不变，字节格式变。 |
| **envelope** | 信封 | JSON-RPC 的 `{"jsonrpc","method","id"}` 外包一层。我们 **不用**。 |
| **HTTP stack** | HTTP 栈 | 路由 + extractor + WebSocket upgrade。六个候选里选出的那个 crate。 |
| **listener** | 监听器 | `tokio::net::TcpListener` 或 `UnixListener`。同一个 Router 可以换监听器。 |
| **UDS** / Unix domain socket | Unix 域套接字 | 见下文专节。 |

| **vsock** / `AF_VSOCK` | virtio socket | 宿主机 ↔ 虚拟机。不是普通 Docker 应用的默认通道。 |
| **published port** | 发布端口 | `docker run -p 127.0.0.1:PORT:PORT`。容器里的 TCP 映射到宿主机。 |
| **WebSocket** | WebSocket | HTTP 升级后的双向连接。本仓用它推 Event、反向 RPC。 |
| **reverse RPC** | 反向 RPC | server 问 TUI：批准工具 / 回答问题。`ReverseRpcGateway`。 |
| **journal** | 事件日志 | 带序号的 Event 环形缓冲，重连时 replay。 |
| **bang** | 交互式 `!` / `!!` | 人在输入框发起的工作区 shell，不是模型工具。 |

## UDS 是什么（Unix domain socket / Unix 域套接字）

普通上网用 **TCP**：程序听一个端口号（如 `127.0.0.1:8080`），另一边用 IP+端口连上来。连本机也要走网络协议栈。

**UDS** 不走 IP。它在磁盘上放一个特殊文件，常见名字像 `/run/xylitol/host.sock`。同一台机器、**同一个 Linux 内核**里的两个进程：一个 `bind` 这个文件并 listen，另一个 `connect` 这个路径。内核在内存里把两边接上。没有网卡、没有端口冲突。

和 TCP 的差别（对本产品）：

- **更快、更本地**：给「本机 TUI attach 本机 serve」用。
- **操作系统能看到是谁连的**（uid），可以挡同机其他用户。
- **不能跨机器、不能跨虚拟机。** Docker Desktop 的容器跑在隐藏 Linux VM 里，你的 TUI 在 Mac/Windows（或另一套内核）上——两边都能看见那个 `.sock` 文件也没用，内核不是同一个。这时只能用 TCP + **published port**。

缩写 UDS = Unix domain socket。有人口语叫 Unix socket，是同一个东西。

### 本机谁 bind、谁 connect（例子）

假设 Linux 上你先起 serve，再开两扇 TUI attach。路径只是例子，实现票再钉 XDG 目录。

1. **serve bind（听）**
   进程 `xylitol serve` 创建目录 `/run/user/1000/xylitol/`（`1000` 是你的 uid），然后：

   ```text
   UnixListener::bind("/run/user/1000/xylitol/host.sock")
   ```

   内核在这个路径上放一个 socket 文件。serve **听着**，等别人连。

2. **TUI connect（连）**
   你在窗口管理器里再开一个终端，跑 `xylitol tui --attach`。这个进程 **不听**，它：

   ```text
   UnixStream::connect("/run/user/1000/xylitol/host.sock")
   ```

   连上之后，两边用 tagged JSON 发 Command / 收 Event。第二扇、第三扇 TUI 都是再 `connect` 同一个路径。

3. **和 TCP 对照**
   同一件事用端口：serve `TcpListener::bind("127.0.0.1:18790")`，TUI 连 `http://127.0.0.1:18790`。Docker 沙盒走这条，因为 TUI 和容器往往不是同一个内核，UDS 连不上。

不要去 `connect` 一个普通文本文件，也没有人把 `.sock` copy 到另一台机器还能用。
