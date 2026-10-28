# Design

## 为什么是 plan-as-state，而不是新 runtime

ADPS A2 的核心结论是：plan 是**持久工件**，runtime 只是 plumbing。xylitol 已有：

- `todo_rewrite` / `todo_update` 工具，结果落会话存储（`SessionAgentTodoGateway`）；
- `latest_agent_todo` + 状态栏把 `<todo>` 注入模型上下文；
- 七阶段中间件可在 `before_turn` / `after_tool` / `decide_continue` / `after_turn` 读写。

因此不需要新引擎：中间件读 todo 结果 → 维护进度 → 用 `decide_continue` 决定是否继续。

## 阶段与职责

| 阶段 | 行为 |
|---|---|
| `before_turn` | 追加系统碎片：要求模型用 `todo_rewrite` 写计划、保持一个 `in_progress`、用 `todo_update` 勾完成；未完成不得宣称完成 |
| `after_tool` | 若工具为 `todo_rewrite` / `todo_update` 且非错误，解析首个文本分片 JSON 的 `items[].status`，更新进度 |
| `decide_continue` | `has_new_user_messages` 或「计划未完成且 `settle_count < max_settles`」→ `continue`；否则 `stop` |
| `after_turn` | 记录 checkpoint 日志（`turn_index`、`continued_tools`、`completed/total`），不发新事件 |

## 顺序

`PlanStateMiddleware.order = 200`，排在示例中间件（`100`）之后。示例中间件的 `decide_continue` 在 `max_settles` 达到时返回 `stop`，此时计划中间件不再运行；否则计划中间件拿到最终裁决权。

## 边界与已知限制

- `DecideContinueCtx` 不暴露 follow-up 队列状态；按 `r2921`，`stop` 不 drain follow-up。示例明确接受这一限制，并在 `c2956` 记录为产品化前需解决的问题。
- `max_settles` 缺省给计划中间件一个 8 的上限，避免在模型不更新 plan 时撞 core 的 32 次硬上限。
- 不改 `control` 闭集；`handoff` 仍由 core 稳定报错。
- 不新增事件；checkpoint 只进日志。

## 验证

- 单测：契约注入、todo 结果解析、未完成继续、完成停止、到上限停止、有新用户消息继续、错误/未知工具不更新。
- 插件单测：`plan_state` 开/关的注册表 id 集合；卸载后为空。
- BDD：`r2925` 新增「计划模式启用后注册计划中间件」。
- 配置：`configs/config.schema.json` 与代码同源；`example.yaml` 注释示例更新。
