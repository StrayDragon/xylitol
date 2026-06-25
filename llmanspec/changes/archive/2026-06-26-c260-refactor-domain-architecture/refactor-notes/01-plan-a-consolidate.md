# 方案 A — 收敛归位

> 野心：删除 / 搬家 / 改名
> 风险：极低（纯 `use` 路径重写 + 文件移动，无逻辑变更）
> 前置：无

## 1. 目标

**不动分层，只做删除/搬家/改名。** 把 `agent/` 这个杂物袋清干净，并为 `interface/` → `interactive/` 改名，为方案 B/C/D 铺路。

方案 A 不引入新概念、不拆 god-object、不新增 trait。它的全部价值在于：消灭名字遮蔽、消灭 `r#loop` 转义、消灭 orphan 文件、把放错层的文件搬回正确归属、并把交互层改名对齐总览词汇表。**是最低风险、最高"立竿见影"的一步。**

> 对齐总览 [00-overview.md](00-overview.md)：本方案服从 HC-1（层方向）、HC-6（NOTE 标注）。

## 2. 迁移映射表（完整）

| 操作 | 源 | 目标 | 标签 | 理由 |
|---|---|---|---|---|
| 删除 | `agent/types.rs` | — | `delete` | 8 行死代码 re-export，全 crate 0 引用；文件头注释自己写"prefer core" |
| 改名+搬 | `agent/traits.rs` | `agent/tools/definition.rs` | `inline`/`move` | 名字遮蔽 `core::traits`；且只有 `ToolDefinition` 一个展示类型，属于 tools 子层 |
| 搬 | `agent/model_manager.rs` | `agent/model/manager.rs` | `move` | 回归兄弟目录，消除 orphan |
| 搬 | `agent/compaction_orchestrator.rs` | `agent/compaction/orchestrator.rs` | `move` | 同上，orchestrator 与 compaction 算法同源 |
| 改名+搬 | `agent/skill_manager.rs` | `agent/skills.rs` | `move` | runtime 侧单文件即可（与 `infra/skills/` 加载侧区分） |
| 搬 | `agent/config_value.rs` | `infra/config/value.rs` | `move` | 624 行，0 个 `crate::` 依赖；属运行时域（infra），非编排（agent） |
| 改名+搬 | `agent/loop.rs` | `agent/runtime/react.rs` | `shrink`/`move` | 杀掉全部 `r#loop`，概念归位"编排循环" |
| 搬 | `agent/queue.rs` | `agent/runtime/queue.rs` | `move` | 与 ReAct loop 同源 |
| 搬 | `agent/retry.rs` | `agent/runtime/retry.rs` | `move` | 与 ReAct loop 同源 |
| 搬 | `agent/output_guard.rs` | `agent/runtime/stdout_guard.rs` | `move` | 编排循环的运行时设施 |
| **改名** | `interface/` 目录 | `interactive/` 目录 | `consolidate` | "interface"与编程语义冲突；新名表达"交互形态"（总览词汇表） |

## 3. 调整后的目标形态（仅 agent 部分）

```
agent/
├── auth/                  # 不变
├── compaction/            # + orchestrator.rs（吸收 orphan）
├── model/                 # + manager.rs（吸收 orphan）
├── provider/              # 不变（方案 A 不动 provider 归属）
├── session/               # 不变
├── tools/                 # + definition.rs（原 traits.rs 的 ToolDefinition）
├── runtime/               # 新：react.rs + queue.rs + retry.rs + stdout_guard.rs
├── commands.rs            # slash 命令表
├── prompt.rs              # system prompt 组装
├── templates.rs           # /template:name 展开
└── skills.rs              # 原 skill_manager.rs（runtime 侧）

# 删除：types.rs（死代码）
# 迁出：config_value.rs → infra/config/value.rs
# agent 顶层从 13 个散文件 → 4 个
```

## 4. 逐项执行细则

### 4.1 删除 `agent/types.rs`

```
grep -rn "agent::types" --include='*.rs' src/   # 当前 = 0
```

