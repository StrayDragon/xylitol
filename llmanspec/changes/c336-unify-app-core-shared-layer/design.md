# c336 Design — 共享 bootstrap 与 dispatch 的接口设计

## 核心张力

rpc.rs 的 `dispatch(state: &Arc<Mutex<RpcState>>, cmd: Command)` 依赖 `RpcState`——一个持有 `ModelRegistry`、`SessionManager`、`current_model_id`、`thinking_level`、`compaction_*`、`cached_agent`、`active_cancel` 等约 15 个字段的状态结构。这是因为 rpc 模式按命令逐条处理、agent 按需重建（session 自动持久化后下次命令重建）。

而 TUI 持有的是 `&mut dyn Driver`，`Driver` trait 只有 `run(&mut self, prompt) -> EventStream` 和 `abort(&self)` 两个方法。**这个 trait 太窄，承载不了 SetModel/Compact/GetState/GetAvailableModels/ExportHtml 等命令的执行语义**——它们需要访问 agent 内部状态（current_model、registry、session），而 Driver 把 agent 藏在实现后面。

## 设计决策：扩展 Driver trait（而非引入新 trait）

### 选项 A（采纳）：扩展 `Driver` trait，补齐命令执行方法

给 `Driver` trait 增加方法，让每个 Command 变体的执行语义落到 Driver 实现上：

```rust
pub trait Driver {
    async fn run(&mut self, prompt: &str) -> EventStream;
    fn abort(&self);

    // 新增（覆盖共享 dispatch 需要的命令语义）
    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, DispatchError>;
    fn cycle_model(&mut self) -> Result<ModelInfo, DispatchError>;
    fn current_model(&self) -> Option<ModelInfo>;
    fn available_models(&self) -> Vec<ModelInfo>;
    fn compact(&mut self) -> Result<(), DispatchError>;
    fn get_state(&self) -> SessionState;
    fn export_html(&mut self, path: &str) -> Result<(), DispatchError>;
    fn export_jsonl(&mut self, path: &str) -> Result<(), DispatchError>;
    fn import_jsonl(&mut self, path: &str) -> Result<(), DispatchError>;
    fn set_thinking_level(&mut self, level: ThinkingLevel);
    // ... 其余非 WS 专属变体
}
```

`app::core::dispatch` 变成一个**纯分派器**：

```rust
pub async fn dispatch(driver: &mut dyn Driver, cmd: Command) -> Result<DispatchOutcome, DispatchError> {
    match cmd {
        Command::SetModel { model_id, .. } => {
            let info = driver.select_model(&model_id)?;
            Ok(DispatchOutcome::Model(info))
        }
        Command::CycleModel { .. } => Ok(DispatchOutcome::Model(driver.cycle_model()?)),
        Command::Abort { .. } => { driver.abort(); Ok(DispatchOutcome::Aborted) }
        // ...
    }
}
```

`InProcessDriver` 实现这些方法时直接调内部 `self.agent.select_model(...)`；`RemoteDriver` 实现时发 REST 请求（`POST /api/v1/session/{id}/model`）。**rpc 和 tui 各自构造好 Driver 实例，共享 dispatch 只负责「Command → Driver 方法调用」的映射**。

**优点**：
- 共享 dispatch 极薄（纯 match，无状态），易测。
- Driver 的两个实现（in-process/remote）天然覆盖「本地 agent」和「连 server」两种拓扑——这正是想法 2（TUI 连 server）需要的。
- 不引入第三种抽象（RpcState 是 rpc 特有的「无状态按命令重建」模式，不该泄漏到共享层）。

**代价**：Driver trait 方法数增加（从 2 到约 12）。但这些都是 agent 命令的本征操作，不是凑数。

### 选项 B（否决）：让共享 dispatch 接收 `&mut ReActAgent` + 周边 state

让 dispatch 直接操作 `ReActAgent`（`select_model`/`compact`/`current_model` 都是 ReActAgent 的公开方法）+ 把 RpcState 的非 agent 字段（thinking_level、compaction）作为额外参数传入。

