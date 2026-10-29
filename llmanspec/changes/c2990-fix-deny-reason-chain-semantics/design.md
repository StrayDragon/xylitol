# Design

## Context

`run_before_tool` 通过 `run_stage` 依次调用中间件，`ctx` 在链上传递。`deny_reason` 目前是普通字段：后置中间件收到前序写入的值，也可以把它改成 `None` 或其他原因。runtime 只在链结束后读取终态。

## Decision

采用 **A. 首次设置即锁定（first-wins）**。

- `run_before_tool` 在进入链前记录 `denied = ctx.deny_reason`。
- 每步之后：
  - 若 `denied` 为空且 `ctx.deny_reason` 非空 → 记录该原因；
  - 若 `denied` 已非空 → 把 `ctx.deny_reason` 恢复为 `denied`，后置中间件仍能读取，但改写/清空无效。
- 链结束时把 `ctx.deny_reason` 固定为首个非空原因。

实现上给 `run_stage` 增加一个 `settle: impl FnMut(&mut C)` 回调，仅 `run_before_tool` 传入锁定逻辑；其他阶段传空操作，行为不变。

## Alternatives

- **B. 最后写入者赢**：保持现状并写进 spec；后置中间件可覆盖，但安全边界更弱。
- **C. 显式 override 标记**：新增 SDK 字段/枚举，改动面大；当前无明确需求。
- **D. 不允许后置中间件读取**：会破坏“后置闸可叠加更严规则”的常见用法。

## Risks

- 锁定后，后置中间件无法把原因改得更精确；这是有意的安全取舍。
- 只影响 `before_tool` 阶段；其他阶段的 ctx 字段不受影响。
