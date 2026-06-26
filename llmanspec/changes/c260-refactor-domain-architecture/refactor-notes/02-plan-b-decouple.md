# 方案 B — agent 瘦身 + facade/Driver 雏形

> 野心：拆 god-file / agent 瘦身为"薄编排" / 为交互层立 Driver 雏形
> 风险：低-中（god-object 拆分需小心 borrow/生命周期，但每步可独立提交）
> 前置：方案 A 已完成
> 对齐总览 [00-overview.md](00-overview.md)：服从 HC-1（层方向）、HC-2（agent 可独立使用）

## 1. 目标

在方案 A 的干净边界之上，做两件事：

1. **让 `agent` 变薄**——只保留 ReAct 循环 + hook 切入面 + 编排状态（当前 model/tools 选择）。其余职责（prompt 输入成型、bash 执行、stdout 守卫等"运行时设施"）下沉或迁出。这直接兑现总览"agent 必须薄"。
2. **给交互层立一个雏形**——`interface`（方案 A 后已名 `interactive`）不再 reach into agent 内部模块，转而通过一个窄入口交互。这个窄入口是方案 D `Driver` 抽象的前身。

方案 B 不引入新 trait port（那是 C 的事），不迁 provider 层（也是 C 的事），不做 server（D 的事）。它的价值在于：agent 回归"薄编排"本位、交互层与 agent 解耦、`AgentSession` 从 1500 行 god-object 瘦身。

## 2. 现状问题（方案 B 要解决的）

### 2.1 interactive 深入 agent 内部

当前 `interactive/rpc.rs`：
```rust
use crate::agent::compaction::CompactionSettings;
use crate::agent::r#loop::{AgentEvent, AgentLoop};   // 方案 A 后变 runtime
use crate::agent::session::{AgentSession, ModelRegistry};
use crate::agent::tools::ToolRegistry;
```

RPC/print 同时依赖 agent 的 4 个内部子模块。这意味着任何 agent 内部重组都会波及 interactive。这违反总览 HC-1 的精神（interactive 应只认 protocol/Driver）。

### 2.2 `AgentSession` 是 god-object

`session/mod.rs` 单文件 1500+ 行，组合了 `ModelManager`/`CompactionOrchestrator`/`SkillManager`/`MessageQueue`，并直接耦合 5 个 infra 子模块（session/event/resource/trust/source_info/sandbox）。

### 2.3 `loop.rs` 是 god-file

683 行，混合了：ReAct 算法、`AgentHooks` 扩展点、`AgentEvent`/`AgentEventStream` 类型、sandbox 路由辅助函数。

## 3. 调整后的目标形态

```
agent/
├── provider/      adapter 层（impl XyModel）—— 方案 B 不动归属
├── runtime/       ReAct 循环 + hooks + queue + retry + stdout_guard + bash_executor
│   ├── mod.rs     re-export
│   ├── react.rs   纯 ReAct 算法
│   ├── hooks.rs   AgentHooks 扩展点（从 react.rs 提出）
│   ├── event.rs   AgentEvent / AgentEventStream（从 react.rs 提出）
│   ├── queue.rs
│   ├── retry.rs
│   ├── stdout_guard.rs
│   └── bash.rs    原 bash_executor.rs
├── session/       会话状态 + 持久化 facade（瘦身后的 AgentSession）
│   ├── mod.rs     AgentSession（仅状态字段 + 委托方法）
│   ├── io.rs      SessionIO（持久化访问，已有）
│   ├── events.rs  事件订阅（已有）
│   ├── export.rs  导出（已有）
│   ├── stats.rs   统计（已有）
│   ├── steering.rs
│   ├── prompt_result.rs
│   └── bash_exec.rs
├── tools/         工具注册表 + 内置工具 + ToolDefinition
├── prompt/        system prompt + templates + commands + skills（合成"输入侧"）
│   ├── mod.rs
│   ├── system.rs     原 prompt.rs
│   ├── commands.rs   原 commands.rs
│   ├── templates.rs  原 templates.rs
│   └── skills.rs     原 skill_manager.rs（方案 A 后的 skills.rs）
└── facade.rs      ← 新增：agent 对外唯一入口
```

