# src/ 核心相对产品 TUI 就绪度与命名债

> **性质**：只读审计 + 命名探索（非规范、非 AGENTS）。
> **日期**：2026-07-10
> **分支语境**：`feat/tui-dev`；轨 A（包/`agent_demo`）已收口；产品 `src/app/tui` 仍冻结。
> **稳定边界仍以**：`src/AGENTS.md`、`src/app/tui/AGENTS.md`、`packages/xylitol-tui/AGENTS.md`、归档 design 为准。
> **进度板**：根 `_HANDOFF.md`（易腐；本文件偏「证据与建议」）。

---

## 0. 总判

| 问题 | 结论 |
|---|---|
| Core 是否跟得上轨 A？ | **骨架跟得上，消费面跟不上。** |
| 要不要先再挖一整套 agent？ | **不要。** 先修 P0 seam / 合流，再开 c465。 |
| 命名要不要大 refactor 挡开闸？ | **不要。** 文档别名 + 小 API（`QueueStats`）优先；大 rename 另开 SDD。 |

**一句话**：`XyEvent` / `InProcessDriver` / c461 队列语义 / `dispatch` / `execute_bash` / edit `display_diff` **代码在、产品未接**；最大阻塞是 `run(_driver)` 零消费、`QueueUpdate` 双通道、若干事件空转、会话树 Driver API 不全。

---

## 1. 分层 inventory（对 TUI）

| 层 | 路径 | 对 TUI 的职责 | 现状 |
|---|---|---|---|
| domain | `lifecycle.rs`, `session_types.rs` | `XyEvent`；`SessionEntry`（`parent_id`） | 枚举齐；部分变体无发射者 |
| runtime_protocol | ports | agent↔infra | 最小 store；树导航不在 port |
| agent | `react.rs`, `session/`, `hooks` | ReAct 流、队列、fork/bash | c461 InProcess Ready；EventBus 分裂 |
| infra | tools, session, trust, bash, event | 实现 | 能力在；多数未暴露到 Driver |
| protocol | `command.rs`, `event.rs` | 线协议 | Command 覆盖好；`QueueUpdate` 不上线 |
| app/core | `driver`, `dispatch`, `composition`, `bootstrap` | **唯一合法 seam** | InProcess 完整；TUI 未调用 |
| app/tui | `host`, `ui_root`, `mod` | host + stub | **冻结**；`run(_driver)` 丢弃 Driver |
| packages/xylitol-tui | 引擎/组件 + `agent_demo` | MVP 原子 | 齐；不接真实 Driver |

依赖：应用面只经 `crate::app::core` / `crate::agent` mod 级（`src/AGENTS.md` + `arch_guard`）。

---

## 2. 事件与流（c465 关键）

### 2.1 谁消费 `Driver::run`？

```rust
// src/app/core/driver.rs
async fn run(&mut self, prompt: &str) -> EventStream; // Item = XyEvent
```

| 消费者 | 状态 |
|---|---|
| `app/cli/print.rs` | **唯一完整** EventStream 渲染器 |
| `app/server/*` | 部分 |
| `app/tui/mod.rs::run(_driver)` | **参数未使用** |
| `agent_demo` | 假脚本，**不接** `XyEvent` |

### 2.2 `XyEvent` 发射矩阵（摘要）

| 变体 | ReAct yield | EventBus | 备注 |
|---|---|---|---|
| Text/Thinking deltas, Message*, Tool Start/End, Turn*, AgentEnd, Error | 是 | — | 主路径 Ready |
| ToolExecutionUpdate | **从不** | — | 死类型 |
| QueueUpdate（drain 后） | 是 | — | 流内可见 |
| QueueUpdate（入队/abort 清 steer） | — | `emit_queue_update` → sink | **生产路径无人 `on_lifecycle`** |
| Compaction* | — | orchestrator | print 流收不到 |
| AutoRetry* / AgentStart | **从不** | — | UI 不可见 |

### 2.3 P0：QueueUpdate 双通道

```text
入队/清队 ──► AgentSession::emit_queue_update ──► EventBus (无人订)
drain 后  ──► react yield XyEvent::QueueUpdate ──► Driver::run 流
```

只听 EventStream 的 bridge **看不到**「流中 Enter 入队」的即时 `steer:N`。
`protocol::Event`：**不**映射 `QueueUpdate`（internal）。

### 2.4 `display_diff`

- 写入：`infra/tools/edit.rs` → `ToolExecutionEnd.result` 为 **整段 JSON 字符串**。
- 字段：`success`, `path`, `diff`, `display_diff`, `strategy`, `edit_count`。
- Bridge 须按 `name == "edit"` + `serde_json` 解析（可行但脆弱；无 typed 顶层字段）。

