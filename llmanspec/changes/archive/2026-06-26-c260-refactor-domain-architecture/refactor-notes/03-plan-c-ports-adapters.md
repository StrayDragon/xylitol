# 方案 C — Ports & Adapters（运行时迁入 infra + port 收口）

> 野心：运行时（provider/tool）迁入 infra / port 收口 / agent 纯编排
> 风险：中-高（port 边界设计需反复推敲，trait 对象注入要重新理顺）
> 前置：方案 A + B 已完成
> 对齐总览 [00-overview.md](00-overview.md)：服从 HC-1、HC-2、HC-4、HC-5

## 1. 目标

彻底执行"开闭 + traits 化"的三条硬约束：

1. **`core` 只放词汇 + port（trait）+ 纯算法。**
2. **`infra` 是运行时域**——所有 port 实现住这里（provider/tool/session/exec-env…）。
3. **`agent` 只对 port 编排**，不直接 import 任何运行时具体类型。这同时修正方案 B facade 里遗留的 HC-2 偏差（agent 不得持有 session_id / 强制 Session）。

**新增 provider / tool / session 后端 = 新文件 impl trait，零改动 agent。** 这是六边形架构的核心承诺，也是"运行时住 infra、agent 薄编排"这一项目本质的兑现形态。

## 2. 为什么 C 排在最后

方案 C 会引入 `SessionStore`/`EventSink`/`SecretResolver` 等 port 的少量 ceremony（trait 定义 + trait 对象注入 + `Arc<dyn Port>`）。这是**有意识的成本**，不是过度抽象——前提是每个 port 都买到了真实的开闭价值。

C 的 P4（新增 port）故意排在最后且设触发条件：**没有第二个实现的真实需求之前，不立 port。** 这是抗过度抽象的纪律。

## 3. Port 立项判据（抗过度抽象自检表）

每个候选 port 必须通过下表，否则内联：

| Port | 第二实现（现实存在或合理预期） | open-closed 价值 | 判定 |
|---|---|---|---|
| `XyModel` | 已有 4（OpenAI/Anthropic/Fake/Mock） | 已兑现 | ✅ 迁入 `core/ports.rs` |
| `XyTool` | 已有 9 | 已兑现 | ✅ 迁入 `core/ports.rs` |
| `SessionStore` | test 内存后端 / 未来 DB 后端 | agent 可脱离文件系统单测 | ✅ 新增（P4，承重） |
| `EventSink` | test 事件收集器 | loop 可单测事件流 | ✅ 新增（P4，承重） |
| `SecretResolver` | env-only / `!cmd` / 静态 | `$ENV`/`!cmd` 解析与 agent 解耦 | ✅ 吸收 config_value（P4） |
| `SandboxEngine` | 已有 2（Noop + Fallback） | 已兑现 | ✅ 保持（已符合） |
| `LlmMessageConverter` | 多（各 provider） | 已兑现 | ✅ 保持 |
| `BashOperations`（tools/bash.rs） | 若仅 1 实现 | 无 | ❌ 内联（过度抽象） |
| `ModelConfigExt` | 仅 1 | 无 | ❌ 改普通 fn（过度抽象） |
| `SettingsStorage` | 若仅 1 | 无 | ❌ 内联，除非要 test 双 |

> NOTE: 这张表是"开闭原则"与"避免过度抽象"的正面交锋现场。**承重的解耦买，不承重的拆。** 立项前必须填这张表，口说无凭。

## 4. 调整后的目标形态（最终态）

```
core/
├── ports.rs        ← 新：所有反向接口集中
│                    XyModel / XyTool（迁自 traits.rs）
│                    + SessionStore（新 port，infra impl）        [P4]
│                    + EventSink（新 port，infra impl）            [P4]
│                    + SecretResolver（新 port，infra impl）       [P4]
├── message.rs      不变
├── model.rs        不变
├── error.rs        不变
└── types.rs        不变

agent/
├── react.rs        ReAct 算法，只依赖 core::ports + AgentState
├── state.rs        AgentSession 状态（不含任何 infra 具体类型）
├── hooks.rs        AgentHooks 扩展点
├── facade.rs       对外唯一入口（方案 B 已建）
├── tools/          内置工具（impl XyTool）—— 留 agent，因属"默认能力"
└── prompt/         输入侧（方案 B 已建）

infra/
├── provider/       ← 新：从 agent/provider 整体迁入（HTTP adapter，impl XyModel）
│   ├── mod.rs
│   ├── openai.rs
│   ├── anthropic.rs
│   ├── fake.rs
│   └── mock.rs     (cfg test)
├── session/        impl SessionStore（文件后端）                    [P4]
├── event/          impl EventSink（in-memory bus）                  [P4]
├── config/
│   ├── value.rs    方案 A 已迁入；P4 改为 impl SecretResolver
│   ├── loader.rs
│   └── ...
└── ...(其余不变)

interactive/
├── cli/            组合根：把 infra adapter 装进 agent 的 port
├── print/          只依赖 agent + core
├── rpc/            只依赖 agent + core
└── diff_review/    只依赖 agent + core
```

