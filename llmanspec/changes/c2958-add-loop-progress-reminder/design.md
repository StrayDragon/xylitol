# Design

## Core seam

`BeforeModelCtx` 增加：

```rust
pub todo: Option<TodoList>,
```

填充位置在 `run_react_loop` 每轮 `before_model` 之前。运行时本来就会为状态栏读取 leaf branch：

```rust
let bar = store.load_leaf_branch(&session_id).await
    .map(|branch| AgentStatusBar::from_session_entries(&branch))
    .unwrap_or_default();
```

把这一步提前到 `before_model` 之前，`bar.todo.clone()` 即中间件可见的 todo；随后 `project_outbound(&messages, &bar)` 仍用同一份状态栏，不增加读取次数。

## Reminder 触发

`ProgressReminderMiddleware` 阶段：`before_model` + `after_tool`（只维护计数）。

`after_tool`：

- `todo_rewrite` / `todo_update` 成功 → 陈旧计数归零；
- 其它工具 → 计数 +1；
- 记录 `is_error`。

`before_model`：

- todo 为空或无未完成项 → 不注入；
- 同一 turn 只提醒一次；
- 满足任一条件才注入：
  1. 连续 `stale_after_tool_calls` 次工具调用未更新 todo；
  2. 最后一条 assistant 宣称完成（`done` / `completed` / `finished` / `完成`）但仍有未完成项；
  3. 上一次工具失败且 todo 未更新。

提醒是短 user 消息（`<progress_reminder>` 包裹），不新增事件，不改 todo。

## 落点

扩展 `xylitol-official-plugin-loop-example`：

- `LoopExampleConfig` 增 `progress_reminder: bool`（缺省 false）；
- `progress_reminder` 为 true 时注册 `ProgressReminderMiddleware`（order 150，介于基础中间件 100 与 plan-state 200 之间）；
- 卸载时一并注销。

这样不新增 crate，也不新增顶层配置节。

## 不做什么

- 不自动改写 todo。
- 不在 `before_model` 调模型/工具。
- 不把提醒做成 hooks 的职责。
- 不改变状态栏 `<todo>` 的渲染。
