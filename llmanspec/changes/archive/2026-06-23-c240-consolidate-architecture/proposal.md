---
id: c240-consolidate-architecture
title: "清除 pi 文档引用并评估宏系统替换微系统设计"
depends_on: [c235-fill-test-coverage]
---

## Why

经过 c225（core 层）、c230（模块内聚）、c235（测试覆盖）三轮重构后，xylitol 已是一个**完全独立的 Rust 项目**——零编译期依赖 pi，零运行时引用 pi。但遗留了以下架构债务：

### 1. 文档/注释层面的 pi 引用（30 个文件）

`Aligns with pi's ...` 是早期从 pi（TypeScript 项目）移植/对齐时的标记。现在 xylitol 的 `core/` 层、`agent/` 层、`infra/` 层都有独立设计。这些注释：

- **误导**新贡献者以为 xylitol 是 pi 的 Rust 端口
- **过时**——`core/message.rs` 中 `LlmMessageConverter` trait、`core/traits.rs` 中的 `XyModel`/`XyTool` 等已是自己独立的 API 语义，不再与 pi 的 TypeScript API 一一对应
- **无维护价值**——pi 的 TypeScript 实现演进方向不同，同步注释只会扩大偏差

### 2. 微系统设计（M 个 manager struct + AgentSession facade）

当前 `agent/session.rs`（1272 行）作为 facade 组合了：

| 组件 | 行数 | 方法数 |
|---|---|---|
| ModelManager | 119 | 12 |
| ToolManager | 50 | 4 |
| SkillManager | 88 | 4 |
| SessionIO | 97 | 6 |
| CompactionOrchestrator | 116 | 2 |
| MessageQueue | 285 | 18 |
| 其他直接方法 | ~300 | ~30 |

这种微系统（micro-system）分解源于 c230 的 facade 模式——各组件通过 `AgentSession` 场（field）独立实例化、独立测试。但：

- **Facade 膨胀**：`AgentSession` 仍是 god object（~50+ pub 方法）
- **间接调用**：调用方需要 `session.some_manager.some_method()` 而非 `session.some_method()`
- **组件间耦合**：`ModelManager` 的 `set_thinking_level` 要写回 session 持久化，但持久化由 `SessionIO` 管理——产生了隐式回调

### 3. 宏系统替换的候选区域

Rust 宏（macro）可以替代某些场景的微系统设计：

| 场景 | 当前（微系统） | 候选（宏系统） |
|---|---|---|
| Tool 注册 | `ToolRegistry.register()` 运行时 | `#[tool]` 过程宏 → 编译期注册 |
| Model 构建 | `ModelConfigExt::build()` 运行时 trait 方法 | `#[provider]` 宏 → 匹配 model kind |
| Slash 命令 | `Vec<SlashCommandInfo>` 运行时构建 | `#[command]` 宏 → 静态表 |
| Hook 调度 | `EventBus` 运行时订阅 | 编译期 hook 链 |

但不是所有场景都适合宏——宏会牺牲动态性。需要评估后选择性替换。

## What Changes

1. **清除所有 pi 文档引用**（30 个文件）：
   - 将 `//! Aligns with pi's ...` → 替换为独立中文/英文描述，说明当前模块的实际职责
   - 从 `session.rs` 文档注释移除 "pi's AgentSession class" 引用
   - 从其他模块移除类似引用

2. **评估宏系统可行性**（设计评审，不直接生产代码）：
   - 产出 `docs/architecture/macro-registration.md`：分析每个候选场景的宏替换 ROI
   - 评估 `#[tool]`、`#[command]`、`#[provider]` 宏的方案设计
   - 决策哪些宏替换值得做，哪些保持运行时
   - **此 change 只做评估 + 文档，不生产宏代码**（留到后续独立 change 实施）

3. **微系统合并**——将极小且稳定的组件合并：
   - `tool_manager.rs`（50 行, 4 方法）→ 直接并入 `session.rs`，不再独立组件
   - `model_manager.rs`（119 行, 12 方法）→ 保持独立但简化为数据持有类，不承担持久化回调

## Capabilities

- `architecture`：更新 capability，规约组件粒度（ar01: 组件 ≥100 行或 ≥5 方法才独立；ar02: 静态注册优先于运行时注册）

## Impact

- **规模**：小至中。30 个文件的注释更新是机械的；微系统合并影响 2-3 个文件；宏评估只产出文档。
- **行为**：零行为变更；被合并的组件是纯数据持有，无行为逻辑。
- **风险**：低；509 lib 测试 + 79 BDD 全绿提供回归保障。
- **不做**：不生产宏代码、不改 agent loop 行为、不移动 session 持久化格式、不做 provider 接口变更。