## 5. 迁移映射表（在方案 A+B 之上）

| 操作 | 源 | 目标 | 类型 | 阶段 |
|---|---|---|---|---|
| 迁层 | `agent/provider/openai.rs` | `infra/provider/openai.rs` | adapter 迁层 | P3 |
| 迁层 | `agent/provider/anthropic.rs` | `infra/provider/anthropic.rs` | adapter 迁层 | P3 |
| 迁层 | `agent/provider/fake.rs` | `infra/provider/fake.rs` | adapter 迁层 | P3 |
| 迁层 | `agent/provider/mock.rs` | `infra/provider/mock.rs`(cfg test) | adapter 迁层 | P3 |
| 迁层 | `agent/provider/mod.rs` | `infra/provider/mod.rs` | adapter 迁层 | P3 |
| 迁层 | `agent/tools/{bash,read,write,edit,grep,find,ls,patch,...}.rs` | `infra/tools/` | 运行时迁层 | P3（可选） |
| 收口 | `core/traits.rs` | `core/ports.rs` | port 集中 | P3 |
| 修正 | `agent/facade.rs::Agent::new(session)` | `Agent::new(ports...)`，去 session_id | HC-2 修正 | P4-a |
| 立项 | — | `core::SessionStore` trait + `infra::session` impl | 新 port | P4 |
| 立项 | — | `core::EventSink` trait + `infra::event` impl | 新 port | P4 |
| 立项 | `infra/config/value.rs` | `core::SecretResolver` trait + `infra/config` impl | port 化 | P4 |

## 6. 逐项执行细则

### 6.1 P3-a：`core/traits.rs` → `core/ports.rs`

把现有 `XyModel`/`XyTool` 及其辅助类型（`XyStream`/`XyToolCtx`/`ToolExecutionMode`）集中到 `core/ports.rs`，文件名表达"这是 agent 依赖的外部能力契约"。

**步骤：**
1. `git mv src/core/traits.rs src/core/ports.rs`
2. 改文件头注释：明确"ports = agent 反向依赖的能力契约"。
3. `core/mod.rs`：`pub mod traits;` → `pub mod ports;`
4. 全局替换 `crate::core::traits` → `crate::core::ports`
5. 为向后兼容可临时 `core/mod.rs` 加 `pub use ports::*;`，但项目规范是"不留兼容 shim"（见 AGENTS.md），直接改干净。

> NOTE: `XyToolCtx`/`ToolExecutionMode` 是 port 的辅助类型，随 port 一起留在 `ports.rs`。它们不是独立概念，没必要拆。

### 6.2 P3-b：provider adapter 整体迁 `infra`

这是方案 C 最显眼的物理变更。provider 是"连接外部 LLM API 的 adapter"，本应属于 infra。

**当前耦合现状（已 grep 确认）：**
```
agent/provider/*.rs 仅依赖 crate::core::{traits,error,message,types}  ← 干净！
```

provider 对 agent 内部零依赖，迁层几乎是无痛的。

**步骤：**
1. `git mv src/agent/provider src/infra/provider`
2. `agent/mod.rs` 删 `pub mod provider;`
3. `infra/mod.rs` 加 `pub mod provider;`
4. 全局替换 `crate::agent::provider` → `crate::infra::provider`
5. 确认 provider 注册点（`agent/model/registry.rs` 或 `cli` 组合根）改为从 `infra::provider` 构造

**关键验证：**
```bash
# agent 内部不得出现 provider 具体类型
grep -rn "infra::provider::OpenAIProvider\|infra::provider::AnthropicProvider" src/agent/
# 期望：0（构造只能在 interface 组合根）
```

> NOTE: provider 迁层后，`agent/model/` 里的"模型注册表"职责需要重新审视。注册表是"持有 ModelConfig + 元数据"的纯数据结构，可留 agent；但"从 config 构建 provider 实例"的工厂应移到 interactive 组合根（或 infra::provider::factory）。这条边界划在哪，取决于 provider 构造是否需要 agent 上下文——若不需要（通常不需要），就移出 agent。

### 6.2b P3-c：内置工具迁 `infra/tools`（可选，遵循同一规则）

内置工具（bash/read/write/edit/grep/find/ls/patch）是"在执行环境中做事"的运行时，与 provider 同质——都 impl 一个 `core` port、都触碰外部世界。按"运行时住 infra"的规则，它们也应迁入 `infra/tools/`。

