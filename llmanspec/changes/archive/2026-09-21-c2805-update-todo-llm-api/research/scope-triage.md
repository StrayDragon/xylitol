# c2805 范围切分（取舍表）

> 2026-09-21。供 propose 拍板，不是 live 合约。
> 同行：`peer-todo-llm-apis.md`。接缝：`current-impl-and-seams.md`。

## 三张信息面（不要捆在一起改）

```text
LLM 工具 args / result     ← 模型付 token 的面
agent_todo 全量快照        ← 会话 JSONL / compact / export（不进前缀，r1124）
XyEvent::TodoUpdated       ← 端侧待办栏 / 远程 attach（必须全量，r1122 / r1718）
```

提案里「mutation 回包改 diff」只该动 **第一面**。后两面改 diff 会直接打穿待办栏 live 路径。

## 现在到底贵在哪

模型若每步 `todo_rewrite` 整表：args O(n) + result O(n)。
模型若走已有 `todo_update`：args O(1)，但 **result 仍全表**（今日实现）。
`agent_todo` 快照不进 LLM 前缀，不计入这一步的模型 token。

所以能立刻省钱的只有：

1. 让模型少用 rewrite（guidelines；description 不再把人往「整表 + 单 in_progress」推）。
2. mutate 对模型回 ack / 受影响条目，全表只留给 `todo_list`。
3. 增删不必再发未改项（同名 merge，或 `todo_update` 能 add/remove）。

message 绑定、嵌套、priority **都不省 token**。

## 三刀范围

| | A 工具面 | B A + 条目结构 | C 连 intake |
|---|---|---|---|
| **做什么** | 稳 id；guidelines；修过期 description；mutate 对 LLM 回 ack；rewrite 可 merge 或 update 能 add/remove。事件仍全量。三工具名不动。 | A + 可选 `activeForm` / `blocked_by` / `parent`（serde 忽略未知，旧快照可读） | B + 条目绑 message、嵌套、默认绑最新、完成时自动记录 |
| **解决** | rewrite 漂移；模型被错误 description 推向整表；回包双份 | 规划语义可进结构，不必塞进 80 字 content | 「这段对话覆盖哪些 todo」可评估 |
| **不解决** | 绑定 / 树 / 优先级 | 待办栏不会突然变树；无 message 归因 | — |
| **合约** | `agent-todo` r1118（id 稳）、r1125（工具语义 + 回包）、r1120/agent-tools 闭集（**保持 10 名**）；`agent-prompt` r1026 guidelines | 另改 r1118 字段闭集；r1129 可声明「栏只展示 content/status，忽略新字段」 | 重写 r1129 三区（树 vs 看板冲突）；可能新 Custom 或快照字段；跨端同源叙事 |
| **风险** | 模型训练先验是 `TodoWrite` 整表（OpenCode #1336、Goose #7589）。merge 同名比新开 `todo_add` 更可能被用。 | `priority`/`activeForm` 在同行几乎只是展示税；parent/blockedBy 是 Gemini tracker / Claude Task **另一产品** | **无同行先例**。待办栏刚按 status 三列落地（c2795）。嵌套会和 doing/pending/completed 分区打架 |
| **同行** | Claude TaskUpdate 补丁 + 服务端 id；Cursor `TodoWrite.merge`；Codex/Goose mutate ack | Claude `activeForm`/`addBlockedBy`；OpenCode `priority`；Gemini `parentId` | 无 |

## 为什么推荐 A，intake 另开

1. change id 是 `update-todo-llm-api`：工具契约，不是呈现/归因。
2. c2795 已声明呈现搬家不吃绑定/嵌套，转给本 change **裁决**——可以裁决为 park，不是必须本波做完。
3. 待办栏是 status 三列看板（r1129）。树要嘛压扁成假清单，要嘛重做栏。刚 archive 的固定区不该立刻掀。
4. 「绑最新 message / 完成时自动记录」先要定义绑的是 user / assistant / turn。这是产品决策，不是 schema 填空。
5. 新工具名会破 `default_tools` 10 闭集（r42）和首轮冻表（r1120）。A 可以不破。

## A 落地后仍要再问的子题（本表不拍）

- rewrite 加 `merge` 旗标 vs `todo_update` 扩 `add`/`remove`（仍三名）。
- LLM 回包：纯 ack vs `{ok, changed:[{id,status}]}` vs 继续全表（r1125 今日写「成功返回全表」）。
- 稳 id 算法：uuid 短截 vs 单调计数器（禁 `todo-{idx}`）。

## Intake 三条若 park，建议的后续 id 语义

- `c28xx-add-todo-item-structure`：parent / blocked_by / activeForm（仍可被栏忽略）。
- `c28xx-add-todo-message-binding`：绑哪类消息、完成时记绑定、是否另写 Custom。
