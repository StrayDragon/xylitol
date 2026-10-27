---
depends_on: []
title: after_turn 中间件控制语义的端到端覆盖
needs_specs_change: true
---

# after_turn 中间件控制语义的端到端覆盖

## Why

`r2921` 规定 `after_turn` 的 `control` 须仅为 `continue`，其他取值须记入观测并忽略。当前只有 `middleware.rs` 的单测 `after_turn_control_is_ignored` 直接调用 `run_after_turn`，没有端到端场景证明：

- 该阶段在真实 ReAct 回合结束后确实被调用；
- 返回 `stop` 时不会提前终止 run（例如吞掉已入队的 `follow_up`）。

七阶段中 `after_turn` 是唯一没有 BDD 覆盖的阶段，语义缺口只能靠读代码发现。

## What Changes

- 在 `agent-runtime` 的 `r2921` 下新增场景「after_turn 非法控制被忽略」：注册 `after_turn` 中间件返回 `stop`，同时入队 `follow_up`；断言中间件被调用且 run 继续到下一轮。
- 新增对应 BDD step 与 binding；复用现有 fake provider 与 `agent_submit_root` harness。
- 不新增生产代码路径，不改 `control` 闭集，不改 `r2921` 的 MUST 文本。

## Capabilities

- `agent-runtime`：为既有 `r2921` 增加端到端验收场景。

## Impact

- 测试：`tests/bdd/steps_loop_middleware.rs` 新增一个中间件实现、given/then step；`tests/bdd/bindings_loop_middleware.rs` 新增一个 `#[scenario]`。
- spec：`llmanspec/specs/agent-runtime/agent-runtime.feature` 在 `r2921` 下新增一个场景。
- 生产代码：预期为零；若场景暴露真实缺口，按 `r2921` 修复并在本 change 内说明。
