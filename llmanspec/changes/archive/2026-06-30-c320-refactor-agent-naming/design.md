# Design — c320-refactor-agent-naming

> 本文档记录本次重构的诊断、目标结构、命名取舍与拆分原则。spec 层的可验证需求见各 `specs/*/spec.toon`；本文是权衡与迁移的依据。

## 1. 诊断：两个病根

### 1.1 名字与概念重量倒挂

`agent/` 层三个具名类型的"名字重量"与"概念重量"反向：

```
概念重量:   重 ──────────────────────────── 轻
            AgentSession    AgentLoop       Agent(空壳)
            (25字段)        (2字段)         (1字段)
名字重量:   轻 ──────────────────────────── 重
            "Session"       "Loop"          "Agent"
            (矮化)          (尚可)          (拔高)
```

证据：
- `facade.rs` 的 `Agent` 对 `AgentLoop` **零行为增量**：`abort()`→`self.loop_.abort()`，`session()`→`self.loop_.session()`，`set_tools()`→`self.loop_.session_mut().set_tools()`……全部纯转发。它存在的唯一理由是"名字好听 + re-export 类型"。
- `AgentBuilder` 的 15 个字段是 `AgentSession::new(...)` 15 个参数的一一投影，`build()` 跑完即弃。读者会误判"builder 才是真正的 Agent"。
- `AgentSession` 才是长期存活、真正持有 models+io+tools、还能运行时 `set_tools`/`set_permission`/`replace_hooks` 的能力体——却被叫"Session"。

### 1.2 AgentSession 是 god-object

25 字段混了 model 管理 / tools / hooks / prompt / session 持久化 / compaction / bash 执行 / export / 权限 / skills / 消息队列 / stats / trust 等十几个不内聚的职责域。其中**近一半方法是 dead code**（探索报告第 4 节）：

| dead 域 | 方法 |
|---|---|
| steering（整个域） | steer/follow_up/clear_queue/pending_message_count/has_pending_steer/get_steering_messages/get_follow_up_messages/drain_queued_messages/message_queue/message_queue_mut |
| prompt 分发 | process_prompt（CLI slash 分发没走它） |
| model 边缘 | cycle_forward/cycle_backward/registry_clone |
| permission 包装 | check_permission_read/write/network（react 自己路由，没用包装） |
| 其他 | share_as_gist/save_trust_decision/set_append_prompt/compact_current_session/compaction_threshold/settings/register_template(s)/register_command/register_skill_commands/expand_skill_command/set_skills/dispose/abort(session版) |

dead code 是 god-object 膨胀的主因，也是拆分阻力小（搬走它们几乎不碰核心 surface）的原因。

## 2. 目标结构

```
现状                                  目标
─────                                  ─────
Agent (facade.rs, 空壳)  ──删──►      (并入 ReActAgent)
  └ AgentLoop (react.rs)               ReActAgent (react.rs, = AgentLoop 改名 + 吸收空壳方法)
      └ AgentSession (session/mod.rs)      └ Agent (session/mod.rs, = AgentSession 改名 + 五块拆出后瘦身)
                                               ├ SessionExporter (session/export.rs)   ← export_io
                                               ├ BashExecHandler (session/bash.rs)     ← bash_executor
                                               ├ PermissionGate (session/permission.rs)← permission
                                               ├ stats::compute (session/stats.rs)     ← 下沉
                                               └ trust::save_trust_decision (session/trust.rs) ← 自由函数

facade.rs (re-export hub) ──删──►     re-export 迁到 agent/mod.rs
AgentBuilder              ──留──►     build() → ReActAgent（签名重新分组：Ports/Config/Slots）
```

- `ReActAgent`：ReAct 策略驱动器，驱动 `Agent` 跑循环。持有 `Agent` + cancel 控制。所有原空壳 `Agent` 的 `run`/`abort`/`set_*` 方法落在这里。
- `Agent`：能力聚合体（models + io + tools + 编排状态）。持有 collaborator 字段 + 核心 session 上下文（`store`/`session_id`/`sink`）。
- 两者关系是 **Strategy（ReActAgent 是 strategy，Agent 是 context）**：`ReActAgent` 驱动 `Agent`。

## 3. 命名取舍

### 3.1 为何 `AgentSession → Agent`（接受词义重叠代价）

把 25 字段的能力体改名为 `Agent`，会让代码里 "session" 一词同时表达两个概念：
- `Agent` 内部状态（本次赋予的新义）；
- `XySessionStore` / `session_id` / `SessionEntry` 的"持久化会话"（既有义）。

**接受这个重叠的理由**：
- 用户的核心诉求正是"真正的能力体被 Session 这个名字矮化了"。改名为 `Agent` 是直击诉求。
- 重叠可由上下文区分：`XySessionStore`/`SessionEntry` 是持久化层词汇（在 `runtime_protocol`/`domain`），`Agent` 是编排层词汇（在 `agent`），分属不同模块，读者不会混淆。
- 替代名（`AgentBody`/`AgentState`）虽避开重叠，但弱化了"这是 Agent 本体"的表达力，且不符合用户明确选择的 `Agent` 命名。

### 3.2 为何 `AgentLoop → ReActAgent`（为多风格家族提前就位）

