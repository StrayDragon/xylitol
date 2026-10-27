---
depends_on:
  - c2957-add-loop-example-plan-state
  - c2960-add-loop-plan-checkpoint
title: 规划执行：把平面 checklist 升级为带依赖与局部重规划的计划
---

# 规划执行：把平面 checklist 升级为带依赖与局部重规划的计划

## Why

`c2957` 证明了 plan-as-state 可以驱动一轮 run：todo 列表 + `decide_continue`，不换 runtime。但它是**平面 checklist**，没有：

- `depends_on`（步骤依赖）；
- `inputs` / `outputs`（步骤间数据传递，防 plan starvation）；
- `verification` / `rollback`（可验证与可回退）；
- 局部 replan（失败只改下游，不重写全计划）；
- checkpoint / resume（`c2960`）。

ADPS A2 的两个命名失效模式正是：

- **Plan starvation**：Step 4 需要 Step 2 的产出，Step 3 把它丢了；裸字符串依赖是配方。
- **Replan thrash**：重规划重写整个计划，重复已完成工作，成本膨胀。

现状证据：

```text
plugins/xylitol-official-plugin-loop-example/src/middleware.rs
  PlanProgress { total, pending, in_progress, completed }   ← 只有计数，没有 DAG

crates/xylitol-plugin-sdk/src/session/todo.rs
  TodoItem { id, content, status }   ← 无 depends_on / inputs / outputs / verification

crates/xylitol-plugin-sdk/src/session/entries.rs
  CustomEntry { base, custom_type, data }   ← 可承载 PlanStep DAG
```

## What Changes（候选，promote 时逐条重估）

### 1. Plan schema

新增 `plan` 工件（`CustomEntry`，`custom_type = "plan"` 或扩展 todo）：

```json
{
  "goal": "...",
  "steps": [
    {
      "id": "s1",
      "description": "...",
      "depends_on": [],
      "inputs": [],
      "outputs": ["artifact:a"],
      "risk": "low",
      "verification": "cargo test -p x",
      "rollback": "...",
      "status": "pending"
    }
  ],
  "approved": false,
  "resume_cursor": null
}
```

### 2. 三个角色

| 角色 | 落点 | 说明 |
|---|---|---|
| Planner | 模型经 `plan` 工具产出 | 高一层模型可选；首版同模型 |
| Executor | 中间件 + core 调度 | `before_model` 暴露下一个可执行 step；`after_tool` 更新 status/output |
| Orchestrator | 中间件状态机 | 维护依赖、checkpoint、局部 replan、审批门 |

### 3. 执行规则

```text
runnable = steps where status == pending and all depends_on == done
before_model: 注入当前 step + 最小工具集（c2959）
after_tool:   读结果 → 更新 status/output；失败 → 标记 failed
decide_continue:
  有 runnable 且未到 cap → Continue
  有 blocked（requires_human）→ 停止并请求审批
  全部 done / 无法推进 → Stop
```

### 4. 局部重规划

失败时只重写失败 step 的下游子 DAG；已完成 step 不得重跑；重规划次数有上限。

### 5. 审批门

`requires_human` step 在执行前阻塞；批准后只消费一次，参数与前置条件须复验（ADPS G1 的要点）。

### 6. 与 core 的边界

- core 仍拥有 turn 边界、取消、权限终裁、落盘格式；
- 模型调用仍由 core 调度；插件不得自行调模型；
- 首版插件化；只有在「plan DAG 执行器」被证明无法用中间件表达时，才评估 core 级模式（见 `c2956`）。

## Capabilities

- `agent-runtime`：`before_model` / `after_tool` / `decide_continue` 的计划执行合约；局部重规划与有界语义。
- `agent-session-store`：`plan` custom entry 的持久化与恢复。
- `tool-system`：`plan` 工具（schema + 执行模式）。
- `runtime-config`：计划模式开关、重规划上限、审批策略。

## Impact

- crate：`plugins/xylitol-official-plugin-plan-execute`（新官方插件）或扩展现有 plan 插件；`xylitol-plugin-sdk` 增 `PlanStep` 类型与 helper。
- 测试：依赖顺序、plan starvation 防护、局部 replan 不重跑已完成、审批门、checkpoint 恢复、上限停止。
- 文档：`src/AGENTS.md` 补「plan 是持久工件，runtime 只是 plumbing」。

## Out of Scope

- 多 Agent / 子 Agent 舰队。
- 云端 PR、向量索引、Repo map。
- 自动 VCS 回滚（先做提示级）。
- 把 plan 执行器做成 core 内联逻辑。

## Further Notes

**ROI**：中高，取决于产品是否要 plan mode。若只是「别迷失」，`c2957` + `c2960` 已够；若要做长程、可恢复、可审批的任务，A2 才值得。

**关键证据**：ADPS A2 的参考实现明确写 "The DAG is intentionally trivial… **The runtime is plumbing; the contract is the pattern.**" 也就是说，先做 plan 工件与执行契约，不先做新引擎。

**开放问题**：

1. `plan` 工具与 `todo_rewrite` 的关系：是替换 todo，还是并存（todo 给人看，plan 给执行）？建议并存，plan 引用 todo id。
2. Planner 是否用独立模型？首版同模型，后续再评估成本。
3. 审批门复用现有 permission/approval 还是新增 plan 审批？建议复用 `ApprovalRequired`，只加 `requires_human` 标记。
4. 局部 replan 的「下游子 DAG」如何计算？需要显式的反向依赖图，不能靠模型自由发挥。