## 4. 迁移映射表（在方案 A 之上）

| 操作 | 源 | 目标 | 标签 | 说明 |
|---|---|---|---|---|
| 搬 | `agent/bash_executor.rs` | `agent/runtime/bash.rs` | `move` | 属运行时设施 |
| 拆 | `agent/runtime/react.rs`（方案 A 后的 loop.rs，683行） | `react.rs` + `hooks.rs` + `event.rs` | `shrink` | god-file 拆解 |
| 合并 | `agent/commands.rs`+`templates.rs`+`prompt.rs`+`skills.rs` | `agent/prompt/{commands,templates,system,skills}.rs` | `consolidate` | 合成"输入/prompt 构造"子层 |
| 新增 | — | `agent/facade.rs` | `consolidate` | 收敛 `AgentLoop`+`AgentSession` 公开 API |
| 收口 | `interactive/{rpc,print,cli}` | 改依赖 `agent::facade` | `consolidate` | 删掉对 agent 内部子模块的直接 import |
| 瘦身 | `agent/session/mod.rs`（1500行） | 状态字段 + 委托 | `shrink` | god-object 拆分 |

## 5. 逐项执行细则

### 5.1 拆分 god-file `runtime/react.rs`

当前 `react.rs`（方案 A 后的 loop.rs）混合三类内容。拆分目标：

```
runtime/
├── react.rs    ReAct 主循环算法（run_react_loop / AgentLoop::run）
├── hooks.rs    AgentHooks struct + Default impl
└── event.rs    AgentEvent enum + AgentEventStream struct + Stream impl
```

**步骤：**

1. 把 `AgentEvent`/`AgentEventStream` 及其 `Stream` impl 整体剪到 `event.rs`。
2. 把 `AgentHooks`/`SteeringMode`/`FollowUpMode`/`Default for AgentHooks` 剪到 `hooks.rs`。
3. `react.rs` 只留 `AgentLoop` + `ReActConfig` + `run_react_loop` + sandbox 辅助函数。
4. `runtime/mod.rs` re-export：`pub use event::*; pub use hooks::*; pub use react::AgentLoop;`
5. 确认所有 `crate::agent::runtime::{AgentEvent, AgentLoop, AgentHooks}` 仍可用（通过 re-export）。

> NOTE: 拆分后 react.rs 应 ≤ 350 行。若仍超，说明 ReAct 算法本身需要进一步函数提取（如把 sandbox 路由独立成 `runtime/sandbox_router.rs`）。

### 5.2 `bash_executor.rs` → `runtime/bash.rs`

```bash
git mv src/agent/bash_executor.rs src/agent/runtime/bash.rs
# runtime/mod.rs 加 pub mod bash;
# 全局替换 crate::agent::bash_executor → crate::agent::runtime::bash
```

> NOTE: `bash_executor.rs` 自述"Shares primitives with the tool version (tools/process, tools/accumulator, tools/truncate)"——它和 `tools/bash.rs` 是两个入口（用户 `!cmd` vs LLM tool-call），但共享底层。搬进 runtime 表明它是"会话运行时设施"而非"工具"。

### 5.3 合成 `agent/prompt/` 子层

当前 agent 顶层有 4 个散文件属于"输入/prompt 构造"职责族：

| 文件 | 行数 | 职责 |
|---|---|---|
| `commands.rs` | 227 | slash 命令表 + 路由 |
| `templates.rs` | 324 | `/template:name` 展开 |
| `prompt.rs` | 318 | system prompt 组装 |
| `skills.rs`（方案 A 后） | 174 | skill 激活 + XML 展开 |

合并成 `agent/prompt/`：

