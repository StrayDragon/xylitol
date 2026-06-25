# 方案 D — Server 常驻核心 + Protocol 统一交互

> 野心：核心常驻、交互解耦、水平可扩
> 风险：中-高（protocol 边界 + 重连/状态机设计是难点）
> 前置：方案 B（agent 足够薄，可被托管）；方案 C 的 port 收口**强烈建议先行**（server 才能干净地注入运行时）

## 1. 目标

兑现总览的 HC-2/HC-3/HC-4：

- **核心常驻**：`agent`+`infra` 由一个 `server` 进程托管，一直跑、一直工作。
- **交互可离线**：`interactive/*`（cli/tui/web）是 client，可随时断开/重连，断连期间核心继续工作，重连后补齐事件。
- **水平扩展 + 多控**：多个 server（每机/每工作区一个），client 用**同一套 protocol** 控制多个；实现"一控多"和"多控一"。
- **统一交互形态**：本地单进程与远程 server+client，client 代码完全相同，仅 Driver 实现不同。

参照实现：kimi-code 的 `packages/server` + `packages/protocol` + `apps/*` 三分。本方案是其架构在 xylitol 单 crate 下的落地。

## 2. 现状起点

当前 `interactive/rpc.rs` 已是 protocol 的**胚胎**：

```
stdin:  一个 JSON 对象/行（RpcCommand）
stdout: 一个 JSON 对象/行（RpcEvent，响应或流式事件）
stderr: 仅诊断
```

它已经具备"命令 + 事件 + id 回显"的雏形。方案 D 把它从"stdio-only、同进程"演进为"transport 无关、可本地可远程"的正式 `protocol/`。

> NOTE: 不要把 rpc.rs 推倒重写。它是已经验证过的交互契约，方案 D 是**抽出 + 泛化**：把它的类型搬到 `protocol/`，再把 stdio 传输抽成一个 transport 实现。

## 3. 目标形态

```
protocol/              ← 统一交互契约（SSOT）
├── command.rs         命令（client→core）：RunPrompt, Cancel, SwitchModel,
│                      SetThinking, ListCommands, ApproveTool, AnswerQuestion, ...
├── event.rs           事件（core→client）：TurnStart, ToolCall, Delta,
│                      TurnEnd, Usage, CompactionStarted, ApprovalRequired, ...
├── envelope.rs        统一信封 { code, msg, data, request_id }（REST）
├── error.rs           错误码
├── frame.rs           WS 帧：hello / ack / event / resync_required + 每 session seq
└── index.rs           re-export

server/                ← 常驻核心
├── mod.rs             start_server(opts) → RunningServer
├── rest.rs            REST 路由（/api/v1 前缀），control 类命令
├── ws.rs              WS 连接 + 帧协议 + 重连 resync
├── lock.rs            单实例锁（防第二实例）
├── runtime.rs         装配 agent + infra 运行时（组合根）
└── gateway/           approval/question 反向 RPC 桥（core→client 请求）

interactive/           ← client（原 interface/）
├── driver.rs          Driver trait（speak protocol）
│   ├── InProcessDriver   本地：直接调 agent
│   └── RemoteDriver      远程：WS/REST 到 server
├── cli/               命令行 + print 模式（用 InProcessDriver）
├── tui/               （未来）终端 UI（可用任一 Driver）
└── web/               （未来）web 客户端（用 RemoteDriver）
```

## 4. 三层契约

### 4.1 protocol 层（transport 无关）

protocol 只定义**说什么**，不定义**怎么传**。命令与事件是纯类型：

