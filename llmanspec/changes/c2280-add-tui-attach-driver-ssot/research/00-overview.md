# c2280 调研：能力归属、传输与拓扑

> 中立事实。裁决在下表的 change，不在本目录。`01`/`04` 里的 axum、`rest.rs` 是现状代码，不是产品结论。

| 问题 | 证据 | 裁决 |
|---|---|---|
| 归属 / 连接方式 / 多窗写入 | `02-split`；开销 `c2300/research/overhead-eval.md` | c2300 |
| 线协议闭集（方法/帧载荷） | `03-paths` | c2301；外包四象限见 c2290 |
| 产品信封 + 默认拓扑 | DSH apiproxy / 2026-08-04 WS 下行 | **c2290**（四象限；POST 上行 + WS 下行；TUI attach） |
| 多会话 host / HTTP 栈 | `01-framework` `04-topology`；`c2302/research/framework-pick.md` | c2302（salvo **载体**，不再全双工 WS Command/Event） |
| embed + attach CLI | `04-topology` | c2303 |
| 符合性闸 | — | c2304（`InProcessClient` vs `HttpWsClient` 同方法表） |
| ACP | `acp-interop.md` | c2305（后置；译进方法表；不共用产品信封） |
| 薄 TS 客户端 | c2290 specta 闸之后 | **c2310**（无 SPA） |
| 方案 A loopback | c2290 design 曾记一节 | **c2315** |
| salvo oapi 调试文档 | type-sharing | **c2320**（非 SSOT） |
| 跨面公共动作 id | `docs/roadmaps/Web与TUI同源.md` | **delayed** `c2325`（client 登记表，不进 Host；Web 未开闸） |
| 吞吐 / 编码 / 工具链 | `05-throughput` | c2300 tagged JSON；c2301 不预埋二进制 |
| 浏览器壳 | `06-web` | 不开闸；契约不堵 WS 下行 |
| 跨语言类型 | c2290 `research/type-sharing.md` | **c2290 落地** specta `bindings.ts` 闸（无 Web UI）；薄 TS 客户端 / SPA 后置草案；不上 rspc/OpenAPI-first |

## 术语（后文每词只用一栏）

| 英文 | 中文 | 含义 |
|---|---|---|
| wire protocol | 线协议 | Command/Event 的 JSON 形状，`src/protocol/wire/` |
| tagged JSON | 带 `type` 的 JSON | `{"type":"prompt",...}` |
| Command | 命令 | 客户端 → host |
| Event | 事件 | host → 客户端 |
| encoding | 编码 | JSON / postcard 等字节格式；消息种类不变 |
| envelope | 信封 | 四象限 RPC 外包层（不是 JSON-RPC 2.0） |
| HTTP stack | HTTP 栈 | 路由 + WS upgrade |
| listener | 监听器 | TCP 或 UDS |
| UDS | Unix 域套接字 | 同内核 IPC；见下 |
| vsock | vsock | 宿主机 ↔ 虚拟机 |
| published port | 发布端口 | `docker run -p 127.0.0.1:H:P` |
| WebSocket | WebSocket | HTTP 升级后的双向连接 |
| reverse RPC | 反向 RPC | host 问客户端：审批 / 答题 |
| journal | 事件日志 | 带 `last_seq` 的环形缓冲 |
| bang | `!` / `!!` | 人发起的工作区 shell |
| host | 宿主 | 操作器进程。代码目录常叫 `server`，同义 |
| operator | 操作器 | ReAct、工具、MCP、工作区、bang |
| face-local | 面本地 | 键位、paint、剪贴板、TTY、`$EDITOR` |
| embed | 嵌入 | 同进程、同一契约自连 |
| attach | 连接 | 显式连已运行 host |
| launch & connect modes | 启动与连接方式 | embed 或 attach |
| session writer | 会话写者 | 一条 session 唯一写者 |
| REST | REST | 现状 `rest.rs`；不承载产品语义 |
| conformance gate | 符合性闸 | 同一 Command 表双实现同跑 |
| coarse sandbox | 粗粒度沙盒 | host 整进程进容器 |
| backpressure | 背压 | 慢客户端队列满时的等待/丢弃 |
| graceful shutdown | 优雅停机 | 先断连再退出 |

## UDS

TCP 听 IP:端口。UDS 听一个套接字文件（如 `/run/user/1000/xylitol/host.sock`），**同一内核**里 bind 与 connect 由内核接上，无网卡。可按 uid 挡同机他人。

不能跨机器、不能跨 VM：Docker Desktop 的容器在隐藏 Linux VM 里，Mac/Windows 上的 TUI 看见 `.sock` 也 `connect` 不上。那时用 TCP + 发布端口。同进程 embed 不需要 UDS；套接字给另一进程 attach。

不要 connect 普通文件；把 `.sock` 拷到另一台机器无效。