`ReActAgent` 暗示未来会有 `PlanExecuteAgent`/`SingleShotAgent` 等其他循环风格。这是**用户的真实计划（哪怕远景）**，所以提前就位命名是对的。若仅为语义清晰而无多风格打算，则应叫 `Agent`（驱动器）+ `AgentBody`，不过度承诺家族——但本变更按用户意图采用 `ReActAgent`。

### 3.3 为何取消 facade 层

`facade.rs` 是"纯转发空壳 + re-export hub"。它造成的认知摩擦（"打开 mod 看到 facade/builder 两个并列模块，不知从哪进门"）大于它提供的封装价值。取消后类型直接定义在 `agent/` 下，从 `agent/mod.rs` re-export，读者第一眼看到的就是 `agent::ReActAgent` + `agent::Agent` 两个明确入口。`mod.rs` 顶部 `//!` 文档承担"进门引导"职责。

**代价**：`agent::runtime` / `agent::session` 内部模块仍需对 app 隐藏（否则 app 可能 reach into 内部）。由 arch_guard 的 R3（app_only_from_driver）和 `pub(crate)` 可见性控制保证，不依赖 facade 这层壳。

## 4. 拆分原则

### 4.1 共享依赖注入，不复制

`store: Arc<dyn XySessionStore>` / `sink: Arc<dyn XyEventSink>` / `session_id: Option<String>` 是"会话上下文三件套"，被 export/bash/stats/compaction/trust 共用。拆 collaborator 时让它们的方法**接收 `&dyn XySessionStore` + `&str` sid 参数**（纯函数风格，参考已有自由函数 `record_bash_result(store, ...)`），而不是各自克隆一份 store。这样 `Agent` 仍是唯一的 session 上下文持有者。

### 4.2 核心 surface 稳定性指标（硬指标）

判断拆分方案好坏的硬指标：**rpc.rs / react.rs / 原 facade 调用点零改或最小改**。为此：
- collaborator 方法签名尽量保持与原 `AgentSession` 方法一致（如 `export_to_html(&self, path)`）；
- `Agent` 上保留 delegate 方法（一行转发到 collaborator），让外部 `session().export_to_html()` 调用点不改；
- 只有"明显该换访问路径"的地方（如 permission 改走 `PermissionGate`）才改调用点。

探索报告第 4 节的"核心 surface"清单（select_model/set_session/ensure_session/build_current_model/tools/hooks/get_permission/execute_bash/export_to_*/get_session_stats/maybe_auto_compact 等）签名尽量不动。

### 4.3 反例：steering.rs "分文件不分对象"

`steering.rs` 用 `impl super::AgentSession` 把方法留在 `AgentSession` 上，只分了文件——这是反面教材（只分文件没分对象，god-object 没真正瘦身）。本次对 steering 域的处理是**直接删 dead**（整个域零生产调用方），不升级成独立 struct（YAGNI，等真要接 steering 再建）。对 export/bash/permission 则**真正分对象**（独立 struct + Agent 持有）。

## 5. 命名映射表（collaborator）

| collaborator | 落点 | 持有字段 | 命名来源 |
|---|---|---|---|
| `SessionExporter` | `agent/session/export.rs` | `export_io` | 沿用 as32 已定名 |
| `BashExecHandler` | `agent/session/bash.rs` | `bash_executor`/`bash_cancel` | 沿用 as32 已定名（非探索时临用的 BashRunner） |
| `PermissionGate` | `agent/session/permission.rs` | `permission` | 本次新定（表达"咨询性闸门"，与 trait `XyPermission` 不重名） |
| `stats::compute` | `agent/session/stats.rs`（已有类型层） | （无新字段，纯函数） | 下沉，非 struct |
| `trust::save_trust_decision` | `agent/session/trust.rs` | （无新字段，自由函数） | 降级，非 struct |

## 6. 实施顺序与风险

P0（改名 AgentSession→Agent）→ P1（改名 AgentLoop→ReActAgent + 删空壳）→ P2（取消 facade）→ P3（五块拆）→ P4（清 dead）→ P5（文档+QA）。每阶段全测试套件绿灯才进下一阶段。

**最高风险点**：
- P0 的 API 快照重生成（`cargo insta review`）——若漏改调用点，快照不匹配。
- P2 的 import 全局替换——`crate::agent::facade::` 在多处，易漏（尤其全限定路径 `crate::agent::facade::Agent` 在 react.rs test helper）。
- P3 的 `&mut self` 借用——`BashExecHandler::execute_bash` 需 `&mut`（改 `bash_cancel`），`Agent` 持它时调用点需 `&mut self`，确认 rpc.rs:392 的 `execute_bash` 调用链可变借用不冲突。

**零风险点**：HC-1..HC-6 不变量、磁盘数据、运行期行为（全 dead code 或纯重组）。

## 7. 不做的事（范围外）

- **不拆 model/prompt/compaction 域**：它们已是 collaborator（`ModelManager`/`SkillManager`/`CompactionOrchestrator`），内聚度高或耦合深（model 变更要持久化是跨域事件），本次不动。
- **不引入新 trait**：collaborator 全是普通 struct（HC-5）。
- **不预留多 agent 风格的 trait 抽象**：`ReActAgent` 是具体类型，不为"未来 PlanExecuteAgent"提前抽 trait（YAGNI，等第二种风格真出现再抽）。
- **不改 `AgentBuilder` 的字段分组为 Ports/Config/Slots 结构体**：本次 builder 只改 build 目标（→ReActAgent）和同步改名，字段分组重组留作后续优化（避免本次改动面过大）。