---

## 3. 队列 / abort（c461）

| 决策 | 代码 | 判读 |
|---|---|---|
| abort 清 steer、留 follow_up | `ReActAgent::abort` → `clear_steer_queue`；单测存在 | **Ready** |
| `steer` / `follow_up` / `clear_queue` / `queue_stats` | `Driver` trait | **Ready（未接线）** |
| follow_up 文案 restore | 仅 `(steer, follow_up)` 计数 | **Partial**：UI 自持快照或扩 peek API |
| RemoteDriver 队列 | 一律 `Err(...not yet implemented)` | Stub；MVP InProcess 可接受 |
| QueueMode 默认 | **All**（非文案 one-at-a-time） | Partial / 文档对齐 |

`_HANDOFF` 标 c461 ✓：对 **InProcess 语义**成立；若理解为「TUI 可直接靠 EventStream 做 queue chrome」则 **oversell**。

---

## 4. slash / Command（c480）

- `protocol::Command`：Abort / SetModel / Steer / FollowUp / Bash / Quit / Prompt… 覆盖好。
- `dispatch.rs`：映射齐；**故意不处理** Prompt/Quit（调用方责任）。
- 产品 TUI：**零调用** dispatch。
- 陈旧注释仍提已删 `tui/commands.rs`。
- Builtin 名 `quit` vs 产品文案 `/exit` → 需别名。

---

## 5. bash / tools / session / trust

| 能力 | 状态 | 证据 |
|---|---|---|
| `execute_bash` | Ready | Driver → Agent → InfraBashExecutor |
| edit `display_diff` | Partial（形状） | JSON-in-string |
| `get_messages` / `fork_session` / `switch_session` | Ready | Driver；未接线 |
| list / branch 树 API | Gap | `SessionManager` 有，Driver 无 |
| TrustManager | Ready | `infra/trust`；bootstrap `on_prompt = |_| None` |
| 默认 permission | 实质 yolo | `allow_all` 除非 config 开闸 |
| RemoteDriver | Stub | 非 MVP |

---

## 6. 风险矩阵

| Area | Status | Blocks | Sev |
|---|---|---|---|
| TUI 消费 `Driver::run` | Gap | c465, c485 | **P0** |
| QueueUpdate 入队仅 EventBus | Gap | c465, c475 | **P0** |
| abort 清 steer 留 follow_up | Ready | — | — |
| Text/Thinking/Tool 主流 | Ready | — | — |
| `display_diff` 抽取 | Partial | c465 | P1 |
| follow_up 正文 peek | Partial | c480 | P1 |
| dispatch 未接线 | Ready(未接) | c480 | P1 |
| 会话树 Driver list/branch | Gap | 真树 | P1 |
| Compaction/AutoRetry 事件 | Stale | c493 | P2 |
| ToolExecutionUpdate | Stale | 流式工具 | P2 |
| RemoteDriver | Stub | 远控 | P3 |

---

## 7. 建议修复顺序（代码向）

**P0 — 开闸 c465 前或同 PR**

1. Host 合流 `Driver::run` EventStream（c465 本体）。
2. 统一 QueueUpdate：入队/abort 也进活跃 run 流，或 Driver 合并可订阅流。
3. Bridge：`apply_xy_event`；edit JSON → Diff；未知事件 tracing。

**P1 — 与 c480 / 垂直切片并行**

4. follow_up 文案：peek API **或** 写死「UI 必须自持快照」。
5. 接线 `dispatch`：`/model`、Abort；调用方处理 Prompt/Quit；`/exit` 别名。
6. host `select!`：Tick + 输入 + **agent 事件**。

**P2+**

7. 发射或删除死类型；Compaction 并入 EventStream。
8. 真树：`Driver::list_sessions` + branch/leaf。
9. c490 trust UI；RemoteDriver；清陈旧注释。

**开闸门槛**：包侧已够；开闸 = 允许开始 c465，不是宣称可聊一轮。

---

## 8. 命名 / 结构调优探索（仅建议，未改码）

对照已归档：**c295**（`Xy*` 领域 + `protocol` 无前缀）、**c320**（`Agent` / `ReActAgent`）、**c450/c461**（steer/follow-up）。

### 8.1 不要回退（settled）

- `Agent` / `ReActAgent` / 无 facade（c320）
- `XyEvent`、ports 的 `Xy*`（c295）
- `protocol::{Command,Event}` **不加** `Xy`、JSON 稳定（c295）
- `execute_bash` / 产品 bash 词（勿改 `execute_shell`）
- steer / follow-up 动词对（c450/c461）
- `UiRoot`；包 `InputEvent`；host 驱动 API
- `packages/xylitol-tui` 零引用主 crate