```rust
// protocol/command.rs
#[derive(Serialize, Deserialize)]
pub enum Command {
    Run { prompt: String, session_id: Option<String>, id: RequestId },
    Cancel { session_id: String },
    SwitchModel { model_id: String },
    SetThinking { level: ThinkingLevel },
    ApproveTool { call_id: String, approved: bool },   // 回答 approval 反向请求
    AnswerQuestion { call_id: String, answer: String }, // 回答 question 反向请求
    // ...
}

// protocol/event.rs
#[derive(Serialize, Deserialize)]
pub enum Event {
    TurnStart { session_id: String },
    Delta { session_id: String, text: String },
    ToolCall { call_id: String, name: String, args: Value },
    ApprovalRequired { call_id: String, tool: String, summary: String }, // 反向 RPC
    QuestionRequired { call_id: String, prompt: String },                // 反向 RPC
    TurnEnd { session_id: String, usage: Usage },
    // ...
}
```

> NOTE: 命令/事件用 `enum` 而非每个一个 struct + trait。一个 enum 是 SSOT，新增能力 = 加一个 variant，wire 向后兼容靠 `#[serde(other)]` 兜底或版本号。这比"一堆命令对象"更利于一眼看全交互面。

### 4.2 Driver 层（interactive 侧的唯一依赖）

interactive 不直接依赖 agent/server，只依赖一个 `Driver`：

```rust
// interactive/driver.rs
#[async_trait]
pub trait Driver: Send {
    /// 发命令，拿一个事件流（含最终响应）。
    async fn send(&self, cmd: Command) -> BoxStream<'static, Event>;
    /// 订阅某 session 的持续事件（重连后从 last_seq 续传）。
    async fn subscribe(&self, session_id: &str, last_seq: u64) -> BoxStream<'static, Event>;
}

pub struct InProcessDriver { /* 持有 Arc<Agent>，直接调 */ }
pub struct RemoteDriver { /* 持有 WS 连接 + REST client */ }
```

**关键不变量**：`interactive/cli`、未来的 `interactive/tui`、`interactive/web` 写出来的事件处理逻辑**逐字相同**——它们都 `match event { ... }`。本地/远程切换 = main 里换一个 Driver 构造。

### 4.3 server 层（常驻核心）

server 是**组合根**之一（另一个是本地 cli 的 InProcess 装配）。它做四件事：

1. **装配**：构造 `infra` 运行时，注入 `agent` 的 port，得到一个 `Agent`。
2. **托管**：持有 sessions，驱动 turn。
3. **暴露**：REST（control：发命令、查状态）+ WS（stream：事件流）。
4. **反向 RPC**：当 agent 需要工具审批/向用户提问时，通过 WS 把 `ApprovalRequired`/`QuestionRequired` 推给某个连着的 client，等待 client 回 `ApproveTool`/`AnswerQuestion`。

```rust
// server/runtime.rs — 组合根
pub fn build_agent(cfg: &AppConfig) -> Agent {
    let store = Arc::new(SessionManager::new(cfg.sessions_dir()));   // infra runtime
    let providers = cfg.models.iter().map(build_provider).collect(); // infra runtime
    let sink = Arc::new(EventBus::new());
    Agent::new(store, sink).with_providers(providers)               // 注入 port
}

// server/mod.rs
pub async fn start_server(opts: ServerOpts) -> Result<RunningServer> {
    let _lock = acquire_lock(&opts.lock_path)?;        // HC: 单实例
    let agent = build_agent(&opts.config);
    let app = mount_rest_routes("/api/v1", agent.clone());
    let ws = mount_ws_gateway(agent.clone());          // 含 approval/question 桥
    listen_with_port_retry(app, opts.port).await
}
```

## 5. 逐项执行细则

### 5.1 抽出 protocol（可提前，B 之后即做）

把 `interactive/rpc.rs` 的 `RpcCommand`/`RpcEvent` 类型**搬**到新 `protocol/` 模块，重命名/整理为 `Command`/`Event` enum + envelope + error code。rpc.rs 退化为"一个 stdio transport + InProcessDriver 的组合"。

