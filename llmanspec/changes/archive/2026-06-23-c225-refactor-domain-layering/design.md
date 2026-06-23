# Design — c225-refactor-domain-layering

## 决策:如何放置领域核心类型

### 方案 A(采纳):新增 `src/core/` 层

```
src/
  core/       ← 新增:领域核心词汇(AgentMessage / XyModel / XyTool / ModelKind / XyChunk)
  agent/      ← ReAct 循环、session、provider、tools(依赖 core)
  infra/      ← config / session / event / skills(依赖 core,不再依赖 agent)
  interface/  ← CLI / print / rpc / diff_review(依赖 agent + infra)
```

依赖方向:`interface → agent → core ← infra`。agent 与 infra 共享 core,彼此不再互相依赖。

**优点**:根治反向依赖;core 成为稳定的领域内核,agent/infra 都可独立演进;符合"依赖指向稳定抽象"。
**代价**:新增一层;需一次性迁移类型与 import。

### 方案 B(否决):依赖倒置(infra 定义 trait,agent 实现)

infra 定义所需 trait,agent 实现。**否决理由**:infra 需要的是**具体领域类型**(AgentMessage 的字段、XyChunk 的结构),不仅是行为抽象。倒置无法消除对具体类型的依赖,且会引入大量样板 trait。

### 方案 C(否决):把类型移入 infra

方向错误——infra 是支撑层(配置/持久化/IO),不是领域层。把领域词汇放进 infra 会让 agent 反过来依赖 infra,制造新的反向依赖。

## 决策:core 层包含什么(边界)

**纳入 core**(真正的领域基础词汇,被 agent 和 infra 同时需要):
- `AgentMessage` / `AgentPart` / `AgentContent`(消息原语)
- `XyModel` / `XyTool` trait(能力契约)
- `ModelKind` / `ModelConfig` / `ModelMeta`(模型标识与配置)
- `XyChunk` / `XyUsage`(流式与用量类型)
- 相关 error 类型(`XyToolError` 等被 core trait 引用的)

**留在 agent**(编排逻辑,非基础词汇):
- ReAct 循环(`loop.rs`)、`AgentSession`、`AgentEvent` 流、provider 实现、tools 实现、trust/resolver/registry 编排。

**留在 infra**:
- JSONL 持久化、配置加载、事件总线基础设施、skills/MCP 适配器骨架(适配器实现 `core::XyTool` 即可,无需依赖 agent)。

判定准则:**如果一个类型同时被 agent 和 infra 引用,它是 core 候选**;如果只被 agent 内部引用,留在 agent。

## 决策:迁移策略

**big-bang 一次性迁移**(采纳),而非渐进 re-export 再清理。

- 项目规则明确"不留后向兼容 shim",渐进式会违反该规则。
- 类型被广泛引用,机械改写可由编译器逐个定位(改一个 `use` 路径,编译报出下一个)。
- 测试套件(454+83)提供强回归保障。

执行顺序:先建 `core/` 并移动类型 → 改 `lib.rs` 声明 → 按 `agent/` → `infra/` → `interface/` 顺序修正 import → 加架构守卫 → 全量验证。

## 决策:架构守卫形式

在 `tests/` 下新增一个测试(或 build.rs 脚本),grep `src/infra/**/*.rs` 中的 `crate::agent`,命中即失败并打印违规文件。比 clippy lint 更直接、更易理解。可选未来升级为 `cargo-deny` 或自定义 lint,但本期用 grep 测试即可。

## 风险与回滚

- 风险:漏改 import 导致编译失败 → 由编译器即时报错,无静默回归。
- 回滚:纯物理移动,git revert 整个 change 即可恢复,无数据/格式迁移。
