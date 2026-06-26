# c270-add-server-process — Design

> 前置设计：c260 design.md（层定义、HC 约束、部署形态）和 c265 design.md（port 边界、Driver 抽象）是本设计的前置。
> 参考实现：kimi-code `packages/server`（REST + WS 架构）、`packages/protocol`（线协议定义）。
>
> **c265 基础不完整，c270 必须先落地**：c265 的 T7/T8/T9/T12/T14 仅部分完成（见 proposal.md 的「c265 未完成项」表）。c270 在推进 server 进程化前必须先补完这些——尤其是 cli 组合根改用 `Agent::with_ports`、rpc 改用 Driver、arch guard interactive 断言。server 装配（build_agent）依赖这些完整后才能工作。

---

## 1. 总体架构

```
                        xylitol server
                     ┌─────────────────────┐
                     │  REST (/api/v1)      │  ← control 命令
                     │  WS (/ws/session/*)  │  → 事件流 + 反向 RPC
                     │  lock (文件锁)        │
                     │  agent (injected)     │
                     │  infra runtimes       │
                     └─────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        InProcessDriver  RemoteDriver    RemoteDriver
        (本地 CLI)        (WS tui)        (WS web)
```

### 数据流（单次 prompt）

```
client → [REST POST /api/v1/session/{id}/run] → server → agent.run(prompt)
client ← [WS event stream] ← server ← [AgentEvent 流]
client → [REST cancel] → server → cancel token
```

### 事件 journal 数据流

```
server.agent.run() → Event(seq=1) → journal[1] → WS client
                    Event(seq=2) → journal[2] → WS client (如果还在线)
client 断连 ❌
                    Event(seq=3) → journal[3] (累积, client 离线)
client 重连 ✅ → subscribe(id, last_seq=2)
                  ← journal[3..N] (回放) + 实时流
```

---

## 2. WS 帧协议

参照 kimi-code `server/ws/protocol.ts` 的帧格式，精简为 xylitol 风格：

```rust
/// Server → Client
pub enum ServerFrame {
    /// 连接建立后的第一条消息。
    ServerHello { version: String },
    /// 对 client 命令的即时确认（序列号、协议 id）。
    Ack { seq: u64, request_id: Option<String> },
    /// 事件推送（流式）。
    Event { session_id: String, seq: u64, event: protocol::Event },
    /// journal 截断，client 需全量重建 session。
    ResyncRequired { session_id: String },
}

/// Client → Server
pub enum ClientFrame {
    /// 订阅 session 事件流。
    Subscribe { session_id: String, last_seq: u64 },
    /// 对反向 RPC 的应答。
    ApproveTool { call_id: String, approved: bool },
    AnswerQuestion { call_id: String, answer: String },
    /// Ping 保活。
    Ping,
}
```

### 状态机

```
Client:    WS connect → send Subscribe → recv ServerHello + Ack
                                    → recv Event* 持续
                                    → (optional) recv ResyncRequired
                                    → resend Subscribe with updated last_seq
                                    → 断连 → (自动重连) → 同上

Server:    WS accept → recv Subscribe → send ServerHello + Ack
                                 → emit Event (来自 agent stream) → 逐个 Ack
                                 → journal 截断 → send ResyncRequired
                                 → 等待 client resubscribe
```

---

## 3. 反向 RPC（Approval / Question）

Conceptually a request from server to client during a turn:

```
agent.execute(tool)
  → agent emits ApprovalRequired { call_id, tool, summary }
  → server receives via EventSink
  → server looks up WS connections holding this session
  → server sends ServerFrame::ReverseRpc { type: "approval", call_id, ... }
  → client renders UI, user decides
  → client sends ClientFrame::ApproveTool { call_id, approved }
  → server resumes agent turn with the approval result
```

### 多 client 歧义处理（v1）

- 多个 WS connection 连同一 session 时，server 向 **所有** 持有者广播反向 RPC 请求。
- **第一个应答生效**，后续应答被忽略（通过 `HashMap<call_id, oneshot::Sender>` 原子性消费）。
- 应答超时（默认 60s）→ 标记 call_id 过期 → agent 收到 `ApprovalTimeout` 错误。
- NOTE: v1 无冲突仲裁。升级: 引入持有者锁或投票。

### call_id 生命周期

```
agent tool start → uuid4 call_id → server 注册 (call_id → oneshot tx)
                → 超时 60s → 超时错误 → agent 继续
                → 任一 client 应答 → server 发送 reply → agent 继续
                → 应答后清理 (remove from HashMap)
```

---

## 4. 单实例锁

借鉴 kimi-code `server/lock.ts` 的 `acquireLock`：

```rust
pub struct ServerLock {
    lock_file: PathBuf,
    handle: File, // held until drop → 文件锁自动释放
}

impl ServerLock {
    pub fn try_acquire(lock_path: &Path) -> Result<Self, ServerLockedError> {
        // O_CREAT|O_EXCL 原子创建，失败 → ServerLockedError
        // 写入端口号、PID、主机名
        // 返回 handle（drop 时删除锁文件）
    }
}
```

锁文件内容（JSON）：`{ "port": u16, "pid": u32, "hostname": String }`，方便识别陈旧锁。

### 端口重试

当 `listen()` 遇 `EADDRINUSE`（非我们的锁，说明端口被第三方占用）：
1. 尝试 `port + 1`
2. 更新锁文件中的端口号
3. 上限 `PORT_RETRY_LIMIT = 10`

