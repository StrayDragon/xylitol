# Tasks: c2715-refactor-eventbus-xyeventsink

> pre-start。依赖 `c2700`。可与 `c2705` 并行。

## 1. 清单

- [ ] 1.1 `rg 'EventBus|settings:changed' src tests` 分类：sink 构造 vs 字符串频道。
- [ ] 1.2 [blocked-by: 1.1] 确认无产品路径依赖频道订阅。

## 2. 收口

- [ ] 2.1 [blocked-by: 1.2] EventBus `pub(crate)`；删除字符串 subscribe API（或测试-only）。
- [ ] 2.2 [blocked-by: 2.1] 改 Settings / lifecycle 注释；禁止假热重载接线。
- [ ] 2.3 [blocked-by: 2.2] 测试改 `dyn XyEventSink`。

## 3. 验证

- [ ] 3.1 [blocked-by: 2.3] 相关测试 + `just qa` 触及面。
- [ ] 3.2 [blocked-by: 3.1] `llman sdd validate c2715-refactor-eventbus-xyeventsink --strict --no-check`。
