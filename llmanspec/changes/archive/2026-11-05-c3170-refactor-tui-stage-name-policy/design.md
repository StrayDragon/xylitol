# Design — 拆分阶段名自动生成策略

## 策略边界

`StageNamePolicy::should_request(StageNameContext)` 只回答“现在是否请求生成”；生成命令、失败处理、slash 强制刷新仍留在 effects / slash。

## 默认行为

`FirstTurn`：`!attempted && !has_name && !aborted`。

- 自动请求前 Host 置 `attempted = true`；失败不重试。
- 恢复会话已有阶段名时不请求；无阶段名时允许首次请求。

## 后续扩展

Context 已含 `completed_turns`；新增 `EveryTurns { every }` 或配置驱动策略时，只改策略枚举，不改事件处理。