---

## 5. Agent runtime 装配（server 组合根）

> ⚠️ 本步骤依赖于「c265 未完成项」中 T8 和 T9 的补完——cli 组合根必须改用 `Agent::with_ports`，rpc 必须改用 Driver。

server 的 agent 构造路径（第二个组合根，与 cli 并列）：

server 的 agent 构造路径（第二个组合根，与 cli 并列）：

```rust
// server/runtime.rs
pub fn build_agent(config: &AppConfig) -> (Agent, Arc<dyn EventSink>) {
    let store: Arc<dyn SessionStore> = Arc::new(
        SessionManager::new(config.server.sessions_dir()),
    );
    let sink: Arc<dyn EventSink> = Arc::new(EventBus::new());

    // 从 config 构造 model registry + tool registry
    let model_registry = build_model_registry(config);
    let tool_registry = ToolRegistry::from_tools(infra::tools::default_tools());

    let agent = Agent::with_ports(model_registry, tool_registry, ...);
    (agent, sink)
}
```

> NOTE: `Agent::with_ports` 已在 c265 中定义。server 通过它注入 SessionStore/EventSink，无需接触 AgentSession 内部。这是 c265 port 设计的核心 payoff。

---

## 6. Driver 远程路径

RemoteDriver 实现 `interactive::driver::Driver` trait：

```rust
pub struct RemoteDriver {
    rest_client: reqwest::Client,
    ws: WebSocketStream,
    base_url: String,
}

#[async_trait]
impl Driver for RemoteDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        // REST POST /api/v1/session/{id}/run
        // retrieve session_id from response
        // WS subscribe to session event stream
        // return merged stream
    }
    fn abort(&self) { /* REST cancel */ }
}
```

关键保证：**RemoteDriver 与 InProcessDriver 的事件序列一致**——对同一 prompt，两种 driver 产生相同顺序的 Event 变体（Delta → ToolStart → ToolEnd → AgentEnd）。这是 BDD 的核心断言。

---

## 7. 依赖清单

| crate | 版本策略 | 用途 |
|---|---|---|
| `axum` | latest, default-features | HTTP 路由 + 请求解析 + 统一 envelope |
| `tokio-tungstenite` | latest, features = ["connect", "server"] | WebSocket client + server |
| `tower-http` | latest, features = ["cors"] | CORS (web client 接入) |
| `reqwest` | latest (already in deps) | RemoteDriver REST client |

> NOTE: axum 与 tokio/tower 生态契合，非重型栈（默认最小 feature）。tokio-tungstenite 是纯 Rust、无绑定、与 tokio 原生集成的 WS 库。

---

## 8. 不做的事（YAGNI 边界，重申）

- **分布式协调**：每 server 独立核心。多实例通过 upstream proxy（nginx/haproxy）路由，server 本身不互相感知。
- **protocol 版本协商**：v1 冻结现有 Command/Event，新 variant 加 `#[serde(other)]` 兜底。版本升级通过 envelope `version` 字段协商。
- **auth/ 认证**：c270 server 不内置认证——假设运行在可信网络或本地。认证层（API key / OAuth）作为未来独立 feature。
- **web/tui client 实现**：c270 只做 server + RemoteDriver。web/tui 作为交互形态后续独立开发。
- **远程日志/监控**：server 日志走标准 stderr/tracing，不内置远程上报。

## 9. 任务溯源（对独立 agent 的导航）

下行表列出 c270 每项工作的原始定义位置，便于无上下文的 agent 快速定位前置设计：

| c270 任务 | 原始定义 | 设计决策位置 | 前置依赖 |
|---|---|---|---|
| 补完 c265 T7：facade HC-2 完全去 session_id | c260 T29 / c265 T7 | c260 design §6.3 / c265 design §2 | core::ports::SessionStore（c265 T1-T4） |
| 补完 c265 T8：cli 组合根改用 with_ports | c260 T30 / c265 T8 | c265 design §2 | Agent::with_ports（c265 T7） |
| 补完 c265 T9：rpc 改用 Driver | c260 T36-T37 / c265 T9 | c265 design §3 | Driver trait + InProcessDriver（c265 T11） |
| 补完 c265 T12：rpc 交互迁移 Driver | c260 T37 / c265 T12 | c265 design §3 | 同上 |
| 补完 c265 T14：arch guard interactive 断言 | c260 T31 NOTE / c265 T14 | c260 design §1 HC-1 / c265 design §2 | 无 |
| P1：HTTP/WS 框架 + 单实例锁 | c260 T38 / c260 T42 | c270 design §4 | 无（新依赖引入） |
| P2：protocol envelope + REST 路由 | c260 T33 (envelope) / c260 T40 | c270 design §2 | protocol.rs（c260 T33-T35） |
| P3：WS 帧协议 + seq + journal + resync | c260 T41 / c260 T43 | c270 design §2-3 | 同上 |
| P4：server 运行时装配 | c260 T39 | c270 design §5 | Agent::with_ports + 补完 T8 |
| P5：反向 RPC | c260 T44 | c270 design §3 | P3 WS 通道 |
| P6：RemoteDriver + CLI 子命令 | c260 T45 / c260 T46 | c270 design §6 | Driver trait（c265 T11） |
| P7：BDD server 场景 | c260 T47 | c270 proposal What Changes P7 | 所有上述实现 |
