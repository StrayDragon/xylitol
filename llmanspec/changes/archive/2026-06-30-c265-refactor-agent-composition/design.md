# c265-refactor-agent-composition — Design

> 记录核心定义、被否决的设计、关键权衡与迁移路径，自洽可读。逐项 git mv/编辑命令见 `tasks.md`。

## 0. 立场

xylitol 的 agent 是**薄编排**：跑 ReAct 循环、装配运行时、发事件。**它不该认识"探索模式""只读模式"这类业务分类**——那是表皮（app/tui）的语义。agent 只提供"把哪些运行时挂上来"的组装能力，组合结果由调用方决定。

由此推出本次的一条硬规则：**agent 层不出现任何 mode/preset enum，只出现最小单元操作。**

## 1. 三条正交组合维度（均已存在，本轮只理顺）

```
   模型看到的工具集     ToolSet          (构造期数据组合)   ← 决定"模型知道哪些工具"
   工具调用的内容边界   XyPermission     (咨询性 port)      ← 决定"调用被允许吗"
   执行前后的切面       AgentHooks       (in-process 闭包)  ← 决定"调用前后插什么逻辑"
```

三者正交，各管一段，可任意组合：

| 想要的 agent       | ToolSet                                  | XyPermission        | Hooks       |
|--------------------|------------------------------------------|---------------------|-------------|
| 纯对话（无工具）   | `empty`                                  | `AllowAll`          | 无          |
| 探索 agent         | `from_iter(defaults).retain(只读判断)`   | `GlobPolicy`(兜底)  | 可选：日志  |
| 完整 coding agent  | `from_iter(defaults)`                    | 策略 / AllowAll     | 可选        |
| 审计 agent         | `from_iter(defaults)`                    | AllowAll            | before/after 记录 |

**关键点**：这张表是二维以上的，任何一维 enum 都压缩不了它而不丢表达力。所以组合维度必须独立。

## 2. 被否决的设计（为什么没有 `AgentMode` / `Capability`）

| 方案 | 否决理由 |
|------|----------|
| `enum AgentMode { ReadOnly, ReadWrite, Custom(...) }` | 预设思维。"只读工具 + 放开 sandbox"这类交叉组合无法表达，enum 必然膨胀。 |
| `enum Capability { Read, Write, ... }` + `ToolSet::capable_of([Capability])` | 仍然是 set 预设。工具自分类一旦引入就被钉死颗粒度（要 `Delete`?`Move`?）。xylitol 尚未到需要它作为 harness 的成熟度。 |
| 把 RO 做成 agent 层语义 | 概念归属错。RO 是 sandbox 实现的一种策略，不是 agent 语义。降到 infra 提供的 `XyPermission` 一个实现即可。 |
| 把 `XySandboxEngine` 改造成"外部 sandbox 适配器" | 陷阱。只要还是 `check_*(target)->Verdict` 形状，在我们这层就仍是咨询性的（能被忽略）。外包咨询逻辑不会让我们这层变强制，只会强化"名字像安全边界"的误解。 |

**本轮只给**：`ToolSet::retain(pred)` 等单元操作——判断逻辑交调用方闭包写。探索 agent 的"只读工具"=调用方自己列出只读工具名传给 `retain`，agent 层毫不知情。

> Capability 分类标 `// NOTE:` 留作未来：当 xylitol 自身要作为 harness、要消除 `react.rs` 里硬编码的 `match tool_name → sandbox check` 派发时，再引入工具自声明能力。**本轮不做。**

## 3. AgentBuilder（取代 14 参 with_ports）

```rust
// src/agent/builder.rs  —— agent 层，只吃 Arc<dyn Port> + 纯数据（HC-1 干净）
pub struct AgentBuilder {
    // 必填：跑一轮对话的最小内核（HC-2）
    model_registry: ModelRegistry,
    model_builder: XyModelBuilder,
    store: Arc<dyn XySessionStore>,
    sink: Arc<dyn XyEventSink>,
    permission: Arc<dyn XyPermission>,
    // 可选槽 —— Default 见 impl
    tools: ToolSet,            // 默认 empty
    system_prompt: Option<String>,
    context_files: Vec<(String,String)>,
    append_system_prompt: Vec<String>,
    max_iterations: u32,       // 默认 50
    compaction: CompactionTuning,
    cwd: String,
    hooks: AgentHooks,         // 默认空链
    bash_executor: Option<Arc<dyn XyBashExecutor>>,   // None → !命令不可用
    export_io: Option<Arc<dyn XyExportIo>>,           // None → /export 不可用
}
impl AgentBuilder {
    pub fn new(model_registry, model_builder, store, sink, permission) -> Self { /* defaults */ }
    pub fn tools(mut self, t: ToolSet) -> Self { self.tools = t; self }
    pub fn system_prompt(mut self, s: impl Into<String>) -> Self { ...; self }
    pub fn hooks(mut self, h: AgentHooks) -> Self { ...; self }
    pub fn bash(mut self, b: Arc<dyn XyBashExecutor>) -> Self { ...; self }
    pub fn export_io(mut self, e: Arc<dyn XyExportIo>) -> Self { ...; self }
    pub fn sandbox(mut self, p: Arc<dyn XyPermission>) -> Self { ...; self }  // 覆盖默认
    pub fn build(self) -> Result<Agent, String> { ... }
}
```