零引用，直接 `git rm src/agent/types.rs` 并从 `agent/mod.rs` 删 `pub mod types;`。

> NOTE: 文件头注释已写明"Prefer importing directly from `crate::core::*` in new code"——这是死代码的典型自证。

### 4.2 `agent/traits.rs` → `agent/tools/definition.rs`

`agent/traits.rs` 只有 73 行，唯一内容是 `ToolDefinition`（一个展示包装器 struct + derive），与 core 的 trait 抽象毫无关系，文件名是历史包袱。

步骤：
1. `git mv src/agent/traits.rs src/agent/tools/definition.rs`
2. `agent/mod.rs` 删 `pub mod traits;`
3. `tools/mod.rs` 加 `pub mod definition;`（或并入既有声明）
4. 全局替换 `crate::agent::traits::ToolDefinition` → `crate::agent::tools::definition::ToolDefinition`
5. 更新 `session/mod.rs` 里 `use crate::agent::traits::ToolDefinition` 等引用

> NOTE: 若替换点极少（当前 grep = 0），可考虑直接把 struct 内联进 `tools/mod.rs`，省一个文件。

### 4.3 吸收 orphan manager

**`model_manager.rs` → `model/manager.rs`：**

消费者仅 `session/mod.rs`（`use crate::agent::model_manager::ModelManager`）。

```bash
git mv src/agent/model_manager.rs src/agent/model/manager.rs
# agent/mod.rs: 删 pub mod model_manager;
# model/mod.rs: 加 pub mod manager; pub use manager::ModelManager;
# session/mod.rs: use crate::agent::model::manager::ModelManager;
```

**`compaction_orchestrator.rs` → `compaction/orchestrator.rs`：**

消费者 `session/mod.rs` + `session/stats.rs`（用 `should_compact`）。

```bash
git mv src/agent/compaction_orchestrator.rs src/agent/compaction/orchestrator.rs
# agent/mod.rs: 删 pub mod compaction_orchestrator;
# compaction/mod.rs: 加 pub mod orchestrator; pub use orchestrator::CompactionOrchestrator;
# session/mod.rs + stats.rs: 更新 use 路径
```

**`skill_manager.rs` → `skills.rs`：**

```bash
git mv src/agent/skill_manager.rs src/agent/skills.rs
# agent/mod.rs: pub mod model_manager; → pub mod skills;
# 全局替换 crate::agent::skill_manager → crate::agent::skills
```

### 4.4 `config_value.rs` → `infra/config/value.rs`

这是放错层最严重的一个。`config_value.rs`（624 行）解析 `$ENV` / `${VAR:-default}` / `!cmd`，**没有任何 `crate::` 依赖**（已用 `grep "use crate::" agent/config_value.rs` 确认 = 0），是纯配置/密钥解析工具。

步骤：
1. `git mv src/agent/config_value.rs src/infra/config/value.rs`
2. `agent/mod.rs` 删 `pub mod config_value;`
3. `infra/config/mod.rs` 加 `pub mod value; pub use value::*;`
4. 全局替换引用路径 `crate::agent::config_value` → `crate::infra::config::value`

> NOTE: 方案 A 仅搬家不改其内部设计。若要做 port 化（`SecretResolver` trait），那是方案 C 的 P4，且需先有第二个实现需求（test 双 / 静态密钥源）才立项，避免过度抽象。

### 4.5 杀 `r#loop`：建 `agent/runtime/`

`loop` 是 Rust 关键字，导致全 crate 4 处 `crate::agent::r#loop::{...}`。**目录化本身救不了**（`agent::loop::X` 仍需转义），必须改名。

步骤：
1. 建目录 `src/agent/runtime/`，加 `mod.rs`：
   ```rust
   pub mod queue;
   pub mod react;
   pub mod retry;
   pub mod stdout_guard;

   pub use react::{AgentEvent, AgentHooks, AgentLoop, AgentEventStream};
   pub use queue::MessageQueue;
   pub use retry::{RetryState, is_retryable_error};
   ```
