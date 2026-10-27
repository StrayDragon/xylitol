# Design

## 配置

```rust
pub struct GeneratorCriticConfig {
    pub enabled: bool,
    pub rubric: Vec<String>,
    pub max_rounds: u32,
}
```

- `rubric` 为空时用内置四项（正确性、证据、边界、可验证性）；
- `max_rounds` 缺省 1，至少 1；critic 调用仍由 core 调度。

## 状态机

`GeneratorCriticMiddleware`：order 170，阶段 `before_model` / `after_model` / `decide_continue`。

```text
Idle
  after_model(assistant 无 tool_calls) 且 rounds < max_rounds
    → Critiquing，rounds += 1
  before_model(Critiquing) → 注入 critic 指令（rubric + 证据要求 + verdict 格式）
  after_model(Critiquing) → 解析 verdict
      APPROVE → Approved
      REVISE  → Revising，保存 critic 意见
      无 verdict → Revising（首次），再次无 verdict → Exhausted
  before_model(Revising) → 注入修订指令（critic 意见）
  after_model(Revising, assistant 无 tool_calls) → Critiquing（rounds += 1，若未达上限）
decide_continue:
  Critiquing / Revising → Continue（core 调度下一次模型调用）
  Approved / Exhausted  → Stop
  Idle / 其它           → Continue（不干预基线）
```

带 tool_calls 的 assistant 不触发评审；工具执行阶段的 settle 不受影响。

## verdict 格式

critic 指令要求以单独一行结尾：

```text
CRITIC_VERDICT: APPROVE
CRITIC_VERDICT: REVISE
```

解析大小写不敏感，接受 `APPROVED` / `REVISION`。没有结构化 verdict 时最多再要求一次，超过一次直接停止，避免无限自评。

## 注入位置

`before_model` 追加一条短 user 消息（`<critic>` / `<critic_revision>`），不改 assistant、不改工具结果。critic 不读文件、不调用工具，只按 rubric 评审并给 verdict。

## 有界

`max_rounds` 默认 1，远小于 core 的 32；达到上限时 `decide_continue` Stop，并记录原因。

## 与其它中间件

- 与 plan-state 并存时，Stop 由最先返回 Stop 的中间件决定；计划完成停止优先于评审续跑。
- 与 self-heal 并存时，自愈只处理确定性工具失败，critic 处理最终答案质量；两者互不调用。

## 不做什么

- 不引入独立 critic 模型/提供商。
- 不自动改代码或提交。
- 不用 critic 替代测试或人工判断。
- 不允许 critic 触发 `compact`。
