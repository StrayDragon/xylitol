# c265-add-server-runtime — Design

> 前置设计：c260 design.md（层定义、词汇表、HC 约束、部署形态）是本设计的前置阅读材料。

## 1. 本 change 的目标

兑现 c260 design §0 的"部署形态 B"（server + 多 client）：
- **核心常驻**：`server/` 托管 agent + infra，一直跑。
- **交互可离线**：client 断连后核心继续工作，重连补齐事件。
- **统一交互**：InProcessDriver（本地）与 RemoteDriver（远程）事件序列一致。
- **开闭**：端口与方法集由 server 托管场景 TDD-reverse 定义，**不抄 AgentSession 全量 surface**。

## 2. HC-2 修正：SessionStore + EventSink port

c260 §2.3 自检表已认证这两个 port 为**真接缝**（test 内存双 + server 托管场景承重）。

### `SessionStore` port —— 只含 agent 真正调用的方法

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load_context(&self, session_id: &str) -> Result<SessionContext, String>;
    async fn append(&self, session_id: &str, entry: SessionEntry) -> Result<(), String>;
    async fn exists(&self, session_id: &str) -> bool;
}
```

> NOTE: 不包含 fork/navigate/switch/export/set_active_session/unlock 等 SessionManager 专有方法。这些是 interactive 组合根特权，不通过 port 暴露。

### `EventSink` port —— 只含 emit 方法

```rust
pub trait EventSink: Send + Sync {
    fn emit_lifecycle(&self, event: &AgentLifecycleEvent);
}
```

### facade HC-2 修正

- `Agent::new` 接收 `Arc<dyn SessionStore>` + `Arc<dyn EventSink>` 注入，不再强制 `AgentSession`
- `Agent::run(&mut self, prompt: &str)` 不再接收 `session_id`（session_id 在 interactive 层绑定 store）

AgentSession 作为全量状态容器保留（for 交互层的 fork/navigate/switch/export），但 facade 不强制它。

## 3. Driver 抽象

```rust
#[async_trait]
pub trait Driver: Send {
    async fn send(&self, cmd: Command) -> Result<EventStream, String>;
}
```

- `InProcessDriver` — 组合根注入 ports，直接调 agent（现有 cli 逻辑的薄包装）
- `RemoteDriver` — WS + REST

> NOTE: InProcessDriver 的单个实现是合理的 YAGNI 违反吗？是的。但它是一个**命名边界**。没有 Driver，interactive/ 就只能硬编码 import agent::facade，无法对等支持本地和远程。Driver 不引入新类型——它是交互层的"协议适配器"模式。

## 4. Server 进程化

参照 kimi-code 的 `packages/server` 设计：
- REST under `/api/v1`（control 命令）+ WS（事件流）
- 每 session 单调 `seq` + 环形 journal (保留最近 N 条，溢出推 `resync_required`)
- 单实例锁（`O_CREAT|O_EXCL`）
- 端口忙走 `port + 1` 重试
- 反向 RPC（approval/question 推送到 client，等待响应匹配 `call_id`）
- **不在 c265 中引入 HTTP/WS 框架**——先立 protocol + Driver + HC-2，server 进程化本身是下一个 change (c270) 的目标

## 5. 不做的事（yagni 边界）

- 不做 server 间分布式协调
- 不做 protocol 版本协商
- 不做 web/tui client 实现
- 不做 OAuth/provider attribution headers
- **不做 server 进程化**（c265 只做 port + Driver，server 是 c270）
