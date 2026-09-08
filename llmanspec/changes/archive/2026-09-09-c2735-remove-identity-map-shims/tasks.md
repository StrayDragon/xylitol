# Tasks: c2735-remove-identity-map-shims

> Implemented。依赖 `c2700`。

## 1. 选型与删

- [x] 1.1 确认 bridge 是否已有 token 类型可 alias（方案 A vs B）。
- [x] 1.2 [blocked-by: 1.1] 删 identity `From`；统一 import。
- [x] 1.3 [blocked-by: 1.2] `rg 'From<AiBridge'` 确认无新 identity map。

## 2. 验证

- [x] 2.1 [blocked-by: 1.3] provider / context token 测。
- [x] 2.2 [blocked-by: 2.1] `llman sdd validate c2735-remove-identity-map-shims --strict --no-check`。