**否决理由**：
- `ReActAgent` 是 `agent` 层类型，`app::core::dispatch` 接收它意味着 dispatch 直接依赖 agent 内部——但 `app/core/` 的 seam 约束是「经 Driver trait，不 reach into agent」。la19 明令 application surfaces 不得直接持 Driver trait 之外的 agent 入口。
- RemoteDriver 场景下根本没有 `ReActAgent`（agent 跑在远端 server），dispatch 接收 `&mut ReActAgent` 就无法复用于 remote 模式——直接断了想法 2 的路。
- rpc 的 `RpcState` 是「无状态重建」专用结构（cached_agent + cache_session_id 的缓存逻辑），把它抬到共享层会强迫 TUI 也采用「无状态重建」模型，而 TUI 是有状态长连接（持有一个 driver 跑到退出）。

### 选项 C（否决）：新增独立 `CommandExecutor` trait 与 Driver 并列

不动 Driver，另起一个 trait 专门承载命令执行。

**否决理由**：抽象冗余。Driver 的语义就是「应用面与执行后端之间的边界」，命令执行本就是这层边界的职责。拆成两个 trait 会让每个执行后端（InProcessDriver/RemoteDriver）实现两套相关 trait，且共享 dispatch 要同时收两个 trait object，调用点变复杂。Driver trait 扩展是更内聚的选择。

## rpc 的 id 语义处理

rpc 的每个 Command 带 `id: Option<String>`，响应里 echo 回去（`Event::Response { id, payload }`）。TUI 没有 id 语义（同步交互，不需要请求关联）。

**处理**：共享 dispatch 的签名是 `dispatch(driver, cmd) -> Result<DispatchOutcome, DispatchError>`，**不处理 id**。rpc 在调 dispatch 前提取 id、在 dispatch 返回后用 id 包 `Event::Response`：

```rust
// rpc.rs
let id = extract_id(&cmd);  // rpc 专属，留在这里
let outcome = app::core::dispatch(&mut *driver, cmd).await;  // 共享
emit_response(id, outcome);  // rpc 专属，留在这里
```

id 是传输层关注点（stdio JSONL 的请求关联），不属于命令执行语义，正确地留在 rpc.rs。

## rpc 的 RpcState 怎么办

rpc 的 `RpcState` 不能直接删——它的「无状态按命令重建 agent + 缓存」逻辑是 rpc 模式特有的（stdin 可能在两次命令间间隔很久，session 已持久化，重建更省内存）。但重建出的 `ReActAgent` 会被包进一个 Driver 实例。

**路径**：rpc 仍保留 RpcState 做「按命令重建/缓存 agent」，但重建后把 agent 包进 `InProcessDriver`，然后调共享 dispatch。即 rpc 内部用 InProcessDriver 作为 dispatch 的执行后端（而不是直接持 agent）。这样 rpc 的状态管理逻辑（缓存、重建）保留，命令执行语义下沉。

> 注意：rpc 的 `RpcState` 与 `InProcessDriver` 内部都持 `ReActAgent`，会有「两个壳包同一个 agent」的过渡。可在阶段 2 后端做一轮清理（让 RpcState 直接持 `InProcessDriver` 而非裸 `ReActAgent`），但属本变更范围外的整理，不强制。

## bootstrap 的 resource 发现对 server 的影响

server/runtime.rs 当前**跳过** context_files / append_system_prompt（注释「headless; resource discovery is the caller's job」）。切换到共享 bootstrap 后会**补回**这些发现。

**这是行为变更**（不是纯重构）：之前 server 跑的 agent 不含 AGENTS.md 内容，切换后会含。需在 tasks 1.6 显式验证：在含 AGENTS.md 的 cwd 启动 server 跑 prompt，确认 system prompt 注入了 AGENTS.md 内容。

如果用户场景里 server 明确需要 headless（如远程纯计算），可在 `BootstrapInput` 加 `resource_discovery: ResourceDiscoveryMode::{Enabled, Skip}` 开关——但**默认 Enabled**（与 print 一致），server 若要 headless 显式传入 Skip。先不预制开关，等真有 headless 需求再加（YAGNI）。

## 不在本变更范围

- server/ws.rs 协议靠拢 protocol SSOT（`ClientFrame`/`ServerFrame` 与 `Command`/`Event` 的统一）——属 c345 范畴，待 remote/web 客户端真启动时做。
- TUI 交互组件（审批浮层、命令面板等）——独立变更。
- TUI `--remote` 模式激活 RemoteDriver——独立变更。
