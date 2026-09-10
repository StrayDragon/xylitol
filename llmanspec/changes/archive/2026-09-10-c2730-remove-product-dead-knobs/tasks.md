# Tasks: c2730-remove-product-dead-knobs

> pre-start。依赖 `c2710`。需要 Specs landing。

## 1. 合约

- [x] 1.1 start 后改 live spec：去掉 DatePlacement 消融与 `cycle_thinking_level`。
- [x] 1.2 [blocked-by: 1.1] validate strict。

## 2. 实现

- [x] 2.1 [blocked-by: 1.1] 删 `DatePlacement` 及 system/prompt_ops 分支；配置键一次性移除。
- [x] 2.2 [blocked-by: 1.1] 删 `cycle_thinking_level` Command/执行器/Driver/remote/harness；TUI 左右切 thinking 走 SetThinkingLevel。
- [x] 2.3 [blocked-by: 2.1, 2.2] 删仅覆盖消融/循环的测试；保留 `/model` picker 测。

## 3. 验证

- [x] 3.1 [blocked-by: 2.3] 相关测试 + `just qa`。
