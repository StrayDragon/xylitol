---
depends_on: []
---

## Why

外部 harness 常见的模式：LLM 每推进一小步就把整张 checklist 原样
重发一遍。xylitol 现行 todo 工具面（`src/infra/tools/todo.rs`）已有 `todo_update`
（单条改 status/content），但仍有结构性短板：

- **增删 / 重排仍只能走 `todo_rewrite` 整表替换**：追加一项、删一项、调顺序都要
  重发全表，token 成本 O(n) 随清单长度增长，长任务下浪费显著。
- **条目结构过薄**：`TodoItem` 只有 `{id, content, status}`（四态闭集），表达不了
  优先级、依赖（blocked_by）、父子拆解等规划语义，LLM 只能把信息塞进 content 文本。
- **rewrite 省略 id 时按序号生成**（`todo-{idx}`）：条目身份随重写漂移，跨
  rewrite 的 id 稳定性无保证。
- **每次变更回包全表**：mutation 返回完整 list，读回成本同样是 O(n)。

## What Changes

落地消融切点 **A3**（见 `design.md`）：三工具各一个动词，不扩闭集、不做旧形状兼容。

- `todo_list`：读全表。
- `todo_rewrite`：整表替换（建表 / 推翻 / 空数组清空；偶发的增删重排也走这里）。
- `todo_update`：只按 id patch（`items[]`，每条必有 id；可批量改 status/content/位置）。
- 服务端 `t_`+hex 短 id；mutate 对模型回短 ack；`TodoUpdated` / `agent_todo` 仍全量。
- 去掉 `cancelled`：LLM/SSOT/待办栏三态 `pending|in_progress|completed`。删一项 = rewrite 不带该行。旧快照 `cancelled` 行读取时丢掉该行。
- 不新增 `todo_add` / `todo_remove`。条目结构与 message 绑定 **不在本波**。

## Capabilities（预估，正式化时核对）

- `agent-todo`：r1118 三态 + 稳 id；r1125 update `items[]`、mutate 回 ack；
  r1129 前翼仅 completed；r1122 事件仍全量。
- `agent-prompt`：r1026 一条 todo guideline。
- `app-tui-transcript`：att36 mutate 块按 ack 画受影响行。

## Impact

- 行为合约变更（工具 schema + 回包 + id 生成 + 工具块 body）→ 完整 SDD。
- 涉及层：`protocol/session`（id 生成）、`protocol/ports`（gateway update 改 patches）、
  `infra/tools`（工具面）、`agent`（guidelines）、`app/tui`（工具块 ack 渲染）。
- 旧 JSONL `agent_todo` 快照、compact 重挂（r1119）、export（r1121）继续可读。
  旧 `{id,status}` 单对象 **不** 再接受。

## Intake（2026-09-20，从 c2795 转来）

呈现搬家不吃这些，本 change propose 时裁决（content ≤80 标量已由
`c2795-update-todo-projection-fixed-zone` 写入 r1842，勿再开一轮字数上限）：

- 条目可绑定 message id，用来评估「哪些 todo 覆盖哪段数据处理范围」。
- 可嵌套。
- 默认绑定最新 message。
- 转为 completed 时自动记录该绑定。

## Open Questions

- 绑定的是用户消息、助手消息，还是一轮 turn？（park：不在本波）
- 嵌套是父子树，还是按 message 范围分组？（park）
- 「自动记录」写进 `agent_todo` 快照字段，还是另写 Custom？（park）

## Further Notes

调研：`research/current-api.md`（现行 schema/prompt）、`research/peer-todo-llm-apis.md`、
`research/current-impl-and-seams.md`、`research/scope-triage.md`。

设计草案：`design.md` — 消融切点 A3 + slim ack/schema：三工具严格意图；
mutate 回 `{ids}` / `{changed:[{id,status}]}`；tools[] 不写长字段说明。
Intake（message 绑定 / 嵌套）park。
