---
depends_on: []
branch: sdd/c2805-update-todo-llm-api
base_branch: main
base_sha: 7cabdd8e9a1b64d6035f2564de70278f9285e2f7
---

## Why

外部 harness 常见的模式：LLM 每推进一小步就把整张 checklist 原样
重发一遍。xylitol 现行 todo 工具面（`src/infra/tools/todo.rs`）已有 `todo_update`
（单条改 status/content），但仍有结构性短板：

- **读表占一个常驻工具名**（`todo_list`）：每 turn 付 schema 税；丢上下文时还要再
  往返一次。n 很小，真正贵的是 tools[] 与 round-trip。
- **增删 / 重排仍只能走 `todo_rewrite` 整表替换**：追加一项、删一项、调顺序都要
  重发全表。
- **rewrite 省略 id 时按序号生成**（`todo-{idx}`）：条目身份随重写漂移。
- **状态栏族 park 提案**（delayed `c1895`–`c1898`）曾把「模型看见 Todo」绑在
  Agent 列 / refresh 工具上，等于再开一条注入通道。

## What Changes

落地 **两工具 + 请求时尾插**（见 `design.md`）。n 很小，不为运行时复杂度优化。
delayed-changes/next-todo 仍 park；本波只吸收「generate 前看见当前清单」，
**不**做全栏子系统、**不** persist 栏消息、**不**加 refresh/publish 工具。

- 去掉 `todo_list`。闭集 9 名。
- 出站 generate：`project_outbound`；非空清单为 `<agent_status_bar>` 下 `<todo>` 子树；
  不落盘、不进 system / session_env。
- `todo_rewrite`：整表替换；成功回全表 `{items}`。
- `todo_update`：只按 id patch（`items[]` 批量）；成功回全表 `{items}`。
- 服务端 `t_`+hex 短 id。
- 去掉 `cancelled`。不新增 `todo_add` / `todo_remove`。绑定/嵌套 **不在本波**。

## Capabilities（预估，正式化时核对）

- `agent-todo`：r1124 AgentStatusBar `<todo>` 子树；r1125 两工具；r1118 三态 + 稳 id；
  r1128 栏只读 SSOT、不 persist；r1129 前翼仅 completed。
- `agent-tools`：r42 / r1144 / r1150 闭集 9、无 `todo_list`。
- `agent-session`：背景步 9 个内置工具。
- `agent-prompt`：r1015 / r1018 仍不把 Todo 写入 system。
- `app-tui-transcript`：r1343 / r1367 不再依赖 `todo_list` 夹具。

## Impact

- 行为合约变更（工具闭集、前缀可见性、schema、id、工具块夹具）→ 完整 SDD。
- 涉及层：`protocol/session`、`agent`（投影尾插）、`infra/tools`、`app/tui`。
- 旧 JSONL `agent_todo` 快照、compact 重挂（r1119）、export（r1121）继续可读。
  旧 `{id,status}` 单对象 **不** 再接受。历史 `todo_list` 调用不复活该工具。

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

调研：`research/current-api.md`、`research/peer-todo-llm-apis.md`、
`research/current-impl-and-seams.md`、`research/scope-triage.md`、
`research/delayed-status-bar-absorb.md`。

设计：两工具；update `items[]`；mutate 回全表；出站 `<agent_status_bar><todo>`；
去掉 `todo_list` 与 cancelled。Intake park。delayed persist/注册表仍 park。