2. `git mv` 四个文件并改名：
   - `agent/loop.rs` → `agent/runtime/react.rs`
   - `agent/queue.rs` → `agent/runtime/queue.rs`
   - `agent/retry.rs` → `agent/runtime/retry.rs`
   - `agent/output_guard.rs` → `agent/runtime/stdout_guard.rs`
3. `agent/mod.rs`：删 4 个 `pub mod`，加 `pub mod runtime;`
4. 全局替换引用：
   - `crate::agent::r#loop::{AgentEvent, AgentLoop}` → `crate::agent::runtime::{AgentEvent, AgentLoop}`
   - `crate::agent::retry::{...}` → `crate::agent::runtime::retry::{...}`（或通过 re-export 用 `crate::agent::runtime::{...}`）
   - `crate::agent::queue::MessageQueue` → `crate::agent::runtime::MessageQueue`
   - `crate::agent::output_guard` → `crate::agent::runtime::stdout_guard`

受影响文件（已 grep 确认，改名后路径同步更新）：
- `interactive/cli/mod.rs`
- `interactive/print.rs`
- `interactive/rpc.rs`
- `agent/session/mod.rs`（多处）
- `agent/mod.rs`

### 4.6 `interface/` → `interactive/`

`interface` 与编程语义（trait/interface）冲突，且总览词汇表已定名 `interactive`（"代码对交互形态的支持"）。这是纯目录改名 + 批量替换，无逻辑变更。

步骤：
1. `git mv src/interface src/interactive`
2. `lib.rs`：`pub mod interface;` → `pub mod interactive;`
3. 全局替换 `crate::interface::` → `crate::interactive::`
4. feature flag、`diff_review` 等文档/注释里的 `interface` 字样一并更新为 `interactive`

> NOTE: 改名是方案 A 的一部分而非独立方案，因为它零风险且是后续所有方案的前提词汇（B 的 facade、D 的 protocol 都住在 interactive 侧）。一口气改干净，避免新旧名并存期混乱。

## 5. 验证清单

- [ ] `grep -rn "agent::types" src/` = 0
- [ ] `grep -rn "r#loop" src/` = 0
- [ ] `grep -rn "crate::agent::config_value" src/` = 0
- [ ] `grep -rn "crate::agent::model_manager" src/` = 0
- [ ] `grep -rn "crate::agent::compaction_orchestrator" src/` = 0
- [ ] `grep -rn "crate::agent::skill_manager" src/` = 0
- [ ] `grep -rn "crate::agent::traits" src/` = 0（除非 tools::definition 内部）
- [ ] `grep -rn "crate::interface" src/` = 0（已改名 interactive）
- [ ] `grep -rn "pub mod interface" src/lib.rs` = 0
- [ ] `cargo build --all-features` 通过
- [ ] `cargo nextest run --profile ci` 全绿（或 `cargo test`）
- [ ] `cargo clippy --all-features -- -D warnings` 通过
- [ ] `cargo fmt --check` 通过

## 6. 风险与回滚

- **风险**：极低。全部是 `use` 路径重写 + `git mv`，编译器会即时暴露任何遗漏。
- **回滚**：单 PR 单 commit，`git revert` 即可。
- **行为变更**：零。模块路径是编译期符号，不影响运行时行为、序列化格式、CLI 接口。

## 7. 净效果

- agent 顶层散文件 13 → 4
- 消灭 2 个核心名字遮蔽（`types`/`traits`）
- 消灭 4 处 `r#loop` 转义
- `config_value` 回到正确层（运行时域 infra）
- `interface/` → `interactive/`，词汇对齐总览
- 为方案 B/C/D 铺平道路（god-file 拆分、facade/protocol、provider 迁层都依赖干净的模块边界与命名）

## 8. 推荐路线

方案 A 一次性交付即可（1 个 PR）。建议拆成三个 commit 以便 review：

1. **commit 1（纯删除 + 搬家）**：删 types、吸收 3 个 orphan、搬 config_value
2. **commit 2（runtime 目录 + 杀 r#loop）**：建 runtime/、改名、批量替换引用
3. **commit 3（interface → interactive 改名）**：目录改名 + 批量替换