**与 provider 的区别（为何标"可选"）：** 工具数量多、且工具注册表（`ToolRegistry`）是轻量编排状态（一个 `Vec<Arc<dyn XyTool>>`），可留 agent。所以折中：**实现迁 infra，注册表留 agent**。

**步骤：**
1. `git mv src/agent/tools/{bash,read,write,edit,grep,find,ls,patch,mutation,accumulator,truncate,process,path_utils}.rs src/infra/tools/`
2. `agent/tools/` 仅留 `registry.rs`（原 mod.rs 的 `ToolRegistry`）+ `definition.rs`（方案 A 后的 `ToolDefinition`）。
3. `infra/tools/mod.rs` re-export 各工具 struct。
4. 注册点（cli 组合根）从 `infra::tools` 收集内置工具注入 `ToolRegistry`。

> NOTE: 这一步标"可选"是 HC-5 的体现——provider 迁层是并 HC-4 的刚需（新增厂商频繁），工具迁层是规则一致性收益、非阻塞。可随真实改动顺手做，不单独立项。

### 6.3 P4-a：HC-2 修正 + `SessionStore` port（触发条件：test 需要内存后端）

**前置：HC-2 修正。** 方案 B 的 facade 为控制改动面保留了 `Agent::new(session)` / `run(prompt, session_id)`。C 阶段先修正它：

- `Agent::new(ports...)`：构造只接受 `Arc<dyn Port>`（store/sink/providers），**不强制 Session**。
- `run(prompt)`：不传 `session_id`；session_id 是 `SessionStore` 的查询键，在交互层绑定，不进编排状态。
- 这让 agent 可脱离具体后端、被 server（方案 D）自由托管。

**仅在以下情况立 `SessionStore` port：** agent 的 ReAct loop / compaction 单测因为依赖真实文件系统而难写时。

**port 定义（`core/ports.rs`）：**
```rust
/// Persistence port — abstracts session storage so agent can be unit-tested
/// without a real filesystem.
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn append(&self, session_id: &str, entry: SessionEntry) -> Result<(), XyError>;
    async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, XyError>;
    async fn exists(&self, session_id: &str) -> bool;
    // ... 仅暴露 agent 真正需要的方法
}
```

**adapter 实现（`infra/session/`）：**
- 现有 `SessionManager` impl `SessionStore`（文件后端）
- 新增 `InMemorySessionStore`（test 用，放 `tests/support/` 或 `cfg(test)`）

**注入：**
```rust
// agent/state.rs
pub struct AgentState {
    store: Arc<dyn SessionStore>,  // 注入，agent 不知道后端
    // ...
}
```

> NOTE: `SessionStore` 的方法集必须**只含 agent 真正调用的**，不是 `SessionManager` 的全量方法抄一遍。抄全量 = 把 god-object 的接口也 trait 化了，违背开闭。

### 6.4 P4-b：新增 `EventSink` port（触发条件：loop 单测需要）

**仅在以下情况立项：** `react.rs` 的单测因为依赖 `EventBus` 的真实广播而难写时。

```rust
/// Event emission port — abstracts lifecycle event delivery.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &AgentLifecycleEvent);
}
```

现有 `infra::event::EventBus` impl `EventSink`；test 用 `RecordingSink`（收集事件断言）。

### 6.5 P4-c：`SecretResolver` port 化（吸收 config_value）

方案 A 已把 `config_value.rs` 搬到 `infra/config/value.rs`。P4 把它 port 化：

```rust
/// Secret resolution port — resolves API keys / headers from env, shell, or static.
pub trait SecretResolver: Send + Sync {
    fn resolve(&self, reference: &str) -> Result<String, XyError>;
}
```

现有解析逻辑 impl `SecretResolver`（env/cmd/default 三策略组合）。test 用 `StaticResolver`（固定值）。

> NOTE: 这一步只有在"agent 内部需要直接解析密钥"的场景才承重。若密钥解析全部发生在 cli 组合根（构造 provider 之前），agent 根本不碰密钥，那 `SecretResolver` 就是过度抽象——此时 `infra/config/value.rs` 作为普通工具函数留在 infra 即可，不立 port。

## 7. 组合根（interactive::cli）的最终形态

方案 C 完成后，`interactive/cli` 是唯一知道所有具体类型的地方（另一个组合根是方案 D 的 server）：

