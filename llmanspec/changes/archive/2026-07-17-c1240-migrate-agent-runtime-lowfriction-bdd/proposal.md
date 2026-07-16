---
change_id: c1240-migrate-agent-runtime-lowfriction-bdd
title: "BDD-on 迁移：agent-runtime 低摩擦批次（5 场景）"
status: proposal
priority: 1240
apply_band: P3-core
depends_on: []
author: agent
track: R
wave: bdd-migration
---

# c1240-migrate-agent-runtime-lowfriction-bdd

## Why

c1215 试点时把 agent-runtime spec 全部 27 个 scenario 批量标了 `feature:true`，但只手工生成了
2 个 .feature 场景（react-terminates、stream-is-xyevent）并绑定 step。其余 22 个处于「声明
可执行但未落地」的悬空状态——声明与实现不一致，违反 solidify 链路的可执行合约。

本变更推进**低摩擦批次**（5 个 scenario），让声明与实现一致，并在 c1220「纯文本 step 避引号
陷阱」基础上固化第二条迁移策略。

## What Changes

1. **delta `agent-runtime` modify ar3/ar4/ar5/ar12**：给这 4 个 requirement 补「该行为 MUST 有
   可执行 BDD 场景」语义，并新增 5 个 op_scenario（feature:true）。
2. **生成 .feature**：在 `llmanspec/specs/agent-runtime/agent-runtime.feature` 追加 5 个 solidify
   风格场景（continues-after-tools、done-not-turn-end、abort-drops-sse、builder-build、ports-exist）。
3. **bdd.rs 绑定**：
   - 2 个 scenario（ar3 × 2）**零新代码**，复用现有 step 函数（`mock 模型先 tool 后无 tool` +
     `运行 AgentRuntime` + `turn_end 事件包含 toolResult`，策略二对齐 spec.toon）。
   - ar12 的 when/then 复用现有 step（`经 Driver 启动会话并在首个 TextDelta 后 abort` +
     `事件流包含 aborted 错误`），但 given 需新写自足 step（solidify feature 无 Background，
     现有 `mock 模型慢速流式返回 {n} 段文本间隔 {ms} 毫秒` 依赖 agent.feature Background 装配）。
     故 ar12 given 改用新写纯文本 step `已装配慢速流式 mock 模型`（策略一），内部完整装配
     model + workspace + slow stream。
   - 2 个 scenario（ar4、ar5）新写 trivial step（构造断言 + 端口存在性断言）。

## 关键决策：迁移策略二（调 spec.toon 对齐现有 step）

c1220 固化的策略一（纯文本 step 避引号陷阱）适用于「无现成 step 可复用」的场景。本变更固化
**策略二**：当现有 step 已覆盖 scenario 语义时，优先**调整 spec.toon 的 given/when/then 文本对齐
现有 step 字符串**，实现零新代码复用。

- ar3 `continues-after-tools` / `done-not-turn-end`：spec.toon 文本从「模型返回 FunctionCall 后
  Done / 循环 / 继续下一轮模型调用并带上 toolResult」调整为「mock 模型先 tool 后无 tool / 运行
  AgentRuntime / turn_end 事件包含 toolResult」，全复用现有 step。
- ar12 `abort-drops-sse`：when/then 调整为复用现有 step（`经 Driver 启动会话并在首个 TextDelta
  后 abort` + `事件流包含 aborted 错误`）。given 因 solidify feature 无 Background（现有 step
  依赖 agent.feature Background 装配 model+tools+ws），改用策略一新写自足纯文本 step
  `已装配慢速流式 mock 模型`。

两条策略互补，构成 solidify 迁移的完整决策树。

## Capabilities

- `agent-runtime` — modify ar3/ar4/ar5/ar12，补 5 个可执行场景

## Impact

- 测试：新增 5 场景（100 → 105 passed），旧 tests/features/ 链路零回归。
- 不删旧 `tests/features/`（双轨并存，tb3 合约）。
- 不动 ar13/ar14（静态检查型，留待后续讨论是否归 verify）。
- 不改 config.yaml。
