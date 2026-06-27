# c278-slim-agent-session-and-merge-export Design

> 精简 `agent/session`：合并 export 转发层、把 `SessionManager`/`EventBus` 具体持有收敛为
> `SessionStore`/`EventSink` 注入端口、评估 AgentSession 瘦身。目标：白名单 **12 → 0**，
> arch_guard 进入"全量扫描、零豁免"终态，agent/ 不再直接 import 任何 `crate::infra::*` 具体类型。

## 背景

c279 完成后，arch_guard 白名单剩 12 项，**全部**落在 c278，集中在 `agent/session` 子系统：

| 文件 | 违规 import | 数量 |
|---|---|---|
| `agent/session/export.rs` | `infra::session::manager` + `infra::session::export::*` ×5 | 6 |
| `agent/session/mod.rs` | `infra::event` + `infra::session::manager` | 2 |
| `agent/session/events.rs` | `infra::event` | 1 |
| `agent/compaction/{mod,orchestrator}.rs` | `infra::session::manager` | 2 |
| `agent/facade.rs` | `infra::session` | 1 |

根因是 c277 注入了 `store: Arc<dyn SessionStore>` 与 `sink: Arc<dyn EventSink>` 两个端口，
但 `AgentSession` **同时仍冗余持有** `session_manager: SessionManager` 与 `event_bus: EventBus`
两个具体 infra 类型。c278 收敛这层冗余。

## 设计原则

1. **零行为变更**：仅移动代码、扩容 port、改注入路径；export/compaction/abort 的可观察行为不变。
   BDD 与现有 539 lib 测试不感知差异。
2. **port 最小集（选项 A）**：`SessionStore` 只新增 compaction/export 真正调用的方法，**不**把
   `fork`/`navigate_tree`/`switch_session` 等管理操作全量上提（那会扩大 port 并泄漏 infra 抽象）。
   管理操作保留在 `SessionManager`，由组合根在需要处直接装配。
3. **组合根统一装配**：CLI / RPC / server 构造 `SessionManager` 并将其既作为 `SessionStore` trait
   object 注入 AgentSession、又作为具体类型供 IO/管理操作使用（见"SessionManager 去留"）。
4. **不向后兼容**：按项目约定，旧调用点一次性更新到新签名，不留 shim。

## 方案

### P1 — 合并 export 转发层（消 6 项白名单）

`agent/session/export.rs` 是纯一跳 wrapper：4 个 `pub async fn` 内部全部
`manager.load(sid).await?` 后调 `infra::session::export::{render_html,render_jsonl,write_to,...}`，
外加 `share_as_gist` 直接转发。它存在的唯一理由是"包一个 `&SessionManager`"。

#### 步骤

1. **删除 `agent/session/export.rs`**。
2. `AgentSession::export_to_html` / `export_to_jsonl` / `import_from_jsonl` / `share_as_gist`
   （`session/mod.rs:804–829`）改为内联实现，经注入的 `self.store.load_entries(sid).await?`
   拿到 `Vec<SessionEntry>` 后直接调 `crate::infra::session::export::render_html(...)`。
3. 唯一外部调用方 `interactive/rpc.rs:460`（`agent.session_mut().export_to_html(&path)`）签名不变。

> **注意**：`AgentSession` 调 `infra::session::export::*` 仍是 agent→infra import，但
> `infra::session::export` 的 `render_html`/`parse_jsonl`/`write_to` 是**纯函数转换层**
> （无 SessionManager 依赖），与 c276 上提的 core 词汇同性质。为彻底归零，这些纯函数应在
> P1 末尾**上提到 `core::session_export`**（纯 IO-free 转换），使 `agent/` 经 core 调用。
> 若上提成本超出本次范围，则保留对 `infra::session::export` 的 import 并在白名单中
> 以 `c278` 显式标注"纯函数转发，待 core 化"——**优先尝试上提**。

### P2 — SessionManager / EventBus 具体持有 → port（消 6 项白名单）