```bash
mkdir src/agent/prompt
git mv src/agent/prompt.rs src/agent/prompt/system.rs
git mv src/agent/commands.rs src/agent/prompt/commands.rs
git mv src/agent/templates.rs src/agent/prompt/templates.rs
git mv src/agent/skills.rs src/agent/prompt/skills.rs
# 建 prompt/mod.rs，re-export
# agent/mod.rs 删 4 个 pub mod，加 pub mod prompt;
```

> NOTE: 合并后 agent 顶层仅剩 `facade.rs`（新增）+ 子目录。顶层"散文件归零"是 B 的关键视觉信号。

### 5.4 新增 `agent/facade.rs` —— 交互层唯一入口（Driver 雏形）

这是方案 B 的核心动作。定义 agent 对外的**唯一入口**，作为方案 D `Driver` 抽象的前身：

```rust
//! Agent facade — the single public entry point for interactive layers.
//!
//! Interactive code (cli/print/rpc) should ONLY import from `agent::facade`.
//! Reaching into `agent::runtime`/`agent::session`/`agent::tools` directly
//! is a layering violation. This facade is the in-process half of the
//! Driver abstraction (see plan D); RemoteDriver in D mirrors it over WS.

pub use crate::agent::runtime::{AgentEvent, AgentEventStream, AgentHooks};
pub use crate::agent::session::AgentSession;
pub use crate::agent::tools::ToolRegistry;

use crate::agent::runtime::AgentLoop;

/// The agent — owns session + runtime loop.
///
/// Constructed at the composition root (interactive::cli) with concrete
/// adapters; interactive layers only see this type and `AgentEvent`.
pub struct Agent {
    loop_: AgentLoop,
}

impl Agent {
    pub fn new(session: AgentSession) -> Self {
        Self {
            loop_: AgentLoop::new(session),
        }
    }

    pub fn with_hooks(mut self, hooks: AgentHooks) -> Self {
        self.loop_ = self.loop_.with_hooks(hooks);
        self
    }

    /// Run a turn. Returns a stream of events for the interface to render.
    pub async fn run(&mut self, prompt: &str, session_id: &str) -> AgentEventStream {
        self.loop_.run(prompt, session_id).await
    }

    /// Access the underlying session for command dispatch
    /// (model switch, compact, export, etc.).
    pub fn session(&self) -> &AgentSession {
        self.loop_.session()
    }

    pub fn session_mut(&mut self) -> &mut AgentSession {
        self.loop_.session_mut()
    }

    pub fn abort(&self) {
        self.loop_.abort();
    }
}
```

> NOTE: facade 当前是薄包装。它的价值不在"加抽象"，而在**确立边界契约**——interactive 只认 `Agent` + `AgentEvent`，agent 内部怎么重组都不波及 interactive。这是开闭原则在"对外稳定性"上的体现。
>
> NOTE（HC-2 偏差，待 C 修正）: 当前签名 `Agent::new(session)` 与 `run(prompt, session_id)` 仍把 Session/session_id 耦合进编排入口，违反总览 HC-2（agent 不得强制 Session、不得持有 session_id）。方案 B 阶段先保留以控制改动面；C 引入 `SessionStore` port 后，facade 改为 `Agent::new(ports...)` + `run(prompt)`，session_id 下沉为 store 的查询键，不进编排状态。这是"先解耦表象、再解耦实质"的两步走。

### 5.5 收口 interface 依赖

**改造前（rpc.rs）：**
```rust
use crate::agent::compaction::CompactionSettings;
use crate::agent::runtime::{AgentEvent, AgentLoop};
use crate::agent::session::{AgentSession, ModelRegistry};
use crate::agent::tools::ToolRegistry;
```

**改造后：**
```rust
use crate::agent::facade::{Agent, AgentEvent, AgentHooks};
// 仅在构造期需要具体类型时，才从 agent 对应子层 import（组合根特权）
```

