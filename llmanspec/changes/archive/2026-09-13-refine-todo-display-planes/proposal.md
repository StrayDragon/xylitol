---
depends_on: []
rules_touched:
- att13
branch: sdd/refine-todo-display-planes
base_sha: 4baeb6252aebd89f659387f4609f7f2a49ef574f
---

## Why

`todo_*` 工具调用块在 TUI 中以原始 JSON 呈现（截图证据：`Todo_rewrite` 展开后是
`{"items":[…]}` 整表快照）。根因：`human_tool_args_preview_with_path` 与
`humanize_tool_result_for_tui` 的按工具呈现表都没有 todo 分支，args 摘要落空、
result 兜底裸奔；`output_looks_like_machine_json` 的机器键名单也不含 `items` 形态。

更深层：todo 信息被多个信息面各自消费，其中 **live 投影靠端侧解析工具结果字符串**
（`sync_todo_checklist_from_tool_result`），与 resume 重建读 `agent_todo` SSOT
类型化快照不同构；这违背「host 持有语义状态、端只渲染」的 CS 角色定位，未来 gpui
GUI 还要再猜一遍 JSON 形状。需要一次信息平面（LLM body / 会话 SSOT / 恢复投影 /
client 展示 / 日志）的显式细化，todo 是第一个受益者。

## What Changes

- **todo_* 工具块人话化（TUI）**：header 摘要补 todo 分支（条目计数 + in-progress
  计数）；展开 body 从原始 JSON 换成勾选清单渲染（状态字形 + content，与 checklist
  投影共用同一字形表）；is_error 结果保持错误文本原样。直播与重建共用同一 helper。
- **host 推类型化领域事件**：新增 `XyEvent` 变体（`TodoUpdated`，载荷为完整
  `TodoList` 快照），在 SSOT 变更点发布；TUI live checklist 投影改从该事件同步，
  删除端侧解析工具结果字符串的路径；线协议 `protocol::Event` 能表达该变体（远程
  attach 不丢 checklist 刷新；未升级端可降级忽略）。resume 重建仍读 SSOT 快照。
- **信息平面总览文档**：`docs/architecture/` 新增 LLM API body（消息列表）/ 会话
  SSOT / 恢复投影 / client 展示 / 日志五面的读写权与流向总览，todo 作为 worked
  example；并按 `designing/` SOP 更新 `tui/modules/tool/` 设计稿（todo 块形态）。
- 不改：LLM body 内工具结果仍为 JSON（模型接口，不动）；att24 保持 todo_* 走
  Used（保留时间线）；`agent_todo` SSOT 持久与 atd3 前缀隔离不变。

## Capabilities

- `agent-todo`：新增类型化 live 投影事件合约（atd13）。
- `app-tui-transcript`：att13 args 摘要表补 todo 分支（rules_touched）；新增 att36
  （todo_* 块 body 清单渲染）。
- `protocol-app`：新增 pa-todo1（TodoUpdated 线协议可表达 + 降级语义）。
