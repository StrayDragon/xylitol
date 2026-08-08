# Tasks：c1990-update-exact-thinking-level-strings

## 1. 合约

- [x] 1.1 更新 `runtime-model-registry` / `package-ai-bridge`（及必要 `runtime-config`）requirement：档名匹配 MUST 精确；MUST NOT casefold/trim 归一
- [x] 1.2 相关 `.feature` 或文档场景：大小写变体拒绝 / bridge 不折叠

## 2. 实现

- [ ] 2.1 bridge：`canonical_known_level` → 精确匹配；单测覆盖 `HIGH`/` high `
- [ ] 2.2 protocol：`thinking_levels_are_adjustable` 对 `off` 精确比较
- [ ] 2.3 manager / 文档注释与 TUI 边框：确认展示模糊不影响档名回写

## 3. 验证

- [ ] 3.1 bridge + lib 相关单测绿
- [ ] 3.2 `just qa`（或约定门禁）绿
