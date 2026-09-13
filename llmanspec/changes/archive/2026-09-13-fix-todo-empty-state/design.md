# Design — fix-todo-empty-state

## D1 空 `items` 摘要 → 空串（不是保留 `...`）

`...`（`PATH_PLACEHOLDER`）源自 pi `renderToolPath` 的流式路径占位，语义是「路径槽
尚无内容」。todo_* 没有路径概念；且 `todo_list` schema `properties: {}`、
`todo_update` 参数为 `id`/`status`/`content`（无 `items` 键）——两者**永远**取不到
`items`，`...` 不是流式瞬态而是永久假参数（用户截图证据）。因此 todo 分支无
`items` 时返回空串，header 走既有「只画工具名」分支（`paint_tool_header_line`
`args_preview.is_empty()` 路径）；rewrite 流式未齐时短暂空串、`items` 到达后变
计数摘要，与 write 族「流式无路径 → 占位」的瞬态隐喻无关但不产生误导。
有 `items`（含 `[]` → `0 items`）行为不变。

## D2 空列表提示放块 body（结果面），固定词 `(empty list)`

- **落点**：header 摘要在 call 时由 args 计算、不含结果信息；「列表为空」是**结果**
  事实，落 att36 块 body（`humanize_tool_result_for_tui`）。直播 End 与 travel
  重建共用该 helper，天然幂等（att50）。
- **词形**：`(empty list)`——英文小写括注，与既有 muted 展示词族
  （`(Alt+E)` / `(timeout {N}s)` / `0 items`）同风格；括注形式与 footer hint 区分
  （后者是 `... (N earlier lines, ctrl+o)` 交互提示）。单行短文本经
  `render_expandable_output` 不触发 footer（`skipped_count == 0` → 无 hint），零噪音。
- **投影行不跟随**：checklist 投影行维持 `refine-todo-display-planes` T2 的
  「空表发布 = 清除行」语义（那是 SSOT 状态面：没有待办就不该有常驻行）；空态提示
  只在工具块 body 事实面（那一次调用确实返回了空表）。

## D3 不给 `todo_update` 摘要补 id/status 细节

可以做（如 `#a → completed`）但超出本次用户反馈范围（聚焦原则）；`todo_update`
与 `todo_list` 一样回落空串 header。需要时另立 change。

## 测试 seam（复用，不发明）

| seam | 用途 |
|---|---|
| `human_tool_args_preview` 单测（`preview.rs` tests） | T1 摘要空串 |
| `humanize_tool_result_for_tui` 单测（同上） | T2 空态提示 |
| transcript BDD：`#[scenario]` 绑定 live spec 场景（`bindings_app_tui_transcript.rs` + `steps_app_tui_transcript.rs`），`cargo test --lib --all-features tests::bdd::` | 两个新 `@executable` 场景；直播 vs 重建同构沿用既有 fixture 机制 |
