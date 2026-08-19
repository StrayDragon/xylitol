# c2280 调研（中立事实集）：双模 TUI 传输与切分

> 本目录只放**中立调研**：候选、能力对照、取舍权衡、一手出处。凡「做 / 不做 / 选哪个」的裁决一律在 `proposal.md`「研究结论」与「非目标」；本目录是它的论据页，不裁、不选。
> 全部能力事实以一手来源为准（本仓源码 + 上游官方文档）；出处 URL 集中在各篇末尾。

## 这份调研在支撑什么

c2280 提案要回答两个问题：attach 之后 TUI 与 server 各管什么（能力归属），以及本机 host 用什么传输载体（候选 vs 取舍）。本目录 01–06 各覆盖一个面：

| 篇 | 覆盖（中立） |
|---|---|
| `01-framework` | 服务端框架 / 流模型候选的能力对照与取舍 |
| `02-split` | TUI 与 server 按能力自然归属的切分观察 |
| `03-paths` | 三种交互（prompt / bang / 审批）的协议流程与现状缺口 |
| `04-topology` | 运行拓扑与 Docker 连法能力对照 |
| `05-throughput` | 吞吐分层、编码选项、工具链 / nightly 能力面 |
| `06-web` | 浏览器 UI 壳的约束维度、候选对照、竞品横切 |

## 概念词映射表（开篇；后文每词只用一个词：中文或英文）

| 英文 | 中文 | 在本仓是什么 |
|---|---|---|
| wire protocol | 线协议 | TUI 与 server 约定的 JSON 形状，类型在 `src/protocol/wire/` |
| tagged JSON | 带 `type` 标签的 JSON | `#[serde(tag="type")]` 的 JSON 形状，如 `{"type":"prompt",...}` |
| Command | 命令 | 客户端 → server（`Prompt` / `Abort` / `Bash` / `ApproveTool` …） |
| Event | 事件 | server → 客户端（`TextDelta` / `ToolStart` / `BashResult` …） |
| encoding | 编码 | 序列化的字节格式（JSON 文本 / postcard 二进制 / protobuf）；消息种类不变 |
| envelope | 信封 | JSON-RPC 的 `{"jsonrpc","method","id"}` 外包层 |
| HTTP stack | HTTP 栈 | 路由 + extractor + WebSocket upgrade 的框架 |
| listener | 监听器 | `TcpListener` 或 `UnixListener`；同一 Router 换 listener 即换载体 |
| UDS | Unix 域套接字 | 同一内核进程间通信；白话见下节 |
| vsock（`AF_VSOCK`） | vsock | 宿主机 ↔ 虚拟机通道 |
| published port | 发布端口 | `docker run -p 127.0.0.1:H:P` 的端口映射 |
| WebSocket | WebSocket | HTTP 升级后的双向连接 |
| reverse RPC | 反向 RPC | server 问 TUI：批准工具 / 答题；`ReverseRpcGateway` |
| journal | 事件日志 | 带序号 Event 的环形缓冲，重连后按 `last_seq` 回放 |
| bang | 交互式 `!` / `!!` | 人在输入框发起的工作区 shell，与模型工具不同 |
| operator | 操作器 | 站 server 侧的东西：ReAct、工具、MCP、工作区、bang 的 shell |
| face-local | 面本地 | 只属本机终端面的能力：键位、paint、剪贴板、TTY、`$EDITOR` |
| conformance gate | 符合性闸 | 同一张 Command 表 + 同一组可观察 Event，对两种驱动实现各跑一遍 |
| coarse sandbox | 粗粒度沙盒 | 整个 serve 进程进容器，隔离进程树 / 文件系统 / 网络命名空间 |
| backpressure | 背压 | 慢客户端发送队列满时的等待 / 丢弃语义 |
| graceful shutdown | 优雅停机 | 停 host 时先断开 / 收摊再退出 |

## UDS 白话（Unix domain socket / Unix 域套接字）

上网常用 **TCP**：程序听一个端口号（如 `127.0.0.1:8080`），对端用 IP+端口连；连本机也要走协议栈。**UDS 不走 IP**：它在磁盘上放一个特殊文件（常见 `/run/xylitol/host.sock`），同一台机器、**同一个内核**里的两个进程——一个 `bind` 该路径并 listen，另一个 `connect` 该路径，内核在内存里把两边接上。没有网卡、没有端口冲突。

对本产品的差别：

- 更快、更本地：适合「本机 TUI attach 本机 serve」。
- 操作系统能看到谁连（uid），可挡同机其他用户。
- **不能跨机器、不能跨虚拟机**：Docker Desktop 的容器跑在隐藏 Linux VM 里，TUI 在 Mac/Windows（另一套内核）上，两边都能看见 `.sock` 文件也没用——内核不同。这时只能走 TCP + 发布端口。

本机谁 bind、谁 connect（例子，路径待实现时钉）：

1. `xylitol serve` 先 `UnixListener::bind("/run/user/1000/xylitol/host.sock")`（1000 = 你的 uid），听着等连接。
2. 再开终端跑 `xylitol tui --attach`，`UnixStream::connect` 同一路径，连上后互发 Command / Event；第二、第三扇窗都是再 connect 同一路径。
3. TCP 对照：serve 听 `127.0.0.1:18790`，TUI 连 `http://127.0.0.1:18790`。Docker 沙盒走这条（TUI 与容器往往不是同一内核，UDS 连不上）。

不要去 connect 一个普通文本文件；把 `.sock` copy 到另一台机器也毫无意义（没有同一内核）。
