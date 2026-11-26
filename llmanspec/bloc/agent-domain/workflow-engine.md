# workflow-engine

> 任务图执行：七阶段容器、任务转移表、批准一次消费

子包：`agent-domain`　代码：`agent-domain/workflow-engine.ts`

## 状态图

```mermaid
stateDiagram-v2
    [*] --> contract
    contract --> executing: writePlan [planIsDag]
    executing --> awaitingApproval: taskBlocked
    executing --> replanning: replan [replansRemain]
    executing --> done: replan [replansExhausted]
    executing --> subWorkflow: enterSubWorkflow
    executing --> summarizing: summarize
    awaitingApproval --> executing: userApproved
    replanning --> executing: writePlan [planIsDag]
    subWorkflow --> executing: exitSubWorkflow
    summarizing --> done: summarize
    done --> [*]
```

## 状态清单

- `contract`
- `executing`
- `awaitingApproval`
- `replanning`
- `subWorkflow`
- `summarizing`
- `done`

## spec 锚点

- `llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2946`
- `llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2966`
- `llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2969`

## 实现落点

- `plugins/xylitol-official-plugin-workflow/。`