#### P2.1 — SessionStore port 扩容（最小集 A）

当前 `core::ports::SessionStore`（c272 引入）只有：
```rust
async fn load_context(&self, sid: &str) -> Result<Vec<AgentMessage>, String>;  // ReAct loop 用
async fn append_entry(&self, sid: &str, entry: serde_json::Value) -> Result<(), String>;
async fn exists(&self, sid: &str) -> bool;
```

compaction 与 export 实际需要的是 **SessionEntry 语义** + **会话上下文构建**：

| 调用点 | 需要的方法 | 现状 |
|---|---|---|
| `compaction/mod.rs:12` | `load(sid) -> Vec<SessionEntry>` | ❌ 缺 |
| `compaction/mod.rs:116` | `append(sid, &SessionEntry)` | `append_entry` 收 `Value`，类型不符 |
| `orchestrator.rs:78` | `build_session_context(sid) -> SessionContext` | ❌ 缺 |
| `export.rs` import 路径 | `append(sid, &SessionEntry)` | 同上 |

**扩容方案**（`core::ports`，`SessionEntry`/`SessionContext` 已在 `core::session_types`）：
```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    // 既有（ReAct loop）
    async fn load_context(&self, sid: &str) -> Result<Vec<AgentMessage>, String>;
    async fn append_entry(&self, sid: &str, entry: serde_json::Value) -> Result<(), String>;
    async fn exists(&self, sid: &str) -> bool;

    // 新增（compaction / export）— 最小集 A
    async fn load_entries(&self, sid: &str) -> Result<Vec<SessionEntry>, String>;
    async fn append_session_entry(&self, sid: &str, entry: &SessionEntry) -> Result<(), String>;
    async fn build_session_context(&self, sid: &str) -> Result<SessionContext, String>;
}
```
`SessionManager` 实现 trait 时，`load_entries`/`append_session_entry`/`build_session_context`
直接转调既有同名方法。

#### P2.2 — 消除 `session_manager` 具体字段

- `AgentSession.session_manager: SessionManager` 字段删除。
- 所有读路径（`exists`/`load`/`append`/`build_session_context`/export）改走 `self.store.*`。
- `AgentSession::session_manager()` accessor 已确认**零外部调用方**（`rg` 无命中），直接删除。
- `facade.rs::with_ports` 的 `session_mgr: SessionManager` 参数：改为组合根在内部构造
  `SessionManager` 并 `Arc<dyn SessionStore>`-ify 后注入；`with_ports` 不再收具体类型。
- compaction：`compact_session(mgr: &SessionManager, ...)` → `compact_session(store: &dyn SessionStore, ...)`；
  `CompactionOrchestrator::compact/maybe_auto_compact` 的 `session_manager: &SessionManager`
  参数 → `store: &dyn SessionStore`。

#### P2.3 — 消除 `event_bus` 具体字段 + 死 API

- 删除 `AgentSession.event_bus: EventBus` 与 `lifecycle_handle: Option<UnsubscribeHandle>`。
- 删除 `agent/session/events.rs` 的 `event_bus()` / `subscribe()` / `unsubscribe()`（死 API，
  零外部调用方）。该文件若无其他内容则整体删除。
- `abort()` 内的 `self.event_bus.emit_lifecycle(&AgentEnd{...})` 改写——见下"abort async 权衡"。

#### abort async 化权衡（关键风险点）

`abort()` 当前 sync，调用方 5 处：`rpc.rs:600` / `driver.rs:75` / `server/rest.rs:104`
均经 `Driver::abort()` 或直接调用。**`Driver::abort(&self)` 是 sync trait method**
（`interactive/driver.rs`），且 `server/rest.rs` 在 `lock().await` 后调 sync abort。

