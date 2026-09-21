# 同行 Todo/checklist LLM API（c2805）

> **同行对照仍可用**；文内 **xylitol 行是 apply 前基线**（当时三工具 + `todo_list`）。落地后是 rewrite/update + 请求时尾插。勿按 xylitol 行回改。
>
> 问题：harness 如何把 Todo 暴露给模型（工具名、schema、整表 vs 增量、字段、回包、id、prompt）？什么真正省 token，什么仍被整表重写？
> 日期：2026-09-21。只引一手源。不改 live specs。xylitol 当时对照见同目录 `current-impl-and-seams.md`。

## 对照表

| product | tools | mutate | 条目字段 | id | 回包 | prompt | 源 |
|---|---|---|---|---|---|---|---|
| **Claude Code** ≥v2.1.142 | 默认 `TaskCreate/Update/List/Get`；`CLAUDE_CODE_ENABLE_TASKS=0` → `TodoWrite` | Task*=单条；TodoWrite=**full-replace** | Create：`subject, description, activeForm?, metadata?`。Update：`taskId` + `status?`（含 `deleted`）/`subject?`/`description?`/`activeForm?`/`addBlocks?`/`addBlockedBy?`/`owner?`/`metadata?`。TodoWrite 项：`content, status∈pending\|in_progress\|completed, activeForm`（无 id） | Create **服务端** `{task:{id,subject}}`；TodoWrite 无 id | Update `{success,taskId,updatedFields}`（非全表）；TodoWrite `{oldTodos,newTodos}` 双全表 | 3+ 步才建；默认可关 | https://code.claude.com/docs/en/agent-sdk/todo-tracking 与 typescript SDK types |
| **Codex CLI** | `update_plan`（#10124 拟改 `todo_write`；main 仍旧名） | **full-replace** `plan[]` | `{step, status∈pending\|in_progress\|completed}` + 可选 `explanation` | 无 | ack `"Plan updated"` | 至多一条 in_progress | openai/codex `codex-rs/core/src/tools/handlers/plan.rs` + `plan_spec.rs` + `protocol/src/plan_tool.rs` |
| **Cursor** | LLM `TodoWrite`；wire `TodoRead`/`UpdateTodos` | `todos[]` + **`merge`**：true=按 id merge | `id, content, status`；proto 另有 `dependencies[]`。agent.v1 有 timestamps/`CANCELLED`。本机 agent schema 允许 `cancelled`，无 activeForm | **客户端必填 id** | `final_todos`+`initial_todos`+`was_merge`（仍全表） | 勿清空 in_progress 却不开下一项 | `/usr/share/cursor/resources/app/extensions/cursor-resolver/dist/browser/main.js`（`aiserver.v1.TodoWriteParams`） |
| **OpenCode** | `todowrite` | **full-replace**（DELETE+INSERT） | `{content, status, priority}`；status 含 cancelled；priority high/medium/low。无 id | 无（content+position） | 回全表 JSON | 3+ 步；恰好一条 in_progress；实时改状态 | sst/opencode `packages/opencode/src/tool/todo.ts` + `todowrite.txt` + `session/todo.ts` |
| **Gemini CLI** | 旧 `write_todos`；实验 `tracker_{create,update,get,list}_task` / `add_dependency` / `visualize`（开 tracker 则关 write_todos） | write_todos=全表；tracker=CRUD+DAG | write_todos：`{description, status∈…\|blocked}` 无 id。tracker：`title, description, type∈epic\|task\|bug, parentId?, dependencies[]`；status open/in_progress/blocked/closed | tracker **服务端 6 hex** | write_todos 回编号全表；tracker mutate 短 ack，树走 UI `returnDisplay` | <2 步不用；仅一条 in_progress | google-gemini/gemini-cli `docs/tools/todos.md` `tracker.md`；`packages/core/src/tools/write-todos.ts` `trackerTools.ts` |
| **Crush** | `todos` | **full-replace** | `{content, status, active_form}`；无 id；diff 用 content 当 key | 无 | 文本 ack+计数；metadata 全表给 UI | 恰好一条 in_progress | charmbracelet/crush `internal/agent/tools/todos.go` + `todos.md` |
| **Goose** | `todo_write`（`todo__todo_write`）；`todo__read` 已删 | overwrite **一整段 markdown** | `content: string` 无结构 | 无 | ack `Updated (N chars)` | plan 变才改；**不必勾掉**；MOIM 注入前缀 | block/goose `crates/goose/src/agents/platform_extensions/todo.rs` |
| **Continue CLI** | `Checklist` | **full-replace** markdown | 一个 `checklist` 字符串 | 无 | 回显整段 | keep the list short | continuedev/continue `extensions/cli/src/tools/writeChecklist.ts` |
| **Cline** | **无 todo 工具**；普通工具 sidecar `task_progress` | 每次可带整份 markdown checklist | 勾选文本 | 无 | 写盘 + UI `say`，非 tool result | 静默写在工具参数里 | cline/cline `src/core/task/focus-chain/index.ts`；PR #6510 |
| **Amp** | 历史 `todo_write`/`todo_read`。现行 https://ampcode.com/docs/tools **不公布 schema** | ACP 帖每次带完整数组 | 样本 `{id, content, status∈todo\|in-progress\|completed, priority}` | 客户端 `"1"`… | 未公开；桥读 args 全表 | 无 | https://ampcode.com/chronicle ；ACP 帖 `T-53e0ba08-62d8-4472-b223-84e0a394b117` |
| **pi-coding-agent** | **非内置**。示例 `todo`：`list\|add\|toggle\|clear` | 增量 add/toggle/clear | `{id:number, text, done}` | 服务端 `nextId` | mutate 短 ack；list 才给行 | 仅列 actions | badlogic/pi-mono `packages/coding-agent/examples/extensions/todo.ts` |
| **Aider** | **无** | — | — | — | — | — | 搜 Aider-AI/aider 无 TodoWrite；#4085 为功能请求 |
| **xylitol** | `todo_list` / `todo_rewrite` / `todo_update` | rewrite=全表；update=按 id 补丁 | `{id, content, status}` 四态；content ≤80 | 可选；省略 → `todo-{idx}`（漂移） | 成功一律回全表 | description 仍写 at most one in_progress，与 r1126 矛盾 | `src/infra/tools/todo.rs`；`src/protocol/session/todo.rs`；agent-todo.feature r1125/r1126 |