**分层归属**：builder 在 `agent/`，只吃 port trait 对象与纯数据，**不 import 任何 `infra` 具体类型**（HC-1）。`composition.rs` 退化为把 infra 具体实现喂给 builder。

**为何 builder 在 agent 层而非 app 层**：放在 app 层会让每个 surface（cli/rpc/server）都得自己重组 14 参——现在 `rpc.rs` 的 `build_agent_fresh` 已经在重复这件事。builder 下沉到 agent 层一次定义，所有 surface 共用。

## 4. 运行时可改：turn-snapshot（决定 A1，否决 A2）

**诉求**：用户在 app/tui 切换 plan/edit 模式，下一轮对话生效。

**事实**（grep 核实 `react.rs:128`）：`run()` 在每个 turn 开头把 `tools.clone()`/`cancel`/`prompt` **clone 进 stream**，stream 独立持有。

**结论**：turn N 跑起来用 N 开始时的快照；N 跑时调 `agent.set_tools(...)`；turn N+1 的 `run()` 重新 clone，自动拿新配置。**这就是"下一轮生效"，无锁、无 `Arc<RwLock>`、无竞争。**

```rust
impl Agent {
    // turn 间调用 —— 改 session 内槽，下个 turn 生效
    pub fn set_tools(&mut self, tools: ToolSet) { ... }
    pub fn replace_hooks(&mut self, hooks: AgentHooks) { ... }
    pub fn add_hook(&mut self, hook: ...) { ... }      // 叠加不覆盖
    pub fn set_sandbox(&mut self, p: Arc<dyn XyPermission>) { ... }
    pub fn set_system_prompt(&mut self, p: Option<String>) { ... }
}
```

**否决 A2（执行中热插拔）**：只有"turn 跑到第 3 轮 ReAct 迭代时中途换工具并立即生效"才需要把快照改成 `Arc<RwLock<共享态>>`。诉求是"下一轮"，不是"执行中"。YAGNI，且 turn 粒度已足够灵活。

**职责边界**：agent 不自决升级/降级，只提供 setter + 快照语义。驱动方是 app/tui（用户按键 → 调 setter）。

## 5. 接上断线的 hooks

**事实**（grep 核实）：`AgentLoop.hooks` 字段从 `with_hooks` 写入，但 `self.hooks` **从未被读取**，`ReActConfig` 无 hooks 字段，5 个回调全仓零调用点。完全断线。

**改动**：
1. `before_tool_call`/`after_tool_call`：`Option<单回调>` → `Vec<回调>`（可运行时 `add_hook` 叠加）。
2. `transform_context` 同理改 `Vec`。
3. **steering/follow-up 拆出**：它们是消息注入（队列回调），不是工具执行拦截，语义不同，独立成结构。
4. `react.rs::run_react_loop` 在工具调用**前**跑 `before` 链（任一返回 Deny 即短路，写 tool-error 结果），调用**后**跑 `after` 链（可观察/改结果）。空链零开销（`Vec::is_empty` 短路）。

**与 `hook-system`（脚本式 HookDispatcher）的关系**：两者是**不同机制**，不合并：
- `HookDispatcher`（`infra/hooks/`，~970 行，活）：外部脚本集成，stdin JSON / stdout action / 超时 kill，用户配置驱动，当前接在 `tui/diff_review`。匹配 `hook-system` spec，**本轮不碰**。
- `AgentHooks`（`agent/runtime/hooks.rs`，死→活）：进程内 Rust 闭包，给 app/tui 层程序化注入（日志、plan 模式切换）而无需写脚本。

未来若两者需统一，是独立 change（见 `future.md`）。本轮只接上 AgentHooks，零改动 HookDispatcher。

