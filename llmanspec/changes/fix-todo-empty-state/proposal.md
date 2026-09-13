---
depends_on: []
rules_touched:
- att13
- att36
---

## Why

用户截图反馈（`Todo_list ... (Alt+E)`）：`todo_list` 是无参只读工具（schema
`properties: {}`），`todo_update` 参数是 `id`/`status`/`content`（无 `items` 键）——
两者在 `human_tool_args_preview_with_path` 的 todo 分支里永远取不到 `items` 数组，
于是折叠 header **永久**挂着 path 占位 `...`，被用户读成一个多余的「... 参数」。
`...` 是 read/write 路径槽的流式占位（att13），对没有路径概念的 todo_* 是错误隐喻。

同时，空列表结果（如新会话首轮 `todo_list` 返回 `{"items":[]}`）经 att36 人话化后
body 为空串，展开块除 header 外**什么都不画**——用户视角没有「列表为空」的显式提示，
与「展开能看到清单」的预期不符。

## What Changes

- **att13 修订（args 摘要）**：todo_* 分支无 `items` 可解析（`todo_list` 无参调用 /
  `todo_update` / rewrite 流式未齐）时摘要 MUST 为空串（header 只画工具名），
  MUST NOT 回落 path 占位 `...`；有 `items`（含空数组 → `0 items`）行为不变。
- **att36 修订（块 body 空态）**：todo_* 成功结果解析为**空列表**时，展开 body MUST
  渲染一行显式空态提示（固定英文词，与 `0 items` / `(Alt+E)` 同一 muted 展示词族），
  MUST NOT 留空 body；直播与重建仍共用同一 helper（幂等语义不变）。
- 不改：checklist 投影行空表清除语义（`refine-todo-display-planes` T2「空表也发布，
  语义为清除」维持）；LLM body 内工具结果 JSON；`todo_update` 摘要暂不加 id/status
  细节（保持改动聚焦，后续需要另立 change）。

## Capabilities

- `app-tui-transcript`：att13 todo 摘要空串条款 + att36 空列表 body 空态条款
  （含 `@executable` 场景扩展）。