本地 `~/.claude` 无 `todos/` 缓存；`@anthropic-ai/claude-code` npx 包无未压缩 `TodoWriteTool.ts`。官方 SDK 文档作 Claude 真源。anthropics/claude-code 公开仓不可用。

## 省 token vs 仍整表重写

**真正变便宜的（有源码/官方迁移为证）**

1. **同一名字上做 merge，而不是新开 4 个工具。** Cursor `merge:true` 仍叫 `TodoWrite`，模型不用换名字。Claude 默认从 `TodoWrite` 迁到 Task*（[todo-tracking](https://code.claude.com/docs/en/agent-sdk/todo-tracking)）：Create 无 id、Update 只补丁、List/Get 按需读。
2. **mutate 回 ack，不回全表。** Codex `"Plan updated"`；Goose `"Updated (N chars)"`；Claude `TaskUpdate` 只回 `updatedFields`；Gemini tracker LLM 短句，树给 UI `returnDisplay`。xylitol TUI 已有 `TodoUpdated` 事件（r1122），LLM 回全表是双份。
3. **少更新次数。** Goose 明确「plan changes 才 rewrite、不必勾掉」，并用 MOIM 把清单塞进前缀、删掉 `todo_read`（[PR #5027](https://github.com/block/goose/pull/5027)）。OpenCode 相反：「实时改状态、勿批量 completed」——在 full-replace 下等于**强制每步 O(n) 重发**。
4. **服务端稳 id。** Claude TaskCreate / Gemini tracker / pi 示例都是 server id。无 id 的产品（OpenCode、Crush、`TodoWrite`、Gemini write_todos）只能靠 content/顺序，重写即丢身份。

**模型仍整表重写的证据**

- Claude `TodoWrite` 用户报「更新一条就丢其余」：[anthropics/claude-code#2250](https://github.com/anthropics/claude-code/issues/2250)。schema 本身就是「交一整份 `todos`」。
- OpenCode 模型乱叫 `todo_write` / `todolist` / `TodoWrite`，对不上 `todowrite`：[#1336](https://github.com/sst/opencode/issues/1336)。训练先验是 **一个** `TodoWrite` 式工具，不是 `todo_add`/`todo_remove` 族。
- Goose #7589：模型按 Claude 形状传 `todos[]`，撞上 Goose 的 `content: string`。
- Amp ACP 帖：每次 `todo_write` 都带完整数组。

**额外字段实际用途**

| 字段 | 谁用 | 是否驱动执行 |
|---|---|---|
| `activeForm` / `active_form` | Claude TodoWrite **必填**；Crush 有则 UI 显示「正在…」 | 展示，不是调度 |
| `priority` | OpenCode **必填**，只入库 | 无按优先级调度 |
| `parent` / `parentId` | 仅 Gemini tracker、Claude Task 图 | tracker/Task 产品，不是 checklist |
| `blockedBy` / `addBlockedBy` / `dependencies` | Claude Task*、Cursor proto、Gemini tracker | Cursor LLM schema 有 `dependencies`，本机 agent 工具描述**没要求填**；未见「按依赖阻塞 in_progress」的 checklist 实现 |
| message 绑定 | **无任何同行** | xylitol intake 独有，不能从同行抄 |

## Implications for xylitol

**增量工具族会被用吗？** 不太会——若再拆 `todo_add`/`todo_remove`/`todo_reorder` 等新名字。训练分布是 `TodoWrite` 整表。xylitol **已经**有 `todo_update`（按 id 补丁），比 OpenCode/Crush/Codex/Continue 更接近 Claude TaskUpdate。缺的是：rewrite 仍承担增删重排，且 description 把模型往「整表 + 单 in_progress」推。

**mutate 回全表？** 在 xylitol 浪费为主：已有 `todo_list`（r1124：模型只从工具结果看 Todo），TUI 不解析 result 字符串（r1122）。回全表既不能替代 list，又每步付 O(n)。Codex/Goose/Claude TaskUpdate 证明 ack/diff 够用。Gemini tracker 把「给模型的短句」和「给 UI 的树」拆开，更贴 xylitol 的事件投影。

**推荐 API**

- **最小（建议默认）：** 保住三工具。① rewrite 省略 id 改为**持久短 id**（禁止 `todo-{idx}`）。② mutate 回 `{ok, changed:[{id,status}]}` 或纯 ack；全表只给 `todo_list`。③ prompt：status/content 走 `todo_update`；仅建表/增删/重排用 `todo_rewrite`；不要逐步重发未改项。④ 可选：给 `todo_rewrite` 加 Cursor 式 `merge`（同名增量，不换工具名）。⑤ 删掉 description 里过时的「at most one in_progress」（r1126）。
- **不要做最大集：** 不要一次加 TaskCreate/Get/List + priority + parent + blockedBy + activeForm + message 绑定。`priority`/`activeForm` 在同行里几乎只是展示税。父子/依赖是 Gemini tracker / Claude Task 的**另一产品**，checklist 上没被用起来。message 绑定无同行先例，应单独裁决，不混进本波工具 schema。
