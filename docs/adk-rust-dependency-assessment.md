# adk-rust 依赖评估报告

> **评估对象**: [zavora-ai/adk-rust](https://github.com/zavora-ai/adk-rust) v0.9.1
> **消费方**: xylitol v0.0.0-dev（当前使用 adk-rust v0.8.2 from crates.io）
> **评估日期**: 2026-05-24

---

## 一、当前依赖关系

xylitol 依赖 5 个 adk-rust crate，全部来自 crates.io v0.8.2：

| xylitol 模块 | adk crate | 使用的 API |
|---|---|---|
| `agent/loop.rs` | `adk-agent` | `LlmAgentBuilder` |
| `agent/loop.rs` | `adk-runner` | `Runner`, `RunnerConfig` |
| `agent/loop.rs` | `adk-core` | `Content`, `Event`, `Part` |
| `agent/model.rs` | `adk-model` | `OpenAIClient`, `AnthropicClient`, `OpenAIConfig`, `AnthropicConfig` |
| `agent/tools/*.rs` (7个) | `adk-core` | `Tool`, `ToolContext`, `AdkError`, `ErrorComponent`, `ErrorCategory` |
| `agent/planner.rs` | `adk-core` | `Llm`, `LlmRequest`, `LlmResponse` |
| `agent/provider/fake.rs` | `adk-core` | `Llm` trait（自定义 mock 实现） |
| `infra/security/mod.rs` | `adk-core` | `Tool`, `ToolContext`（wrapper） |
| `infra/skills/mcp.rs` | `adk-core` | `AdkError`（MCP 适配器） |
| `interface/cli/mod.rs` | `adk-session` | `InMemorySessionService` |
| `interface/tui/app.rs` | `adk-session` | `SessionService`, `ListRequest` |
| 测试代码 | `adk-model` | `MockLlm` |

**关键观察**：xylitol 的 adk 依赖集中在 **核心 trait** 和 **少数具体类型** 上，接触面相对可控。

---

## 二、adk-rust 质量评估

### 2.1 评分总览

| 维度 | 评分 | 详情 |
|---|:---:|---|
| 成熟度 | ★★★★☆ | 已发布 crates.io，6个月27个版本，正式稳定性分级 |
| 核心代码质量 | ★★★★★ | 干净的 trait 设计、结构化错误、builder pattern、全面 tracing |
| 测试 | ★★★★★ | ~198 集成测试、72 proptest 文件、3平台CI、contract tests |
| API 稳定性 | ★★★★☆ | `#[non_exhaustive]`、semver CI、N+2 弃用策略；仍是 pre-1.0 |
| 文档 | ★★★★☆ | 52页官方文档、56个示例、rustdoc 覆盖核心 crate |
| 依赖健康 | ★★★★☆ | 主流 crate、feature-gated 重依赖、统一 crypto provider |
| 社区/维护 | ★★★☆☆ | 单一团队维护、GitHub stars/adoption 未知、无广泛社区验证 |

### 2.2 核心优势

1. **正式稳定性合约（STABILITY.md）**
   - 12 个 Stable crate（含 xylitol 使用的全部 5 个）
   - N+2 minor 弃用策略
   - `cargo-semver-checks` CI 强制执行

2. **结构化错误体系**
   - `AdkError` 带 retry hints、HTTP 映射、property test 验证
   - `ErrorComponent` / `ErrorCategory` 均为 `#[non_exhaustive]`

3. **严格 CI 门控**
   - `clippy -D warnings`（零容忍）
   - nextest + 跨平台（Linux/macOS/Windows）
   - rustdoc 覆盖检查（12 个 Stable crate）
   - semver-checks 自动 PR 检查

4. **feature-gated 架构**
   - `minimal` / `standard` / `enterprise` / `full` 四层
   - xylitol 只用 minimal + openai + anthropic，编译开销可控

### 2.3 风险点

1. **Pre-1.0 API 不保证向后兼容**
   - 虽有弃用策略，但 0.8.5 仍删除了已废弃 API
   - 0.8.5 `RunnerConfig` 加了 `#[non_exhaustive]` 是 breaking change

2. **单一维护团队**
   - 无广泛社区采用验证
   - 如果维护者停止更新，依赖方承担 fork 维护成本

3. **复杂度过高**
   - 35+ crate、54+ 示例
   - `llm_agent.rs` 单文件 2400 行（维护热点）

4. **部分功能不完整**
   - Experimental crate（sandbox/audio/realtime）有 stub
   - 空的测试文件（如 `openai_responses_integration_tests.rs`）

5. **无代码覆盖率报告**
   - CI 不追踪覆盖率，无法验证 1.0 roadmap 的 90% 目标

---

## 三、0.8.2 → 0.9.1 升级评估

### 3.1 Breaking Changes 清单

| 变更 | 影响 crate | 对 xylitol 的影响 |
|---|---|---|
| `RunnerConfig` / `RunConfig` 变为 `#[non_exhaustive]` | `adk-runner` / `adk-core` | ⚠️ **中等** — 需确认是否有 struct literal 用法 |
| `DatabaseSessionService` 别名删除 | `adk-session` | ✅ **无影响** — xylitol 使用 `InMemorySessionService` |
| `RustCodeTool` 删除 | `adk-tool` | ✅ **无影响** — xylitol 未使用 |
| `sanitize_schema` 删除 | `adk-tool` | ✅ **无影响** — xylitol 未使用 |
| `server` feature 含 A2A v1 | `adk-server` | ✅ **无影响** — xylitol 未使用 server |
| `McpToolset::tools()` 返回原始 schema | `adk-tool` | ✅ **无影响** — xylitol 自建 MCP 适配 |

### 3.2 升级建议

**影响较小，可以安全升级。** 主要检查 `Runner::builder()` / `RunnerConfig` 构建方式即可。

---

## 四、Rust 生态替代方案调研

### 4.1 主要候选框架

| 框架 | Stars | 下载量(90d) | 版本 | 维护者 | 许可证 | 特点 |
|---|:---:|:---:|---|---|---|---|
| **[rig](https://github.com/0xPlaygrounds/rig)** | 7,221 | 588,931 | 0.37.0 | 210 contributors | MIT | 最成熟、社区最大 |
| **adk-rust** | ? | ? | 0.9.1 | 单一团队 | Apache-2.0 | 最全功能、xylitol 已集成 |
| **[AutoAgents](https://github.com/liquidos-ai/AutoAgents)** | 529 | ? | 0.3.7 | 多 contributors | Apache-2.0 | 多 agent 编排、WASM sandbox |
| **[daimon](https://github.com/Lexmata/daimon)** | 2 | ? | 0.16.0 | 1 contributor | Apache-2.0 | 小巧、ReAct 专注 |
| **[swarms-rs](https://crates.io/crates/swarms-rs)** | ? | 521 | 0.2.1 | Swarm Corp | Apache-2.0 | 企业多 agent |
| **[rswarm](https://docs.rs/rswarm)** | ? | ? | 0.1.8 | ? | ? | Swarm 移植、SQLite/PG 持久化 |
| **[agent-runtime](https://crates.io/crates/agent-runtime)** | ? | 102 | 0.3.0 | 1 contributor | MIT/Apache-2.0 | MCP 原生、小型 |

### 4.2 核心对比：rig vs adk-rust

| 对比项 | rig (0.37.0) | adk-rust (0.9.1) |
|---|---|---|
| **社区规模** | 7,200+ stars, 210 contributors, 588K 下载/90d | 未知 stars, 单一团队 |
| **API 稳定性** | 明确警告 "Here be dragons"，频繁 breaking | 有 STABILITY.md 分级、semver CI |
| **Provider 支持** | 20+ provider | 10+ provider（feature-gated） |
| **Tool 系统** | `Tool` trait + ToolSet + MCP | `Tool` trait + MCP + 确认策略 |
| **Session 管理** | `memory` 模块 | 6 种后端（SQLite/PG/Redis/MongoDB/Firestore/Neo4j） |
| **多 Agent 编排** | 有但较基础 | 5 种 agent 类型 + graph workflow |
| **Realtime/Audio** | 无 | WebRTC/Live API/TTS/STT |
| **安全** | 无内建 | Auth/Guardrail/Sandbox |
| **WASM** | 核心支持 | 不支持 |
| **Agent 协议** | 无 A2A | A2A v1.0 + AWP + ACP |
| **适合场景** | 通用 LLM 应用、RAG、嵌入 | 全栈 agent 平台、coding agent |

### 4.3 对 xylitol 的适配评估

xylitol 使用的核心能力 vs 各框架覆盖度：

| xylitol 需求 | adk-rust | rig | AutoAgents | 自建 |
|---|:---:|:---:|:---:|:---:|
| ReAct agent loop | ✅ 原生 | ✅ 原生 | ✅ 原生 | 🔧 ~500行 |
| Tool trait + 7 工具 | ✅ 原生 | ✅ 原生 | ✅ 原生 | 🔧 ~300行 |
| OpenAI/Anthropic 客户端 | ✅ 原生 | ✅ 原生 | ✅ 原生 | 🔧 用 async-openai 等 |
| 流式输出 | ✅ 原生 | ✅ 原生 | ✅ 原生 | 🔧 ~200行 |
| MockLlm 测试 | ✅ 原生 | ✅ test_utils | ❌ | 🔧 ~100行 |
| Session 管理 | ✅ 原生 | ✅ 基础 | ✅ 基础 | 🔧 ~200行 |
| 结构化错误 | ✅ AdkError | ❌ anyhow | ❌ | 🔧 ~150行 |
| `#[non_exhaustive]` config | ✅ | ❌ | ❌ | N/A |

---

## 五、迁移成本评估

### 5.1 如果迁移到 rig

| 工作项 | 估计工作量 | 风险 |
|---|---|---|
| 替换 `Tool` trait 实现（7个工具 + security wrapper + MCP adapter） | 2-3 天 | 中（API 差异较大） |
| 替换 `LlmAgentBuilder` → rig `AgentBuilder` | 0.5 天 | 低 |
| 替换 `Runner` → rig agent run loop | 1-2 天 | 中（事件流模型不同） |
| 替换 `SessionService` | 0.5-1 天 | 低 |
| 替换 `MockLlm` 测试 | 0.5 天 | 低 |
| 替换 `AdkError` → 自定义/anyhow | 1-2 天 | 中（错误分类需重建） |
| `FakeProvider` 重写 | 0.5 天 | 低 |
| Planner 的 `Llm` trait 替换 | 0.5 天 | 低 |
| 回归测试 + 调试 | 2-3 天 | 高 |
| **总计** | **~8-13 天** | |

**额外风险**：rig 自身明确警告 "Here be dragons, future updates will contain breaking changes"——从一个 pre-1.0 换到另一个频繁 breaking 的框架，收益有限。

### 5.2 如果迁移到自建

| 工作项 | 估计工作量 | 说明 |
|---|---|---|
| 基础 `Llm` trait + provider 接口 | 2-3 天 | 可直接用 `async-openai` + `anthropic-sdk` |
| ReAct loop 实现 | 3-5 天 | 流式 tool call 解析是核心难点 |
| `Tool` trait + registry | 1-2 天 | |
| Session 管理 | 1-2 天 | 内存版简单，持久化需额外开发 |
| Mock / 测试基建 | 1-2 天 | |
| 错误体系 | 0.5-1 天 | |
| 持续维护 provider API 变更 | 持续成本 | 每次 provider API 变更需要手动适配 |
| **总计（初始）** | **~10-15 天** | |
| **持续维护** | **~2-4 天/月** | 跟进 provider 变更、修 bug |

### 5.3 如果 fork adk-rust 自维护

| 工作项 | 估计工作量 | 说明 |
|---|---|---|
| Fork + CI 搭建 | 0.5 天 | |
| 裁剪无用 crate（~25个） | 1-2 天 | 只保留 core/agent/model/runner/session |
| 切换到 path/git 依赖 | 0.5 天 | |
| 初始审计 + 理解代码 | 2-3 天 | 重点理解 `llm_agent.rs`（2400行） |
| **总计（初始）** | **~4-6 天** | |
| **持续维护** | **~1-3 天/月** | cherry-pick 上游修复、provider 更新 |
| **优势** | | 版本完全可控、可按需裁剪、可随时合并上游更新 |

---

## 六、建议方案

### 推荐：短期继续用 crates.io + 中期准备 fork 能力

#### 阶段一：当前（继续迭代 xylitol）

1. **升级到 0.9.1**（breaking change 对 xylitol 影响小）
2. **加强抽象层隔离**：
   - xylitol 的 `AgentEvent` 模式很好，继续保持
   - 避免让 `adk_core::Event` 等类型泄漏到 interface 层
   - 所有 adk 类型集中在 `agent/` 层使用
3. **固定精确版本**（`=0.9.1` 而非 `"0.9"`）

#### 阶段二：准备 fork 基础设施

1. **创建 fork**（不急于裁剪，保持与上游同步能力）
2. **建立 cherry-pick 流程**：上游有用的 commit 选择性合入
3. **只在需要时切换**：
   - 上游出现严重 breaking change 且不适用时
   - 上游停止维护时
   - 需要深度定制时（如裁剪、性能优化）

#### 不推荐的方案

- ❌ **迁移到 rig**：rig 社区更大但同样频繁 breaking，且缺少 xylitol 需要的结构化错误和安全功能
- ❌ **现在就全部自建**：xylitol 处于 v0.0.0-dev，应优先迭代产品而非基础设施
- ❌ **迁移到其他小框架**：daimon/swarms-rs/agent-runtime 成熟度更低

---

## 七、xylitol 解耦策略（降低未来迁移成本）

### 7.1 接口层隔离（已有，建议强化）

```
interface/     ← 不应出现任何 adk_* 类型
  ↓ AgentEvent（自有类型）
agent/         ← adk 类型的唯一使用区域
  ↓
infra/         ← 仅通过 agent/ 间接接触 adk
```

### 7.2 Trait 薄封装

为 xylitol 定义自己的 trait，用 adapter pattern 桥接 adk-rust：

```rust
// xylitol 自有 trait（不依赖 adk）
pub trait XyModel: Send + Sync {
    async fn generate(&self, req: XyRequest) -> Result<XyResponse>;
}

// 桥接 adapter（依赖 adk）
pub struct AdkModelAdapter(Arc<dyn adk_core::Llm>);
impl XyModel for AdkModelAdapter { /* ... */ }
```

如果未来需要换底层框架，只需替换 adapter 实现。

### 7.3 关键隔离点（按优先级）

| 优先级 | 接口 | 当前 adk 类型 | 建议封装 |
|:---:|---|---|---|
| P0 | Model/LLM | `Llm`, `LlmRequest`, `LlmResponse` | 自定义 `XyModel` trait |
| P0 | Tool | `Tool`, `ToolContext` | 自定义 `XyTool` trait |
| P1 | Error | `AdkError` | 自定义 `XyError` + `From<AdkError>` |
| P1 | Runner | `Runner`, `RunnerConfig` | 自定义 `AgentRunner` facade |
| P2 | Session | `SessionService` | 自定义 `XySession` trait |
| P2 | Event | `Event`, `Content`, `Part` | 已有 `AgentEvent`，继续扩展 |

**注意**：不建议现在全部封装。按 xylitol 迭代节奏，优先 P0（Tool/Model），在重构时逐步完成。

---

## 八、结论

| 问题 | 回答 |
|---|---|
| adk-rust 代码质量靠谱吗？ | **靠谱**。核心 crate 质量高于平均水平（严格 CI、proptest、semver-checks） |
| 继续用合适吗？ | **短中期合适**。xylitol 只用 5 个 Stable crate，接触面可控 |
| 最大风险是什么？ | **维护持续性**。单一团队维护 35+ crate 的长期可持续性存疑 |
| 需要现在换吗？ | **不需要**。没有替代方案在所有维度上都优于 adk-rust |
| 建议的防御策略？ | **加强抽象隔离 + 准备 fork 能力**，在上游不满足需求时可快速切换 |
| 对 xylitol 迭代的影响？ | **应优先产品迭代**，依赖管理工作控制在最小必要范围内 |