**步骤：**
1. 新建 `src/protocol/`，`lib.rs` 加 `pub mod protocol;`。
2. 把 `rpc.rs` 的命令/事件类型搬到 `protocol/{command,event,envelope,error}.rs`。
3. `rpc.rs` 改为：解析 stdin → `Command`，用 `InProcessDriver` 执行，把 `Event` 序列化到 stdout。即 rpc.rs 变成 protocol 的**首个 transport 消费者**。
4. `interactive/print.rs`、`interactive/cli` 改用 `Driver` 抽象（先只有 InProcessDriver 实现）。

> NOTE: 这一步不引入 server，但已经把"统一交互契约"立起来了。RemoteDriver 和 server 可以随后任意加，interactive 不动。这是 HC-3 的最小兑现。

### 5.2 加 RemoteDriver + WS transport（C 之后做）

`protocol/` 是 transport 无关的，现在加第二个 transport：WebSocket。

1. `interactive/driver.rs` 加 `RemoteDriver { ws: WsClient, rest: RestClient }`。
2. `RemoteDriver::send`：命令小且需响应→走 REST；流式事件→开 WS 订阅。
3. `RemoteDriver::subscribe`：连 WS，发 `{ session_id, last_seq }`，收 `Event` 流。

验证开闭：**InProcessDriver 的任何代码不改**，新增了一种交互形态（远程）= 新增一个 Driver impl。

### 5.3 server 进程化（依赖 C 的薄 agent）

1. 引入 HTTP/WS 后端（建议 `axum`，生态成熟、与 tokio 契合）。
2. `server/runtime.rs` 做 agent+infra 装配（组合根）。
3. `server/rest.rs`：`/api/v1` 下挂 control 路由（run/cancel/switch-model/list），统一 envelope。
4. `server/ws.rs`：WS 帧协议（`hello`/`ack`/`event`/`resync_required`），每 session 维护单调 `seq`。
5. `server/lock.rs`：单实例锁（启动先 acquire，第二实例报 `ServerLockedError`）；端口占用走 port-retry。
6. CLI 加子命令 `xylitol server run` / `server install`（OS 服务化，仿 kimi-code）。

### 5.4 重连与离线工作（server 的核心卖点）

- server 每个 session 维护事件 `seq`（单调递增）+ 事件 journal（环形/落盘）。
- client 断连期间，server 继续跑 turn、写 journal。
- client 重连：`subscribe(session_id, last_seq)` → server 从 journal 回放 `last_seq+1..` 之后的事件，再续实时流。
- 若 journal 已被截断超出 `last_seq`，server 推 `resync_required`，client 全量拉取 session 状态重建。

> NOTE: 这是"交互可离线"的技术内核。journal 的保留窗口是可调旋钮（`// NOTE: 默认保留最近 N 条，溢出触发 resync`），不要一上来做无限保留。

### 5.5 反向 RPC（工具审批 / 用户提问）

coding agent 必须能在执行中**向 client 请求决策**（工具是否放行、向用户问澄清）。这是 server→client 的请求：

1. agent 执行 tool 前，若需审批，发 `ApprovalRequired` 事件（带 `call_id`）。
2. server 把它经 WS 推给"持有该 session 的 client"，挂起 turn 等待。
3. client 弹审批 UI，用户选择后回 `ApproveTool { call_id, approved }`。
4. server 用 `call_id` 匹配挂起的 turn，恢复执行。

`interactive/` 侧需要一个 `reverse-rpc` 适配（kimi-code 同名）：把 `ApprovalRequired`/`QuestionRequired` 转成 UI 数据形状，把用户选择转回 `ApproveTool`/`AnswerQuestion`。

### 5.6 水平扩展 / 多控

- 每个 server 实例 = 一个工作区/一台机的核心，单实例锁保证不冲突。
- 一个 client 可连多个 server（`RemoteDriver` 持有多连接，按 `server_id` 路由）→ **一控多**。
- 多个 client 连同一 server（server 用 `seq` + journal 让多端看到一致事件流）→ **多控一**。
- protocol 完全相同，server 端不关心 client 是 tui 还是 web。