```rust
// interactive/cli/mod.rs —— composition root (in-process mode)
async fn run() -> Result<()> {
    let config = load_app_config(...)?;

    // 构造 adapters（infra）
    let store: Arc<dyn SessionStore> = Arc::new(SessionManager::new(sessions_dir));
    let sink: Arc<dyn EventSink> = Arc::new(EventBus::new());
    let secrets: Arc<dyn SecretResolver> = Arc::new(ConfigSecretResolver::new());

    // 构造 providers（infra）
    let providers: Vec<Arc<dyn XyModel>> = config.models.iter()
        .map(|m| build_provider(m, secrets.as_ref()))
        .collect();

    // 构造 agent（注入 port，HC-2：不强制 Session、不传 session_id）
    let agent = Agent::new(store, sink).with_providers(providers);

    // 选交互形态（方案 D 后：interactive 只认 protocol+Driver）
    if args.rpc { rpc::run(InProcessDriver::new(agent)).await }
    else { print::run(InProcessDriver::new(agent)).await }
}
```

**验证边界纯净性：**
```bash
# agent 不得 import 任何 infra 具体类型（只依赖 core::ports）
grep -rn "crate::infra::" src/agent/
# 期望：0（内置工具迁 infra 后，agent 连 tools 子层都不再持具体类型）
```

## 8. 验证清单

### P3
- [ ] `grep -rn "crate::agent::provider" src/` = 0
- [ ] `grep -rn "crate::core::traits" src/` = 0（已改名 ports）
- [ ] `grep -rn "infra::provider::[A-Z]" src/agent/` = 0（agent 不碰具体 provider 类型）
- [ ] 新增一个 OpenAI 兼容厂商 = 仅加一个 `infra/provider/xxx.rs` + cli 注册，零改 agent

### P4（每个 port 独立验证）
- [ ] `SessionStore` 有 ≥2 实现（文件 + 内存 test 双）
- [ ] `EventSink` 有 ≥2 实现
- [ ] `SecretResolver` 仅在 agent 确实需要解析时才立项（否则降级为 infra 工具函数）
- [ ] agent 的 ReAct loop 可脱离文件系统单测（用内存 port 双）

### 全程
- [ ] `cargo build --all-features` 通过
- [ ] `cargo nextest run --profile ci` 全绿
- [ ] `cargo clippy --all-features -- -D warnings` 通过
- [ ] BDD 全绿
- [ ] 层边界编译期断言（见方案总览第 6 节）通过

## 9. 风险与回滚

- **风险点 1（P3）**：provider 迁层后，`agent/model/registry.rs` 若持有 provider 构造逻辑，会产生跨层耦合。需把"从 ModelConfig 构造 Arc<dyn XyModel>"的工厂移到 `infra/provider/factory.rs` 或 cli。
- **风险点 2（P3）**：`Arc<dyn XyModel>` 注入后，错误类型/生命周期需理顺（`XyStream` 的 `Send` 约束已有，但 trait object 的 async 方法要确认 `async_trait` 兼容）。
- **风险点 3（P4）**：port 方法集设计是难点。抄全量 = trait 化 god-object；抄太少 = 反复扩 port 接口。建议先写 agent 单测驱动 port 方法集（TDD 反向定义接口）。
- **风险点 4（P4）**：过度抽象陷阱。每立一个 port 前必须填第 3 节的自检表，第二实现不存在就降级为内联。
- **回滚**：P3 独立 PR；P4 每个 port 独立 PR，互不依赖。

## 10. 净效果

- 真正的六边形架构：agent 纯编排，所有外部能力是 port
- `grep "crate::infra::provider::" src/agent/` = 0
- 新增 OpenAI 兼容厂商 = 加一个文件 impl `XyModel` + cli 注册
- agent 可脱离文件系统/真实 HTTP 单测
- 长期最低的迭代门槛（开闭原则完全兑现）

## 11. 推荐路线

| PR | 内容 | 阶段 | 风险 |
|---|---|---|---|
| C-1 | `core/traits.rs` → `core/ports.rs` + 全局改名 | P3-a | 低 |
| C-2 | provider 整体迁 `infra/provider` + 工厂移位 | P3-b | 中 |
| C-3 | `SessionStore` port（仅在 test 痛点出现后） | P4-a | 中 |
| C-4 | `EventSink` port（同上触发条件） | P4-b | 中 |
| C-5 | `SecretResolver` port（仅在 agent 需解析时） | P4-c | 中 |

> NOTE: C-3/C-4/C-5 全部带触发条件。**不要为了"架构完整"提前立 port。** 触发条件未满足时，留在方案 A/B 的形态即可，架构已经足够干净。

## 12. 何时停止

方案 C 不是"必须做完才停"。决策树：

```
P0-P2（方案 A+B）做完 → 架构已显著改善，可长期在此形态迭代
                      │
                      ├─ 新增厂商频繁？ → 做 P3（provider 迁层）
                      │
                      └─ agent 单测被 IO 拖累？ → 做对应 P4 port
                                                （一次只做一个，验证承重后再下一个）
```

架构重构的目标是**降低未来迭代的成本**，不是达成某种"完美形态"。当进一步重构的边际收益低于新功能开发时，就停。
