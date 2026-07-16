# Tasks — c1120-add-app-tui-reload-slash

## 1. Slash + catalog

- [x] 1.1 `commands` / `slash_catalog`：识别 `/reload`；busy 拒绝
- [x] 1.2 `PendingSlash` / effects：编排 reload 步骤

## 2. 接线 foundation

- [x] 2.1 keybindings · skills · mcp · themes · context（已有 API）
- [x] 2.2 成功后刷新 `$skill` catalog；系统块报告
- [x] 2.3 harness：历史条数不变 + 至少 skills 或 mcp 可观测变化

## 3. 校验

- [x] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1120-… --no-interactive`
- [x] 3.2 `just qa`（或 lint + 相关测）