> NOTE: 水平扩展不需要"分布式 agent 协调"。每个 server 是独立的核心，扩展 = 多起几个 server。协调（如果有）属于未来的 orchestration 层，**现在不做**（HC-5：YAGNI）。

## 6. 迁移映射表

| 操作 | 源 | 目标 | 阶段 |
|---|---|---|---|
| 抽出 | `interactive/rpc.rs` 的类型 | `protocol/{command,event,envelope,error}.rs` | D-1 |
| 退化 | `interactive/rpc.rs` | stdio transport + InProcessDriver 包装 | D-1 |
| 新增 | — | `interactive/driver.rs`（Driver trait + InProcessDriver） | D-1 |
| 新增 | — | `interactive/driver.rs::RemoteDriver` + WS transport | D-2 |
| 新增 | — | `server/`（runtime/rest/ws/lock/gateway） | D-3 |
| 新增 | — | 重连 journal + `resync_required` | D-4 |
| 新增 | — | 反向 RPC（approval/question gateway） | D-5 |
| 新增 | — | 多 server 路由（一控多） | D-6 |

## 7. 验证清单

- [ ] `grep -rn "crate::agent\|crate::infra" src/interactive/` 仅出现在 `driver.rs` 的 InProcessDriver 装配点（组合根特权）
- [ ] `interactive/cli` 与（未来的）`interactive/tui` 共用同一份事件 `match` 逻辑
- [ ] 同一段 client 代码，接 InProcessDriver 与接 RemoteDriver 行为一致（用 trait + 同测试用例验证）
- [ ] 断连→server 继续工作→重连后事件无丢失（journal 回放测试）
- [ ] server 单实例锁生效（第二实例报错）
- [ ] 工具审批经反向 RPC 在 client 弹窗、回传后恢复 turn
- [ ] 新增一个 LLM 厂商 = 仅 `infra/provider/` 加文件 + 组合根注册，server/interactive 零改动
- [ ] `cargo build --all-features` / `nextest` / `clippy -D warnings` / BDD 全绿

## 8. 风险与回滚

- **风险点 1**：protocol 边界设计。enum 一旦发布 wire 兼容性就锁住。**先用现有 rpc.rs 的命令集冻结 v1**，不臆造命令；新命令随真实需求加。
- **风险点 2**：重连状态机（seq + journal + resync）易出 off-by-one。建议 D-4 配 property-based 测试。
- **风险点 3**：反向 RPC 的"哪个 client 应答"在多控一时有歧义。v1 规则：**最先注册的持有者应答**，或**任意一个应答即生效**。先取后者（简单），写进 NOTE 标天花板。
- **风险点 4**：server 依赖第三方 HTTP/WS 框架（axum）。先确认与现有依赖（tokio 已有）契合，避免引重型栈。HC-5：能复用就不加新重型依赖。
- **回滚**：D-1/D-2 不引入 server，可独立交付；D-3 起才需要 server 进程，按 PR 粒度 revert。

## 9. 推荐路线

| PR | 内容 | 阶段 | 依赖 |
|---|---|---|---|
| D-1 | 抽 protocol（rpc.rs 类型搬家 + Driver 抽象 + InProcessDriver） | 5.1 | 方案 B |
| D-2 | RemoteDriver + WS transport | 5.2 | D-1 + 方案 C |
| D-3 | server 进程化（rest/ws/lock + `server run`） | 5.3 | D-2 |
| D-4 | 重连 journal + resync | 5.4 | D-3 |
| D-5 | 反向 RPC（approval/question） | 5.5 | D-3 |
| D-6 | 多 server 路由（一控多） | 5.6 | D-2 |

> NOTE: D-1 是最高性价比的一步——它独立于 server，却把"统一交互契约"立住了。即便永远不做 server，protocol + Driver 抽象也让 interactive 与 agent 彻底解耦，未来加 tui/web 零成本。建议 B 之后立即做 D-1。
