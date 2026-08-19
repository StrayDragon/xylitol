# c2280 调研（中立事实集）：能力归属、传输与拓扑

> 本目录只放**中立调研**：候选、能力对照、取舍权衡、一手出处。凡「做 / 不做 / 选哪个」**不**写在本目录。
> 全部能力事实以一手来源为准（本仓源码 + 上游官方文档）；出处 URL 集中在各篇末尾。

## 裁决落点（地图，不是本篇裁决）

本目录写于 c2280 探索期，当时提案自己收「研究结论」。产品位拆分之后：

| 问题 | 证据在本目录 | 产品裁决落点（change proposal，非本目录） |
|---|---|---|
| 能力归属、启动与连接方式、多窗写入 | `02-split` | `c2300-update-cs-capability-split` |
| 线协议闭集、禁双语义、面本地不进协议 | `03-paths` | `c2301-update-stable-wire-protocol` |
| host 多会话、传输面、HTTP 栈 | `01-framework`、`04-topology`；补测 `c2302/research/framework-pick.md` | `c2302-update-host-multi-session` |
| 默认 embed + 显式 attach | `04-topology`、本篇术语 | `c2303-update-unified-entry` |
| 符合性闸 | 本篇术语 | `c2304-add-conformance-gate` |
| ACP 外层适配 | `acp-interop.md` | `c2305-update-acp-provider-adapter`（后置） |
| 载体开销是否挡统一 CS | 补测 `c2300/research/overhead-eval.md` | `c2300`（默认形态 A） |

c2280 `proposal.md` 正文里仍有探索期「已钉 / 研究结论」（双模、axum、REST 旁路）。**以拆分草案段 + 上表产品位为准**；本目录不负责改写那些裁决句，只保证事实页可读、不把旧前提写成当前真源。

## 这份调研在支撑什么

当初要回答两个问题：TUI 与操作器各管什么（能力归属），以及本机 host 用什么传输载体（候选 vs 取舍）。01–06 各覆盖一个面；后续补了 ACP 与开销实测，仍按「证据页」读。

| 篇 | 覆盖（中立） |
|---|---|
| `01-framework` | 服务端框架 / 流模型候选的能力对照 |
| `02-split` | TUI 与操作器按能力自然归属的切分观察 |
| `03-paths` | 三种交互（prompt / bang / 审批）的协议流程与现状缺口 |
| `04-topology` | 运行拓扑与 Docker 连法能力对照 |
| `05-throughput` | 吞吐分层、编码选项、工具链 / nightly 能力面 |
| `06-web` | 浏览器 UI 壳的约束维度、候选对照、竞品横切 |
| `acp-interop.md` | ACP 外部侧表面事实（续传 / steer / 多客户端） |

## 阅读 01–06 时把「现状代码」和「产品位」分开

后文有几处按**当时代码与当时前提**写，不是产品位真源：

- `01` / `04`：当前仓用 axum；`rest.rs` 存在；UDS 能力对照把 axum 当本仓基线。产品位已把 REST 排除出产品语义、框架裁决在 c2302（salvo 默认 / axum 回退）。读能力表时只取「crate 能做什么」，不要把「本仓已依赖 axum」读成「继续用 axum」。
- `05`：分层杠杆仍成立；「极薄 HTTP 旁路」是候选句，不是产品要求。开销数量级以 `c2300/research/overhead-eval.md` 为准（tagged JSON 载体相对 500 tok/s 余量三个数量级；相对大头是广播 clone，不是编码）。
- `06`：「Host / 线协议 = axum HTTP/WS」是现状描述。产品约束仍是「浏览器可升级同一套 Command/Event」，壳开不开闸是非目标。
- `04`「两种运行形态」把「TUI 进程内跑操作器」写成今天行为——这是现状，不是「必须保留第二条语义」。embed / attach 的产品说法在 c2300 / c2303。

## 概念词映射表（开篇；后文每词只用一个词：中文或英文）

