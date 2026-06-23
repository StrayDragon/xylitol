---
id: c235-fill-test-coverage
title: "填补测试缺口并建立 BDD-vs-单元测试 的分界标准"
depends_on: [c225-refactor-domain-layering, c230-refactor-module-cohesion]
---

## Why

架构重构（c225/c230）完成后，当前代码库有 **421 个单元测试 + 79 个 BDD 场景** 全绿通过，但审计发现以下测试缺口与策略问题：

| 问题 | 具体表现 | 影响 |
|---|---|---|
| 核心数据类型无测试 | `core/error.rs`、`core/traits.rs`、`core/types.rs`、`core/model.rs` 缺少 `#[cfg(test)]` | 类型约定变更无安全网 |
| 纯逻辑组件无测试 | `agent/queue.rs`（MessageQueue 推/拉/清空逻辑）、`agent/retry.rs`（状态机）、`agent/commands.rs`（命令解析）、`agent/templates.rs`（模板展开）零覆盖 | 纯函数回归成本低，测试收益高 |
| Session 子组件无测试 | `model_manager.rs`、`tool_manager.rs`、`skill_manager.rs` 作为 c230 抽取的责任组件未附带测试 | 组件边界未锁定为契约 |
| 测试策略未文档化 | 哪些测法（BDD vs 单测）适合什么场景无标准 | 新贡献者可能重复覆盖或遗漏 |

与此同时，`../pi` 项目（TypeScript）的测试结构展示了清晰的职责分配：纯数据结构/算法用直接单元测试；全栈编排用 AgentHarness 集成测试；真实 provider 调用用 E2E 测试。xylitol 应建立类似的 Rust 版本策略，重点是 **BDD 与单元测试不重叠**。

## What Changes

1. **新增核心类型单元测试**（`core/`）：
   - `error.rs`：`XyError` / `XyToolError` 的 Display、From 转换、变体匹配
   - `traits.rs`：`XyModel` / `XyTool` 的 mock 实现验证 trait 契约可正确执行
   - `types.rs`：`XyChunk` 变体构造、`ThinkingLevel` 序列化/反序列化、`XyToolSchema` / `ModelMeta` 构造
   - `model.rs`：`ModelKind::from_provider_name` 解析、`ModelConfig` 构建

2. **新增纯逻辑组件单元测试**（`agent/`）：
   - `queue.rs`：MessageQueue 所有方法（push/steer/followUp/drain/clear/pending_count），含边界情况（空队列、多次 drain、混合 steering+followup）
   - `retry.rs`：RetryState 状态转换（init → prepare → abort → reset）和超时判定
   - `commands.rs`：SlashCommandInfo 构造、is_slash_command 解析、find_command 匹配
   - `templates.rs`：PromptTemplate expand 逻辑，含参数插值、缺失参数处理
   - `config_value.rs`：ConfigValue 解析/展开逻辑（若无现有测试覆盖）

3. **新增 Session 子组件单元测试**（`agent/`）：
   - `model_manager.rs`：cycle_forward、select_model、set_thinking_level boundary
   - `tool_manager.rs`：register、set_active_tools 过滤
   - `skill_manager.rs`：skill 激活、XML 命令展开

4. **建立测试策略文档**——在 `docs/testing-strategy.md` 中明确：
   - **BDD 场景覆盖**：agent 循环编排、tool 执行流程、session 生命周期、CLI 命令分发、错误消息输出 —— 这些需要进程级别 setup
   - **单元测试覆盖**：pure data 结构（消息序列化、类型枚举）、算法（cut point 检测、token 估算）、组件内状态机（queue、retry、command 解析）—— 这些是快速、隔离的
   - **两者不重叠**原则：如果一个行为已在 BDD 中覆盖（例如 "agent 正确处理工具调用"），单元测试只测该行为用到的底层数据结构和函数边界，不重复验证全流程。

## Capabilities

- `testing-standards`：新增 capability，规约 BDD 与单元测试的分界标准（ts01/ts02）。

## Impact

- **规模**：中型，预计新增 60-120 行新测试代码（不含 fixture），零生产代码变更。
- **行为**：零行为变更；仅新增测试覆盖。
- **风险**：低；现有 421 单元 + 79 BDD 全绿提供回归保障。
- **不做**：不改 agent loop 行为、不改 session 持久化逻辑、不新增 BDD feature 文件（已有 14 个足够）、不触及 compaction 测试（已有覆盖）。

## 测试策略决策总结

受 `../pi` 项目（TypeScript vitest 分层）启发：

```
BDD 场景 (rstest-bdd, .feature)
  ├─ 端到端编排：agent 循环、tool 调用、session 生命周期
  ├─ 用户可见错误：缺少参数、权限拒绝、沙箱拦截
  └─ 跨组件集成：hooks → agent → tool、配置加载 → agent 行为

单元测试 (#[cfg(test)] in source)
  ├─ 纯算法：cut point 检测、token 估算、队列操作
  ├─ 类型契约：序列化/反序列化、Display、From 转换
  ├─ 状态机：retry 状态转换、queue 生命周期
  └─ 组件边界：model_manager cycle、tool_manager 过滤

不测试（无重复覆盖）：
  └─ BDD 已覆盖的全流程路径不重复写单元测试
```
