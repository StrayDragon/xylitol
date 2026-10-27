# Design

## 场景选择

`after_turn` 的契约有两条可观察承诺：

1. 它在每个回合结束后运行；
2. 它的 `control` 只允许 `continue`，其他值必须被忽略。

单测只能证明第 2 条在 `run_after_turn` 内部成立，不能证明它在真实循环里不会提前终止 run。因此场景必须同时具备：

- 一个返回 `stop` 的 `after_turn` 中间件；
- 一个会让 run 进入第二轮的 `follow_up`。

如果 `stop` 被错误采纳，run 会在第一轮后结束，`TurnStart` 计数为 1；忽略则计数为 2。

## 断言

- `after_turn` 调用计数 `>= 1`（证明阶段真的被调用）；
- `TurnStart` 计数 `== 2`（证明 `stop` 被忽略，`follow_up` 未被吞掉）；
- 事件流不含 `LoopMiddleware` 错误。

## 不做什么

- 不新增 `after_turn` 专属事件或错误 kind。
- 不把「记入观测」具体化为新的日志格式契约；span 与 warn 日志是现有实现，`r2924` 已覆盖 span 机制。
