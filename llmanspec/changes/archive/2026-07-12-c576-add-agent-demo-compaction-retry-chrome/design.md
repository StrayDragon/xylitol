# design — c576 agent_demo compact/retry chrome

## 决议

1. 只预览 **c493 status 短词**：`Compacting`、`Retry n/m`，End 后回 `Working`/`Ready` + muted System。
2. 触发：和弦 + Ctrl+P plate + slash；时序用 demo `TimedAction`。
3. **不做** overlay / focus-steal demo；确认类 UX 用槽内组件。

## 验收

| ID | Then |
|---|---|
| D1 | Alt+K 后 status 出现 Compacting，稍后 Ready |
| D2 | Alt+Y 后 status 出现 Retry 1/3，稍后 Ready |
| D3 | 无 overlay-focus plate / Alt+R 演示入口 |
