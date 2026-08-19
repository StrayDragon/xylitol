# Tasks

测试边界：`llman sdd validate` + 现有 `protocol-app` 审查行 + `just qa`。不新增可执行 `.feature`，不改 Rust 枚举。

## 1. 闭集合约进 live specs

- [ ] 1.1 `protocol-app`：单一产品真源、面本地排除、bang/工具分流、握手版本、闭集缺口（`feature: false`）
- [ ] 1.2 **不**删 `ip3` / `ip9`

## 2. 校验

- [ ] 2.1 `llman sdd validate c2301-update-stable-wire-protocol --strict --no-interactive`
- [ ] 2.2 结构闸绿（本票无运行时改动）