### 8.2 双词汇表（须文档钉死）

| 层 | 类型 | 角色 |
|---|---|---|
| 领域 | `XyEvent` | ReAct + sink |
| 线协议 | `protocol::Event` | wire；丢弃 QueueUpdate 等 |
| Driver | `EventStream` = `Stream<XyEvent>` | 与 crossterm `EventStream` **撞名** |
| TUI 输入 | crossterm `Event` / 包 `InputEvent` | 与领域无关 |
| 命令 | `protocol::Command` | 非 UI 字符串 `/model` |
| slash | agent `SlashCommandInfo` / 包 `SlashCommand` / Driver `CommandInfo` | 三套 |

### 8.3 建议 rename（风险）

| old → new | 风险 | 动机 |
|---|---|---|
| `Driver::EventStream` → `DriverEventStream` / 直接用 `XyEventStream` | medium | 消撞名 |
| `queue_stats() -> (usize,usize)` → `QueueStats { steer_count, follow_up_count }` | medium | footer / c480 |
| `get_messages` → `get_session_entries` | medium | 名实一致 |
| `HostSession` → `TuiHost` / `HostLoop` | medium | 非持久化 session |
| `tui::run` → `run_tui` | safe–medium | 与 `Driver::run` 区分 |
| settings `SteeringMode` 与 `QueueMode` 对齐或文档 alias | medium | 双枚举 |
| dispatch 注释去掉 `tui/commands.rs` | **safe** | 历史债 |
| `Driver::run` → `submit_prompt` | breaking | 仅全面改 surface 时 |
| `protocol::Event` → `XyEvent` | **禁止** | 违 c295 |
| `Agent` → `AgentSession` | **禁止** | 违 c320 |
| `execute_bash` → `execute_shell` | **禁止** | 产品词 |

### 8.4 建议 SDD 分批（purpose-draft 意向，未创建）

| 意向 | purpose | 时机 |
|---|---|---|
| 文档 chore（可挂 c465） | 事件/命令/队列词汇表；钉 QueueUpdate 双路径；修 dispatch 死链注释 | 开闸前 / 与 c465 同批 |
| clarify-driver-queue-stats | `QueueStats` 结构体；可选 `get_session_entries` | c465 前或紧随 |
| unify-queue-mode-settings | `SteeringMode` ≡ `QueueMode` | 非闸门阻塞 |
| rename-tui-host | `HostSession`→`TuiHost`；`run_tui` | c480 前后 |
| coalesce-queue-update | 入队与 run 流单通道 | bridge 稳定后 |

**不要**为「好看」单独开大 rename 挡 c465/c480。

### 8.5 命名闸门优先级

1. **P0 文档**：`XyEvent` vs `protocol::Event` vs `HostEvent` vs crossterm；QueueUpdate 双路径；TurnEnd ≠ 用户轮结束。
2. **P0 c480**：只用 `steer`/`follow_up`/`abort`；slash → Command → dispatch。
3. **P1**：`QueueStats`；dispatch 注释；`HostSession`/`run` 命名。
4. **P2**：settings 统一；QueueUpdate 合流实现；`get_messages` rename。

---

## 9. 与轨 A / DESIGN 的关系（摘要）

- 包 + `agent_demo`：MVP 原子（布局、Editor、树、Diff、expandable、bash 边框、Ctrl+G、theme auto）**够开闸**。
- DESIGN：`DESIGN.md` + 多数 `design/*.md` 够用；bridge 生命周期无独立 design（随 c465 补）；trust/compaction 仍草稿。
- 产品 stub（c491）与 demo 活树分工已写清：未开闸勿在 stub 上扩。

---

## 10. 指针

| 资源 | 路径 |
|---|---|
| 架构 SSOT | `src/AGENTS.md` |
| 产品面冻结 | `src/app/tui/AGENTS.md` |
| 进度板 | `_HANDOFF.md` |
| c461 design | `llmanspec/changes/archive/2026-07-10-c461-expose-steer-followup-seam/`（或活跃副本） |
| c465 purpose | `llmanspec/changes/c465-add-app-tui-bridge/proposal.md` |
| 包差异台账 | `packages/xylitol-tui/PI_DELTAS.md` |
| 交互版摘要（IDE canvas，非仓内） | Cursor canvases `src-core-tui-readiness.canvas.tsx` |

---

## 修订记录

| 日期 | 说明 |
|---|---|
| 2026-07-10 | 初版：就绪度审计 + 命名探索落地 |