**关于 `security-policy` r1 的 `SecurityToolWrapper`**：grep 确认**未实现**（零匹配）。它是第三种"工具前拦截"的设想，本轮不引入、不实现，记入 `future.md` 作为候选清理（spec 描述了未建的东西）。

## 6. XyPermission：诚实改名（不是改成适配器）

**事实**（读 `infra/permission/mod.rs`，原 `infra/sandbox`）：`GlobPolicy.check_write` 做的是 `policy::path_matches_any(path, &write_denied)` 纯 glob 字符串匹配；`react.rs` 在工具调用前 `check_*`，`Deny` 就不调工具、给 LLM 返回错误。**进程内、咨询性、靠循环自觉。** bash 工具能跑 `rm -rf`，而循环对 bash 只调 `check_network(command_as_path)`——拿命令字符串当 path 去匹配网络域名，无意义。这不是隔离。

**改名**：
- trait `XySandboxEngine → XyPermission`
- verdict `XySandboxVerdict → XyPermissionVerdict`
- `NoopEngine → AllowAllPermission`、`FallbackBackend → GlobPolicy`

**文档照搬 pi 措辞**：
```rust
//! XyPermission — advisory in-process permission gate.
//! NOT a security boundary. It politely prevents the ReAct loop from calling a
//! tool the policy denies; it does NOT prevent host-level access. Real
//! isolation requires OS/container/VM boundaries. Use this for behavioral
//! guardrails (plan mode, read-only exploration, UX deny-lists), never as a
//! security control.
```

**为什么不"改成外部 sandbox 适配器"**：见 §2 否决表。即便把 glob 逻辑换成"问外部 OpenShell gateway"，只要还是 `check_*(path)->Verdict` 形状，我们这层仍能忽略返回值——咨询性质不变，只是"更权威的咨询"。名字叫 sandbox + 对接外部，反而强化 pi 警告的误解。诚实命名 + 文档，比包装更安全。

**与 RO/plan 模式不冲突，反而澄清**：RO 探索 agent = `ToolSet`(不装 write 工具) + `XyPermission`(GlobPolicy 兜底)，两者都是行为层约束。诚实承认"非安全边界"是加法：告诉用户这是 UX 语义，对抗恶意 prompt 得靠容器。

## 7. 迁移

- **零数据迁移**：被删的 `active_tools`/`allowed`/`excluded` 是进程内临时态，不在 session JSONL 持久化（`build_session_context` schema 无这些字段）。`ToolSet`/`Option<bash>`/`AgentBuilder` 都是构造期参数。fork/export/session entry schema 不变。
- **硬切**（不留 shim，遵循 AGENTS.md）：删 `Agent::new`/`with_ports`/`with_hooks`/`with_tool_mode`、删过滤 API、改三处调用点到 builder、接 hooks、改 react 派发——一个 change 一次到位。
- **BDD 是护栏**：P0~P3 行为不变（删的是死代码/被绕过路径），任一 BDD 失败 = 行为被破坏，立即停下排查。

## 8. HC 影响

| 约束 | 影响 |
|------|------|
| HC-1 层方向 | builder 在 agent/ 只吃 `Arc<dyn Port>`，不破；arch_guard 确认无新 agent→infra 生产依赖 |
| HC-2 facade 无 session 依赖 | 沿用（builder 不强制 session_id；session_id 仍由 surface 绑定） |
| HC-4 开闭 | 新工具无需改 loop 的工具执行部分；唯一例外是 `react.rs` 硬编码的 `match tool_name→sandbox check` 派发，已标 `// NOTE:` 待 harness 成熟时引入工具自声明能力分类解决 |
| HC-5 trait 需 ≥2 实现 | **不引入新 trait**。builder/ToolSet 是普通结构；`XyPermission` 是改名（已有 AllowAll + GlobPolicy 两实现） |

## 9. 词汇表增量（防概念漂移）

| 术语 | 定义 | 不可混淆为 |
|------|------|-----------|
| **ToolSet** | agent 持有的工具集合，构造期定型，单元操作组合 | ≠ ToolRegistry（旧名，含死的运行期过滤）；≠ mode preset |
| **XyPermission** | 咨询性进程内权限 port，循环工具调用前查询 | ≠ 安全边界；≠ OS sandbox；≠ 外部 sandbox 适配器 |
| **AgentHooks** | 进程内 Rust 闭包切面（before/after 工具调用） | ≠ HookDispatcher（脚本式，外部脚本集成）；≠ SecurityToolWrapper（未实现的设想） |
| **turn-snapshot** | run() 在 turn 开头 clone 配置进 stream，turn 间改槽下轮生效 | ≠ 执行中热插拔 |