interface 各文件：
- `print.rs`：只需 `AgentEvent`，改为 `use crate::agent::facade::AgentEvent;`
- `rpc.rs`：用 `Agent` 替代 `AgentLoop`，删除对 `session`/`tools` 的直接 import（构造期除外）
- `cli/mod.rs`：作为组合根，保留对 `AgentSession`/`ToolRegistry`/`ModelRegistry` 的构造期访问，但运行期通过 `Agent` 交互

### 5.6 `AgentSession` 瘦身

`session/mod.rs` 当前 1500+ 行，已是 `io.rs`/`events.rs`/`export.rs`/`stats.rs` 子文件的"伞文件"。瘦身原则：**mod.rs 只留状态字段 + 委托方法，业务逻辑下沉到子文件。**

可下沉的内容：
- sandbox 相关方法（`check_sandbox_read/write/network`、`get_sandbox_engine`）→ `session/sandbox.rs`
- bash 执行（`execute_bash`/`record_bash_result`/`abort_bash`）已有 `bash_exec.rs`，确认委托干净
- trust/fork/navigate/switch 等会话管理 → 视行数决定是否拆 `session/lifecycle.rs`

> NOTE: 目标是 `session/mod.rs` ≤ 600 行。拆分依据是"职责族"而非"行数均分"——sandbox 是一类、持久化是一类、生命周期是一类。

## 6. 验证清单

- [ ] `grep -rn "crate::agent::runtime" src/interactive/` 仅出现在构造期 / facade re-export
- [ ] `grep -rn "crate::agent::session::" src/interactive/` 仅出现在 cli 组合根
- [ ] `wc -l src/agent/runtime/react.rs` ≤ 350
- [ ] `wc -l src/agent/session/mod.rs` ≤ 600
- [ ] `agent/` 顶层散文件 = 0（全是子目录 + `facade.rs`）
- [ ] `cargo build --all-features` 通过
- [ ] `cargo nextest run --profile ci` 全绿
- [ ] `cargo clippy --all-features -- -D warnings` 通过
- [ ] BDD 场景全绿（`cargo test --test bdd -- --test-threads=1`）

## 7. 风险与回滚

- **风险点 1**：`react.rs` 拆分涉及 `AgentEventStream` 的 `Stream` impl 跨文件引用，注意 `Pin<Box<dyn Stream>>` 的可见性。
- **风险点 2**：`AgentSession` 瘦身涉及大量 `&self`/`&mut self` 方法迁移，borrow checker 可能报错。**每下沉一个职责族就单独编译验证**。
- **风险点 3**：facade 引入后，RPC 协议若暴露了 `AgentSession` 的内部方法名，需确认 RPC schema 不变（行为兼容）。
- **风险点 4（HC-2）**：facade 暂保留 Session/session_id 耦合（见 5.4 NOTE），需在 C 修正。**不要在 B 阶段中途引入半成品 port**，以免 C 还要返工。
- **回滚**：建议拆成 3~4 个 PR（5.1+5.2 / 5.3 / 5.4+5.5 / 5.6），每个可独立 revert。

## 8. 净效果

- interactive 不再 reach into agent 内部
- agent 回归"薄编排"：只留循环 + hook + 编排状态，其余下沉/迁出
- `AgentSession` 从 god-object 瘦身
- agent 顶层散文件归零
- 立起 facade 边界契约，为方案 D 的 `Driver` 抽象（InProcess/Remote）直接铺路

## 9. 推荐路线

建议拆 PR：

| PR | 内容 | 对应章节 |
|---|---|---|
| B-1 | 拆 `react.rs` → react/hooks/event；`bash_executor`→runtime/bash | 5.1 + 5.2 |
| B-2 | 合成 `agent/prompt/` 子层 | 5.3 |
| B-3 | 新增 `facade.rs` + 收口 interactive 依赖 | 5.4 + 5.5 |
| B-4 | `AgentSession` 瘦身 | 5.6 |

B-3 是关键 PR，建议配 BDD/单元测试护栏后再合。B 完成后可直接进入 D-1（抽 protocol + Driver），不必等 C。