| 英文 | 中文 | 在本仓是什么 |
|---|---|---|
| wire protocol | 线协议 | TUI 与 host 约定的 JSON 形状，类型在 `src/protocol/wire/` |
| tagged JSON | 带 `type` 标签的 JSON | `#[serde(tag="type")]` 的 JSON 形状，如 `{"type":"prompt",...}` |
| Command | 命令 | 客户端 → host（`Prompt` / `Abort` / `Bash` / `ApproveTool` …） |
| Event | 事件 | host → 客户端（`TextDelta` / `ToolStart` / `BashResult` …） |
| encoding | 编码 | 序列化的字节格式（JSON 文本 / postcard 二进制 / protobuf）；消息种类不变 |
| envelope | 信封 | JSON-RPC 的 `{"jsonrpc","method","id"}` 外包层 |
| HTTP stack | HTTP 栈 | 路由 + extractor + WebSocket upgrade 的框架 |
| listener | 监听器 | `TcpListener` 或 `UnixListener`；同一 Router 换 listener 即换载体 |
| UDS | Unix 域套接字 | 同一内核进程间通信；白话见下节 |
| vsock（`AF_VSOCK`） | vsock | 宿主机 ↔ 虚拟机通道 |
| published port | 发布端口 | `docker run -p 127.0.0.1:H:P` 的端口映射 |
| WebSocket | WebSocket | HTTP 升级后的双向连接 |
| reverse RPC | 反向 RPC | host 问 TUI：批准工具 / 答题；`ReverseRpcGateway` |
| journal | 事件日志 | 带序号 Event 的环形缓冲，重连后按 `last_seq` 回放 |
| bang | 交互式 `!` / `!!` | 人在输入框发起的工作区 shell，与模型工具不同 |
| host | 宿主 / 操作器进程 | 产品用语：承载模型、会话、MCP、工作区执行的那一个进程。代码目录仍常叫 `server` |
| server | （代码名） | `src/app/server/` 等现状模块名；后文若写 server，与 host 指同一操作器，不另立语义 |
| operator | 操作器 | 站 host 侧的东西：ReAct、工具、MCP、工作区、bang 的 shell |
| face-local | 面本地 | 只属本机终端面的能力：键位、paint、剪贴板、TTY、`$EDITOR` |
| embed | 嵌入 | 默认启动：TUI 与 host 同进程，经同一套 Command/Event 自连 |
| attach | 连接已运行 host | 显式连一个已经在听的 host（本机 UDS 或 TCP） |
| launch & connect modes | 启动与连接方式 | embed 或 attach；不是两套产品语义 |
| session writer | 会话写者 | 一条 session 同一时刻唯一允许发写命令的那个客户端 |
| REST | （现状 HTTP JSON 面） | 当前 `src/app/server/rest.rs`；本表只指名，去留是产品裁决 |
| conformance gate | 符合性闸 | 同一张 Command 表 + 同一组可观察 Event，对两种驱动实现各跑一遍 |
| coarse sandbox | 粗粒度沙盒 | 整个 host 进程进容器，隔离进程树 / 文件系统 / 网络命名空间 |
| backpressure | 背压 | 慢客户端发送队列满时的等待 / 丢弃语义 |
| graceful shutdown | 优雅停机 | 停 host 时先断开 / 收摊再退出 |

## UDS 白话（Unix domain socket / Unix 域套接字）

上网常用 **TCP**：程序听一个端口号（如 `127.0.0.1:8080`），对端用 IP+端口连；连本机也要走协议栈。**UDS 不走 IP**：它在磁盘上放一个特殊文件（常见 `/run/xylitol/host.sock`），同一台机器、**同一个内核**里的两个进程——一个 `bind` 该路径并 listen，另一个 `connect` 该路径，内核在内存里把两边接上。没有网卡、没有端口冲突。

对本产品的差别：

- 更快、更本地：适合「本机 TUI attach 本机 host」。
- 操作系统能看到谁连（uid），可挡同机其他用户。
- **不能跨机器、不能跨虚拟机**：Docker Desktop 的容器跑在隐藏 Linux VM 里，TUI 在 Mac/Windows（另一套内核）上，两边都能看见 `.sock` 文件也没用——内核不同。这时只能走 TCP + 发布端口。
- **同进程 embed 不需要 UDS**：TUI 与 host 在一个进程里自连，套接字是给「第二扇窗 / 另一进程 attach」用的。

本机谁 bind、谁 connect（例子，路径待实现时钉）：

1. 常驻 host（`xylitol serve`，或默认 embed 若暴露传输面）先 `UnixListener::bind("/run/user/1000/xylitol/host.sock")`（1000 = 你的 uid），听着等连接。
2. 再开终端跑 `xylitol tui --attach`，`UnixStream::connect` 同一路径，连上后互发 Command / Event；第二、第三扇窗都是再 connect 同一路径。
3. TCP 对照：host 听 `127.0.0.1:18790`，TUI 连 `http://127.0.0.1:18790`。Docker 沙盒走这条（TUI 与容器往往不是同一内核，UDS 连不上）。

不要去 connect 一个普通文本文件；把 `.sock` copy 到另一台机器也毫无意义（没有同一内核）。
