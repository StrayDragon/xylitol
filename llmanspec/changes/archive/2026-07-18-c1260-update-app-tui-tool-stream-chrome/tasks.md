# Tasks: c1260-update-app-tui-tool-stream-chrome

## Promote

- [x] live specs：atb10 / att13（feature:false 场景 + 单测证据）
- [x] `llman sdd change attach c1260-update-app-tui-tool-stream-chrome`
- [x] proposal `status: full`

## 实施

- [x] `human_tool_args_preview` + `upsert_tool_entry` / `sync_tool_intent_from_message`
- [x] MessageUpdate → 挂载/更新 Tool；Start upsert 不重复
- [x] session_tree 重建共用摘要
- [x] bridge 单测（intent / stream args / bash summary）
- [x] validate + attach
