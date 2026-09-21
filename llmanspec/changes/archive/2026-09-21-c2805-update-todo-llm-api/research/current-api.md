# 现行 Todo LLM API 对照（c2805 基线）

> **apply 前基线**。落地后无 `todo_list`、出站 `<agent_status_bar><todo>`、update `prompt_guidelines`。勿按本表回改。
>
> 2026-09-21。当时代码真值，非目标合约。

## 模型能看见的三件事

| 通道 | 现行内容 | 进模型？ |
|---|---|---|
| 系统提示 Available tools | `- {name}: {prompt_snippet \|\| description}` | 是 |
| 系统提示 Guidelines | `prompt_guidelines()` 非空才进 | **否**（todo 三工具全空） |
| 工具 schema / description | provider `tools[]` | 是 |
| 工具结果字符串 | `{"items":[...]}` 全表 | 是 |
| `agent_todo` 快照 | 会话 Custom | **否**（r1124） |
| `TodoUpdated` 事件 | 全量 `TodoList` | **否**（端 / wire） |

`default_system.j2`：`Available tools:` 下逐行 `- {{ t.name }}: {{ t.snippet }}`。snippet 缺省 = `description()`。

## 领域

`TodoItem { id: String, content: String, status: TodoStatus }`
`TodoStatus` = `pending | in_progress | completed | cancelled`
`TodoList { items: Vec<TodoItem> }` → 快照 `{ "items": [...] }`，`custom_type = agent_todo`

校验：id/content trim 非空；content ≤ 80 标量；**允许多条** in_progress。
rewrite 省略 id → `todo-{idx}`（1-based，**会漂**）。
无 `deny_unknown_fields`：旧快照多字段可忽略。

## 工具（闭集 10 名里的 3 个，Barrier/Sequential）

### `todo_list`

- description: `Return the current session Todo checklist (full list). Read-only; does not mutate.`
- schema: `{ type: object, properties: {}, additionalProperties: false }`
- 成功: `{"items":[...]}` 全表；不写快照；不发事件

### `todo_rewrite`

- description: `Replace the entire session Todo checklist. Returns the full written list. At most one item may be in_progress.` ← **与 r1126 矛盾**
- schema:
  - required: `items`
  - item: `{ id?: string, content: string, status?: enum }`（status 默认 pending）
  - `id` 说明：`Stable id; omitted → server generates`
- 成功: 全表 JSON + persist 全量快照 + `TodoUpdated`
- 空 `items` = 清表

### `todo_update`

- description: `Update one Todo item by id (status and/or content). Returns the full list. Rejects unknown id or dual in_progress.` ← 后半句过时
- schema: required `id`；可选 `status` / `content`（至少改一项）
- **不能** add / remove / reorder
- 成功: 全表 JSON + persist + `TodoUpdated`

## 端口

`AgentTodoGateway`: `list` / `rewrite(Vec<TodoItemDraft>)` / `update(id, status?, content?)` → 均 `TodoList`。

## 端侧（模型不可见）

- 待办栏：只画 `status` + `content`
- 工具块 header：有 `args.items` 才 `N items · M in progress`；`todo_update` / `todo_list` 无 items → 空摘要
- 工具块 body：解析结果 `{items}` 成勾选行
