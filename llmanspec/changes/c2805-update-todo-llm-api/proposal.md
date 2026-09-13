---
depends_on: []
---

## Why

外部 harness 常见的模式（见会话截图）：LLM 每推进一小步就把整张 checklist 原样
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

（方向性草案，具体取舍 propose/design 裁决）

- 补齐**增量变更原语**：单条/批量 add、remove、reorder、patch；`todo_rewrite`
  降级为「首次建表 / 批量重建」专用，不再承担日常小步更新。
- **增强条目结构**（候选）：优先级、依赖关系（blocked_by）、父子嵌套拆解、
  content 与进行时描述拆分——哪些进产品按「LLM 规划表达收益 vs 结构复杂度」
  裁决；wire 兼容旧 `agent_todo` 快照（向后可读）。
- **收敛返回值**：mutation 回包从全表改为 diff / 受影响条目（或全表 + diff），
  压读回成本。
- **id 稳定性**：服务端生成持久短 id，跨 rewrite / 重排不漂移。

## Capabilities（预估，正式化时核对）

- `agent-todo`：atd1 领域模型（条目结构扩展）、atd4 工具语义（增量原语改写）、
  atd5 单在途约束（结构增强后是否保留/泛化）、atd13 类型化投影事件（事件形状随
  diff 收敛）、atd2 latest-wins 快照（形状兼容）需修订。
- `agent-prompt`：工具使用指引（何时 rewrite、何时增量）需同步。
- `跨端同源` 约束板：gpui 桌面端同款 todo 语义。

## Impact

- 行为合约变更（工具 schema + `agent_todo` 数据契约 + 事件形状）→ 走完整 SDD
  pipeline（propose 起）。
- 涉及层：`protocol/session`（领域词汇与校验）、`protocol/ports`（AgentTodoGateway
  端口形状）、`infra/tools`（工具面）、`agent`（系统提示指引）、`app/tui`（投影与
  渲染）。
- 兼容性红线：会话 JSONL 旧 `agent_todo` 快照、compact 快照保留（atd10）、export
  展示（atd12）必须继续可读。
