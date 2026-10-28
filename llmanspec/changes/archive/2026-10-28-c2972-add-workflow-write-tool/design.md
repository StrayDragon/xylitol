# Design

## Tool schema

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["goal", "steps"],
  "properties": {
    "goal": { "type": "string" },
    "steps": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["id", "description"],
        "properties": {
          "id": { "type": "string" },
          "description": { "type": "string" },
          "depends_on": { "type": "array", "items": { "type": "string" } },
          "inputs": { "type": "array", "items": { "type": "string" } },
          "outputs": { "type": "array", "items": { "type": "string" } },
          "risk": { "type": "string", "enum": ["low", "medium", "high"] },
          "verification": { "type": "string" },
          "rollback": { "type": "string" },
          "requires_human": { "type": "boolean" },
          "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "failed", "blocked"] },
          "output_ref": { "type": "string" },
          "sub_workflow": { "type": "object" }
        }
      }
    },
    "resume_cursor": { "type": "string" }
  }
}
```

## Write path

1. Deserialize args into `Workflow`.
2. Load latest persisted workflow for the session (if any).
3. `validate_workflow` then `validate_workflow_replan(previous, next)`.
4. Append `CustomEntry(workflow)`.
5. Publish `XyEvent::WorkflowUpdated { workflow }`.
6. Return `{goal, steps, completed, runnable}`.

Errors are `InvalidArgs` with the validation message; no entry is written on failure.

## Middleware simplification

- `before_model`:
  - No workflow → inject `WORKFLOW_CONTRACT`: "call `workflow_write` with the full workflow; do not emit a `<plan>` block".
  - Pending error → inject `<workflow_error>` once.
  - Runnable/failed/blocked steps → existing instructions unchanged.
- `after_tool`:
  - `workflow_write` success → adopt the returned workflow and clear errors.
  - `workflow_write` error → set `pending_error` from the tool result.
- Remove `after_model` parsing entirely.

## Provider projection

- The raw `<plan>` block is no longer part of the workflow contract.
- Assistant text remains normal prose; tool calls carry the workflow.
- No new projection rule is needed beyond removing the old contract string.

## Specs

- `agent-workflow`: write/read tools, tool-boundary validation, error feedback; replace the `<plan>` block clause.
- `agent-session-store`: workflow written by the tool with the same CustomEntry format.
- `agent-llm-projection`: no raw control block in provider POST.

## Tests

- Unit: tool writes valid workflow; invalid DAG/replan rejected; event published.
- Unit: middleware adopts tool success; injects error once.
- Unit: no `<plan>` contract text remains.
- BDD: model calls `workflow_write` and the step instruction follows.
- `just qa` and `llman-sdd validate --all --strict`.

## Task breakdown

- 1. **specs**（spec）：`agent-workflow` 改为工具写入并删除 `<plan>` 块条款；`agent-session-store` 记录工具写入同一 CustomEntry；`agent-llm-projection` 记录 POST 不含原始控制块。测：`llman-sdd validate --all --strict`。
- 2. **工具**（plugin）：新增 `workflow_write` / `workflow_read`；校验 DAG 与 replan；成功 append CustomEntry 并发布 `WorkflowUpdated`；失败不写。测：合法/非法/replan/事件。
- 3. **中间件**（plugin）：删除 `extract_plan_json` 与 `after_model` 解析；`after_tool` 采用工具结果或记录错误；`before_model` 注入 `workflow_write` 契约与 `<workflow_error>`。测：采用/错误注入一次。
- 4. **契约文本**（plugin）：移除 `<plan>` 契约与相关提示；确认 provider POST 不含原始块。测：POST 投影单测。
- 5. **BDD**（tests）：模型调用 `workflow_write` 后进入步骤指令。测：`cargo test --all-features --test bdd`。
- 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