| 方案 | 改动面 | 取舍 |
|---|---|---|
| **A. abort 保持 sync，fire-and-forget emit** | `abort()` 内 `tokio::spawn` 一个 `sink.emit(AgentEnd{aborted}).await` | ✅ 不波及 Driver trait；⚠️ emit 可能丢失（abort 是终态，可接受） |
| B. abort 改 async | `Driver::abort` 也改 async + 所有 impl | ❌ 波及 interactive trait 契约，超出 c278 范围 |
| C. 保留 sync EventBus 仅作 abort emit | event_bus 字段不删 | ❌ 白名单无法归零，违背 c278 目标 |

**选 A**：`abort()` 保持 sync，内部用 `tokio::spawn` fire-and-forget 向 `sink` 发 `AgentEnd{reason:"aborted"}`。
`dispose()` 不 emit（现状如此，仅 drop handle → 改为无操作或保留 `abort_bash` + 清队列）。
design 记录：lifecycle 事件在 abort 路径为 best-effort；若未来需可靠投递，再单独提案改 async。

### P3 — AgentSession 瘦身评估（非阻塞，本变更内尽力）

`agent/session/mod.rs` 37k 字符，仍背负 stats / bash exec handler / steering 等非编排职责。
本变更评估（不强制全部落地）：
- `session/stats.rs`、`session/steering.rs` 已是独立文件，确认是否可整体迁出 agent 或仅做字段收敛。
- `save_trust_decision`（c279 已改 `&dyn TrustStore`）等 handler 方法的归属再确认。

**P3 不影响白名单归零**，仅作职责收敛。若某项拆分会显著扩大改动面，则记入 `future.md`，
不阻塞 c278 收尾。

## SessionManager 去留

`SessionManager` 的**管理操作**（`fork`/`navigate_tree`/`switch_session`/`create`/`list`/
`get_tree` 等）仍需被组合根或 `agent/session/io.rs`（IO wrapper）使用。决策：

- `AgentSession` **不**再持有 `SessionManager`，只持 `Arc<dyn SessionStore>`。
- **管理操作路径**（`io.rs` 的 `load_context`/`append_entry`/`exists` 已是 store 转发；
  未来的 fork/navigate 等命令）在**组合根**或 `interactive` 层直接持有 `SessionManager`
  调用，不经 `AgentSession`。即：AgentSession 回归"编排状态持有者"（HC-2），管理命令
  不经过它。

这一步若发现 `io.rs` 的某些方法被 react loop 经 AgentSession 调用，则把该方法上提到
`SessionStore` 或在调用点改用 `store`。本变更在实现阶段逐一核对 `io.rs`。

## 风险矩阵

| 风险 | 影响 | 缓解 |
|---|---|---|
| abort fire-and-forget emit 丢失 | lifecycle 订阅者偶发漏 AgentEnd | 接受（abort 终态）；记 future |
| SessionStore 扩容遗漏某 SessionManager 方法 | compaction/export 编译失败 | 实现时以编译器为向导逐个补 |
| `infra::session::export` 纯函数上提到 core 失败 | P1 白名单无法完全归零 | 回退：保留 import 并显式标注 |
| io.rs 管理方法经 AgentSession 调用 | 改动面扩大 | 核对后逐方法上提 port 或下沉组合根 |
| compaction 测试依赖具体 SessionManager | 测试编译失败 | 测试改用 `SessionManager`（impl SessionStore）实例传入 |

## 验证策略

1. `cargo nextest run -p xylitol --lib`（≥539 测试全绿）。
2. `cargo test bdd -- --test-threads=1`（88 BDD 全绿）。
3. `arch_guard` 3/3 绿，且 `AGENT_INFRA_ALLOWLIST` **清空为 `&[]`**。
4. `rg 'crate::infra::(session|event)' src/agent/`（production 区）**零命中**。
5. `cargo clippy --lib` 不新增 error；`cargo fmt --check` 通过。
6. API baseline snapshot 更新（`with_ports` / `new` / `abort` 签名变化）。

## Out of Scope

- `fork`/`navigate_tree`/`switch_session` 的 port 化（保留在 SessionManager，组合根调用）。
- `Driver::abort` async 化（单独提案）。
- stats/steering 模块物理迁出 agent（仅 P3 评估，非强制）。
